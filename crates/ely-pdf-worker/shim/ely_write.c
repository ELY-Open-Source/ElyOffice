/* New text on a page: a pen, lines, content (PDF-03). */

#include <math.h>
#include <string.h>

#include "ely_shim.h"

/* What to write, where, in what. */
typedef struct
{
	const char *text;
	const char *font;
	float size;
	int rgb;
	/* Page space: left, top, width. */
	float box[3];
} text_args;

/* `text` as code points; the count is returned. */
static int runes(fz_context *ctx, const char *text, int **out)
{
	int n = 0, c;
	const char *s;
	for (s = text; *s; n++)
		s += fz_chartorune(&c, s);
	*out = fz_malloc_array(ctx, n + 1, int);
	for (n = 0, s = text; *s; n++)
		s += fz_chartorune(&(*out)[n], s);
	return n;
}

/* Lines no wider than `width`: [from, to) each, and widths. */
static int lay_out(fz_context *ctx, ely_pen *p, const int *u, int n, float size, float width, int *from, int *to,
	float *wide)
{
	int i = 0, lines = 0;
	while (i <= n)
	{
		int j, space = -1;
		float run = 0, before = 0;
		for (j = i; j < n && u[j] != '\n'; j++)
		{
			float w = ely_pen_advance(ctx, p, u[j], size);
			/* A box measured from these widths fits them. */
			if (run + w > width + 0.01f && j > i)
				break;
			if (u[j] == ' ')
				space = j, before = run;
			run += w;
		}
		from[lines] = i;
		if (j < n && u[j] != '\n' && space > i)
		{
			/* Too long for the line: wrap at a space. */
			wide[lines] = before;
			to[lines++] = space;
			i = space + 1;
		}
		else
		{
			wide[lines] = run;
			to[lines++] = j;
			i = j < n && u[j] != '\n' ? j : j + 1;
		}
	}
	return lines;
}

/* Lines split at newlines only, each kept whole. */
static int lay_kept(fz_context *ctx, ely_pen *p, const int *u, int n, float size, int *from, int *to, float *wide)
{
	int i, lines = 0;
	from[0] = 0;
	wide[0] = 0;
	for (i = 0; i <= n; i++)
	{
		if (i < n && u[i] != '\n')
		{
			wide[lines] += ely_pen_advance(ctx, p, u[i], size);
			continue;
		}
		to[lines++] = i;
		from[lines] = i + 1;
		wide[lines] = 0;
	}
	return lines;
}

/* A resource name in `type` that is free, given `ref`. */
const char *ely_add_resource(fz_context *ctx, pdf_obj *page, pdf_obj *type, const char *stem,
	pdf_obj *ref, char *name, size_t room)
{
	pdf_obj *res = pdf_dict_get_inheritable(ctx, page, PDF_NAME(Resources));
	pdf_obj *dict;
	int n;
	if (!res)
		res = pdf_dict_put_dict(ctx, page, PDF_NAME(Resources), 2);
	dict = pdf_dict_get(ctx, res, type);
	if (!dict)
		dict = pdf_dict_put_dict(ctx, res, type, 2);
	for (n = 1;; n++)
	{
		fz_snprintf(name, room, "%s%d", stem, n);
		if (!pdf_dict_gets(ctx, dict, name))
			break;
	}
	pdf_dict_puts(ctx, dict, name, ref);
	return name;
}

/* The marker of a page whose own content Ely wrapped. */
static const char wrap[] = "q\n%Ely\n";

/* Adds `buf` after the page's content; the new stream. */
pdf_obj *ely_append_content(fz_context *ctx, pdf_document *pdf, pdf_obj *page, fz_buffer *buf)
{
	pdf_obj *contents = pdf_dict_get(ctx, page, PDF_NAME(Contents));
	pdf_obj *list, *first = pdf_is_array(ctx, contents) ? pdf_array_get(ctx, contents, 0) : NULL;
	fz_buffer *head = NULL, *mark = first ? pdf_load_stream(ctx, first) : NULL;
	pdf_obj *added = NULL;
	int wrapped;
	fz_var(head);
	fz_try(ctx)
	{
		wrapped = mark && mark->len == strlen(wrap) && !memcmp(mark->data, wrap, mark->len);
		if (wrapped)
			list = contents;
		else
		{
			list = pdf_new_array(ctx, pdf, 4);
			head = fz_new_buffer_from_copied_data(ctx, (const unsigned char *)wrap, strlen(wrap));
			pdf_array_push_drop(ctx, list, pdf_add_stream(ctx, pdf, head, NULL, 0));
			if (pdf_is_array(ctx, contents))
			{
				int i;
				for (i = 0; i < pdf_array_len(ctx, contents); i++)
					pdf_array_push(ctx, list, pdf_array_get(ctx, contents, i));
			}
			else if (contents)
				pdf_array_push(ctx, list, contents);
			fz_drop_buffer(ctx, head);
			head = fz_new_buffer_from_copied_data(ctx, (const unsigned char *)"Q\n", 2);
			pdf_array_push_drop(ctx, list, pdf_add_stream(ctx, pdf, head, NULL, 0));
			pdf_dict_put_drop(ctx, page, PDF_NAME(Contents), list);
		}
		added = pdf_add_stream(ctx, pdf, buf, NULL, 0);
		pdf_array_push_drop(ctx, list, added);
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, head);
		fz_drop_buffer(ctx, mark);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
	return added;
}

/* Text laid out: the pen, code points, lines, placement. */
typedef struct
{
	ely_pen p;
	int *u, n;
	int *from, *to, lines;
	float *wide;
	fz_point first;
	float gap;
	/* Each line's origin, when lines keep their places. */
	const fz_point *at;
} laid;

static void drop_laid(fz_context *ctx, laid *l)
{
	ely_drop_pen(ctx, &l->p);
	fz_free(ctx, l->u);
	fz_free(ctx, l->from);
	fz_free(ctx, l->to);
	fz_free(ctx, l->wide);
}

/* Where line `line` starts, page space. */
static fz_point line_origin(laid *l, int line)
{
	return l->at ? l->at[line] : fz_make_point(l->first.x, l->first.y + line * l->gap);
}

/* Lays `a` out; a missing glyph, or -1: unwritable font. */
static int lay_text(fz_context *ctx, text_args *a, fz_font *own, pdf_page *page, fz_point first, float gap,
	const fz_point *at, laid *l)
{
	int i;
	if (ely_choose_pen(ctx, a->font, own, page, &l->p) < 0)
		return -1;
	l->n = runes(ctx, a->text, &l->u);
	for (i = 0; i < l->n; i++)
		if (l->u[i] != '\n' && !ely_pen_has(ctx, &l->p, l->u[i]))
			return l->u[i];
	l->from = fz_malloc_array(ctx, l->n + 2, int);
	l->to = fz_malloc_array(ctx, l->n + 2, int);
	l->wide = fz_malloc_array(ctx, l->n + 2, float);
	if (at)
	{
		l->lines = lay_kept(ctx, &l->p, l->u, l->n, a->size, l->from, l->to, l->wide);
		/* A kept line past the box: wrap like any edit. */
		for (i = 0; i < l->lines && at; i++)
			if (at[i].x + l->wide[i] > a->box[0] + a->box[2] + 0.5f)
				at = NULL;
	}
	/* Wrap from the first line's x to the box edge. */
	if (!at)
		l->lines = lay_out(ctx, &l->p, l->u, l->n, a->size, a->box[0] + a->box[2] - first.x, l->from, l->to, l->wide);
	l->first = first;
	l->gap = gap;
	l->at = at;
	return 0;
}

/* The area laid text covers, in page space. */
static fz_rect laid_area(laid *l, float size)
{
	fz_rect area = fz_empty_rect;
	int line;
	for (line = 0; line < l->lines; line++)
	{
		fz_point o = line_origin(l, line);
		area = fz_union_rect(area, fz_make_rect(o.x, o.y - size, o.x + l->wide[line], o.y + size * 0.3f));
	}
	return area;
}

/* Writes laid text onto `page` of `pdf`. */
static void draw_laid(fz_context *ctx, pdf_document *pdf, pdf_page *page, laid *l, text_args *a)
{
	fz_buffer *buf = NULL;
	fz_var(buf);
	fz_try(ctx)
	{
		fz_matrix to_user = ely_to_user(ctx, page);
		char name[32];
		int i, line;
		ely_pen_resource(ctx, pdf, page, &l->p, name, sizeof name);
		buf = fz_new_buffer(ctx, 256);
		fz_append_string(ctx, buf, "q\n");
		for (line = 0; line < l->lines; line++)
		{
			fz_point o = line_origin(l, line);
			fz_matrix tm = fz_concat(fz_make_matrix(1, 0, 0, -1, o.x, o.y), to_user);
			fz_append_printf(ctx, buf, "BT /%s %g Tf %g %g %g rg %g %g %g %g %g %g Tm <", name, a->size,
				((a->rgb >> 16) & 255) / 255.0f, ((a->rgb >> 8) & 255) / 255.0f, (a->rgb & 255) / 255.0f,
				tm.a, tm.b, tm.c, tm.d, tm.e, tm.f);
			for (i = l->from[line]; i < l->to[line]; i++)
				ely_pen_code(ctx, buf, &l->p, l->u[i]);
			fz_append_string(ctx, buf, "> Tj ET\n");
		}
		fz_append_string(ctx, buf, "Q\n");
		ely_append_content(ctx, pdf, page->obj, buf);
	}
	fz_always(ctx)
		fz_drop_buffer(ctx, buf);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* Edits a snapshot copy and draws `region`; the document stays. */
static void preview(fz_context *ctx, pdf_document *pdf, int number, int block, const char *expected, text_args *a,
	laid *l, const char *password, fz_rect region, float scale, unsigned char **pixels, int *w, int *h)
{
	fz_buffer *buf = fz_new_buffer(ctx, 4096);
	fz_output *out = NULL;
	fz_document *copy = NULL;
	pdf_page *page = NULL;
	ely_shape shape = { NULL };
	fz_var(out);
	fz_var(copy);
	fz_var(page);
	fz_try(ctx)
	{
		int tile[4];
		out = fz_new_output_with_buffer(ctx, buf);
		pdf_write_snapshot(ctx, pdf, out);
		fz_close_output(ctx, out);
		copy = fz_open_document_with_buffer(ctx, "application/pdf", buf);
		if (fz_needs_password(ctx, copy) && !fz_authenticate_password(ctx, copy, password))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "the copy refused its password");
		page = pdf_load_page(ctx, ely_pdf_of(ctx, copy), number);
		if (block >= 0)
		{
			ely_read_block(ctx, copy, page, number, block, expected, &shape);
			ely_remove_block(ctx, ely_pdf_of(ctx, copy), page, &shape);
		}
		draw_laid(ctx, ely_pdf_of(ctx, copy), page, l, a);
		region = fz_expand_rect(region, 8);
		tile[0] = (int)floorf(region.x0 * scale), tile[1] = (int)floorf(region.y0 * scale);
		tile[2] = (int)ceilf(region.x1 * scale) - tile[0], tile[3] = (int)ceilf(region.y1 * scale) - tile[1];
		*pixels = fz_malloc(ctx, (size_t)tile[2] * tile[3] * 4);
		if (ely_render(ctx, copy, number, scale, 0, tile, *pixels) < 0)
			fz_throw(ctx, FZ_ERROR_GENERIC, "%s", ely_error());
		*w = tile[2], *h = tile[3];
	}
	fz_always(ctx)
	{
		ely_drop_shape(ctx, &shape);
		fz_drop_page(ctx, (fz_page *)page);
		fz_drop_document(ctx, copy);
		fz_drop_output(ctx, out);
		fz_drop_buffer(ctx, buf);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* How many newlines `text` holds. */
static int newlines(const char *text)
{
	int n = 0;
	for (; *text; text++)
		n += *text == '\n';
	return n;
}

/* Writes or previews text: 0, 2 missing, 3 unwritable, -1. */
int ely_put_text(fz_context *ctx, fz_document *doc, int number, int block, const char *expected, const char *text,
	const char *font, float size, int rgb, const float box[3], const char *password, float scale, int *missing,
	int *grew, unsigned char **pixels, int *w, int *h)
{
	pdf_document *pdf = NULL;
	pdf_page *page = NULL;
	ely_shape shape = { NULL };
	laid l;
	int begun = 0;
	fz_var(pdf);
	fz_var(page);
	fz_var(begun);
	memset(&l, 0, sizeof l);
	*pixels = NULL;
	*grew = 0;
	fz_try(ctx)
	{
		text_args a = { text, font, size, rgb, { box[0], box[1], box[2] } };
		fz_point first = fz_make_point(box[0], box[1] + size * 0.8f);
		float gap = size * 1.2f;
		const fz_point *at = NULL;
		pdf = ely_pdf_of(ctx, doc);
		page = pdf_load_page(ctx, pdf, number);
		/* Checks precede the step, so redo survives a refusal. */
		if (block >= 0)
		{
			ely_read_block(ctx, doc, page, number, block, expected, &shape);
			first = shape.first;
			gap = shape.gap;
			/* Same lines, size and width: each line stays put. */
			if (newlines(text) + 1 == shape.count && fabsf(size - shape.size) <= 0.01f
				&& fabsf(box[2] - (shape.bbox.x1 - shape.bbox.x0)) <= 0.5f)
				at = shape.at;
		}
		*missing = lay_text(ctx, &a, shape.font, page, first, gap, at, &l);
		if (!*missing)
		{
			fz_rect drawn = laid_area(&l, size), region = drawn;
			if (block >= 0)
			{
				fz_rect room = fz_expand_rect(shape.bbox, 1);
				region = fz_union_rect(shape.bbox, drawn);
				*grew = fabsf(size - shape.size) > 0.01f || !fz_contains_rect(room, drawn);
			}
			if (scale > 0)
				preview(ctx, pdf, number, block, expected, &a, &l, password, region, scale, pixels, w, h);
			else
			{
				pdf_begin_operation(ctx, pdf, block >= 0 ? "Edit text" : "Add text");
				begun = 1;
				if (block >= 0)
					ely_remove_block(ctx, pdf, page, &shape);
				draw_laid(ctx, pdf, page, &l, &a);
				pdf_end_operation(ctx, pdf);
				begun = 0;
			}
		}
	}
	fz_always(ctx)
	{
		drop_laid(ctx, &l);
		ely_drop_shape(ctx, &shape);
		fz_drop_page(ctx, (fz_page *)page);
	}
	fz_catch(ctx)
	{
		fz_free(ctx, *pixels);
		*pixels = NULL;
		return ely_edit_failed(ctx, pdf, begun);
	}
	return *missing < 0 ? 3 : *missing ? 2 : 0;
}
