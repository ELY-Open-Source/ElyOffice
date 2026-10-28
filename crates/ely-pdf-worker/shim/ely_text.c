/* Text blocks: list, take out glyphs, write new text (PDF-03). */

#include <math.h>
#include <string.h>

#include "ely_shim.h"
#include "mupdf/ucdn.h"

/* The page's text, read the same way every time. */
static fz_stext_page *read_text(fz_context *ctx, fz_document *doc, int number)
{
	fz_stext_options opts = { 0 };
	return fz_new_stext_page_from_page_number(ctx, doc, number, &opts);
}

/* The `index`th text block, or NULL. */
static fz_stext_block *nth_block(fz_stext_page *text, int index)
{
	fz_stext_block *block;
	for (block = text->first_block; block; block = block->next)
		if (block->type == FZ_STEXT_BLOCK_TEXT && index-- == 0)
			return block;
	return NULL;
}

/* Glyph origins to take out, in user space. */
typedef struct
{
	fz_point *at;
	int *c;
	int n;
	int hit;
	/* Page-wide: each target's block, and hits per block. */
	int *owner;
	int *hits;
	/* The glyph each block last counted, so once each. */
	int *seen;
	int glyph;
} targets;

/* The glyphs of `block` with their user-space origins. */
static void block_targets(fz_context *ctx, fz_stext_block *block, fz_matrix to_user, targets *t)
{
	fz_stext_line *line;
	fz_stext_char *ch;
	int n = 0;
	for (line = block->u.t.first_line; line; line = line->next)
		for (ch = line->first_char; ch; ch = ch->next)
			n++;
	t->at = fz_malloc_array(ctx, n + 1, fz_point);
	t->c = fz_malloc_array(ctx, n + 1, int);
	t->n = t->hit = 0;
	for (line = block->u.t.first_line; line; line = line->next)
		for (ch = line->first_char; ch; ch = ch->next)
			if (!(ch->flags & FZ_STEXT_SYNTHETIC))
			{
				t->at[t->n] = fz_transform_point(ch->origin, to_user);
				t->c[t->n++] = ch->c;
			}
}

/* `c` as stext spells it (stext-device.c); ligatures split. */
static int spell(int c, int *out)
{
	static const char *named[] = { "ff", "fi", "fl", "ffi", "ffl", "st", "st" };
	int i, n = 0;
	uint32_t lig[18];
	if (c >= 0xFB00 && c <= 0xFB06)
	{
		for (i = 0; named[c - 0xFB00][i]; i++)
			out[n++] = named[c - 0xFB00][i];
		return n;
	}
	if ((c >= 0xfb00 && c <= 0xfdff) || (c >= 0xfe70 && c <= 0xfefc))
		n = ucdn_compat_decompose((uint32_t)c, lig);
	for (i = 0; i < n; i++)
		out[i] = (int)lig[i];
	if (n > 0)
		return n;
	out[0] = c;
	return 1;
}

/* Targets from `i` one glyph spells: a ligature spells several. */
static int spelled(targets *t, int i, const int *ucs, int ucslen)
{
	int j = 1;
	if (ucslen < 2 || ucs[0] != t->c[i])
		return 1;
	while (j < ucslen && i + j < t->n && t->c[i + j] == ucs[j] && (!t->owner || t->owner[i + j] == t->owner[i]))
		j++;
	return j;
}

/* Removes a glyph drawn at one of the targets. */
static int take_glyph(fz_context *ctx, void *opaque, int *ucs, int ucslen, fz_matrix trm, fz_matrix ctm,
	fz_rect bbox, int tr, float ca, float CA)
{
	targets *t = opaque;
	(void)bbox;
	(void)tr;
	(void)ca;
	(void)CA;
	fz_point at = fz_transform_point(fz_transform_point(fz_make_point(0, 0), trm), ctm);
	/* MuPDF gives at most PDF_MRANGE_CAP; each spells to 18. */
	int i, k, found = 0, spelt[18 * PDF_MRANGE_CAP], n = 0;
	t->glyph++;
	if (ucslen > PDF_MRANGE_CAP)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a glyph with %d characters", ucslen);
	/* Match what stext reads: ligature code points spelled out. */
	for (k = 0; k < ucslen; k++)
		n += spell(ucs[k], spelt + n);
	ucs = spelt;
	ucslen = n;
	for (i = 0; i < t->n; i++)
	{
		int match = 0;
		if (fabsf(at.x - t->at[i].x) > 0.5f || fabsf(at.y - t->at[i].y) > 0.5f)
			continue;
		for (k = 0; k < ucslen && !match; k++)
			match = ucs[k] == t->c[i] || t->c[i] == FZ_REPLACEMENT_CHARACTER;
		if (!match)
			continue;
		if (!t->owner)
		{
			t->hit += spelled(t, i, ucs, ucslen);
			return 1;
		}
		/* Every block this glyph matches counts it once. */
		if (t->seen[t->owner[i]] != t->glyph)
		{
			t->seen[t->owner[i]] = t->glyph;
			t->hits[t->owner[i]] += spelled(t, i, ucs, ucslen);
		}
		found = 1;
	}
	t->hit += found;
	return found;
}

/* Runs the glyph filter over the page; `dry` changes nothing. */
static void filter_glyphs(fz_context *ctx, pdf_document *pdf, pdf_page *page, targets *t, int dry)
{
	pdf_sanitize_filter_options sopts = { 0 };
	pdf_filter_factory list[2] = { { pdf_new_sanitize_filter, NULL }, { NULL, NULL } };
	pdf_filter_options opts = { 0 };
	sopts.opaque = t;
	sopts.text_filter = take_glyph;
	list[0].options = &sopts;
	opts.filters = list;
	opts.no_update = dry;
	t->hit = 0;
	pdf_filter_page_contents(ctx, pdf, page, &opts);
}

/* User space from the page space MuPDF reports in. */
fz_matrix ely_to_user(fz_context *ctx, pdf_page *page)
{
	fz_matrix ctm;
	pdf_page_transform(ctx, page, NULL, &ctm);
	return fz_invert_matrix(ctm);
}

/* Whether all of `block` sits in the page's own content. */
static int reachable(fz_context *ctx, pdf_document *pdf, pdf_page *page, fz_stext_block *block)
{
	targets t = { NULL, NULL, 0, 0, NULL, NULL, NULL, 0 };
	int ok = 0;
	fz_var(t.at);
	fz_var(t.c);
	pdf_begin_operation(ctx, pdf, "Look at text");
	fz_var(ok);
	fz_try(ctx)
	{
		block_targets(ctx, block, ely_to_user(ctx, page), &t);
		filter_glyphs(ctx, pdf, page, &t, 1);
		ok = t.n > 0 && t.hit == t.n;
	}
	fz_always(ctx)
	{
		pdf_abandon_operation(ctx, pdf);
		ely_forget_resources(ctx, pdf);
		fz_free(ctx, t.at);
		fz_free(ctx, t.c);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
	return ok;
}

/* Each text block's glyphs found in page content, one pass. */
static void reachable_all(fz_context *ctx, pdf_document *pdf, pdf_page *page, fz_stext_page *text, int n, int *ok)
{
	targets t = { NULL, NULL, 0, 0, NULL, NULL, NULL, 0 };
	int *count = NULL, all = 0, b;
	fz_matrix to_user = ely_to_user(ctx, page);
	fz_stext_block *block;
	fz_stext_line *line;
	fz_stext_char *ch;
	fz_var(t.at);
	fz_var(t.c);
	fz_var(t.owner);
	fz_var(t.hits);
	fz_var(t.seen);
	fz_var(count);
	pdf_begin_operation(ctx, pdf, "Look at text");
	fz_var(all);
	fz_try(ctx)
	{
		for (block = text->first_block; block; block = block->next)
			if (block->type == FZ_STEXT_BLOCK_TEXT)
				for (line = block->u.t.first_line; line; line = line->next)
					for (ch = line->first_char; ch; ch = ch->next)
						all++;
		t.at = fz_malloc_array(ctx, all + 1, fz_point);
		t.c = fz_malloc_array(ctx, all + 1, int);
		t.owner = fz_malloc_array(ctx, all + 1, int);
		t.hits = fz_calloc(ctx, n + 1, sizeof(int));
		t.seen = fz_calloc(ctx, n + 1, sizeof(int));
		count = fz_calloc(ctx, n + 1, sizeof(int));
		b = 0;
		for (block = text->first_block; block; block = block->next)
		{
			if (block->type != FZ_STEXT_BLOCK_TEXT)
				continue;
			for (line = block->u.t.first_line; line; line = line->next)
				for (ch = line->first_char; ch; ch = ch->next)
					if (!(ch->flags & FZ_STEXT_SYNTHETIC))
					{
						t.at[t.n] = fz_transform_point(ch->origin, to_user);
						t.c[t.n] = ch->c;
						t.owner[t.n++] = b;
						count[b]++;
					}
			b++;
		}
		filter_glyphs(ctx, pdf, page, &t, 1);
		for (b = 0; b < n; b++)
			ok[b] = count[b] > 0 && t.hits[b] == count[b];
	}
	fz_always(ctx)
	{
		pdf_abandon_operation(ctx, pdf);
		ely_forget_resources(ctx, pdf);
		fz_free(ctx, t.at);
		fz_free(ctx, t.c);
		fz_free(ctx, t.owner);
		fz_free(ctx, t.hits);
		fz_free(ctx, t.seen);
		fz_free(ctx, count);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* The text of `block`, lines on lines. */
static void block_text(fz_context *ctx, fz_buffer *buf, fz_stext_block *block)
{
	fz_stext_line *line;
	fz_stext_char *ch;
	for (line = block->u.t.first_line; line; line = line->next)
	{
		if (line != block->u.t.first_line)
			fz_append_byte(ctx, buf, '\n');
		for (ch = line->first_char; ch; ch = ch->next)
			fz_append_rune(ctx, buf, ch->c);
	}
}

/* Horizontal lines only; turned text is not edited. */
static int upright(fz_stext_block *block)
{
	fz_stext_line *line;
	for (line = block->u.t.first_line; line; line = line->next)
		if (line->wmode || fabsf(line->dir.x - 1) > 0.01f)
			return 0;
	return 1;
}

/* JSON of the page's text blocks. */
int ely_text_blocks(fz_context *ctx, fz_document *doc, int number, unsigned char **data, size_t *len)
{
	fz_stext_page *text = NULL;
	pdf_page *page = NULL;
	fz_buffer *buf = NULL, *words = NULL;
	fz_output *out = NULL;
	int *reached = NULL;
	fz_var(text);
	fz_var(page);
	fz_var(buf);
	fz_var(words);
	fz_var(out);
	fz_var(reached);
	fz_try(ctx)
	{
		pdf_document *pdf = ely_pdf_of(ctx, doc);
		fz_stext_block *block;
		int first = 1, n = 0, at = 0;
		text = read_text(ctx, doc, number);
		page = pdf_load_page(ctx, pdf, number);
		for (block = text->first_block; block; block = block->next)
			n += block->type == FZ_STEXT_BLOCK_TEXT;
		reached = fz_calloc(ctx, n + 1, sizeof(int));
		reachable_all(ctx, pdf, page, text, n, reached);
		buf = fz_new_buffer(ctx, 1024);
		out = fz_new_output_with_buffer(ctx, buf);
		fz_write_string(ctx, out, "[");
		for (block = text->first_block; block; block = block->next)
		{
			fz_stext_char *ch;
			int editable;
			if (block->type != FZ_STEXT_BLOCK_TEXT)
				continue;
			ch = block->u.t.first_line->first_char;
			editable = reached[at++];
			editable = editable && upright(block);
			fz_drop_buffer(ctx, words);
			words = NULL;
			words = fz_new_buffer(ctx, 256);
			block_text(ctx, words, block);
			fz_terminate_buffer(ctx, words);
			fz_write_printf(ctx, out, "%s{\"rect\":[%g,%g,%g,%g],\"text\":",
				first ? "" : ",", block->bbox.x0, block->bbox.y0, block->bbox.x1, block->bbox.y1);
			ely_json_string(ctx, out, fz_string_from_buffer(ctx, words));
			fz_write_string(ctx, out, ",\"font\":");
			ely_json_string(ctx, out, fz_font_name(ctx, ch->font));
			fz_write_printf(ctx, out, ",\"size\":%g,\"color\":%d,\"editable\":%s}",
				ch->size, (int)(ch->argb & 0xffffff), editable ? "true" : "false");
			first = 0;
		}
		fz_write_string(ctx, out, "]");
		fz_close_output(ctx, out);
		*len = fz_buffer_extract(ctx, buf, data);
	}
	fz_always(ctx)
	{
		fz_drop_output(ctx, out);
		fz_drop_buffer(ctx, buf);
		fz_drop_buffer(ctx, words);
		fz_drop_page(ctx, (fz_page *)page);
		fz_drop_stext_page(ctx, text);
		fz_free(ctx, reached);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* Distinct baselines, and the gap between the first two. */
static void baselines(fz_stext_block *block, ely_shape *shape)
{
	fz_stext_line *line;
	float last = block->u.t.first_line->first_char->origin.y;
	shape->lines = 1;
	shape->gap = 0;
	for (line = block->u.t.first_line->next; line; line = line->next)
	{
		float y = line->first_char->origin.y;
		if (fabsf(y - last) <= 0.5f)
			continue;
		if (shape->lines == 1)
			shape->gap = y - shape->first.y;
		shape->lines++;
		last = y;
	}
	/* Runs on one baseline: an ordinary line gap. */
	if (shape->gap <= 0)
		shape->gap = shape->size * 1.2f;
}

/* Reads block `index` and checks it; nothing changes. */
void ely_read_block(fz_context *ctx, fz_document *doc, pdf_page *page, int number, int index, const char *expected,
	ely_shape *shape)
{
	pdf_document *pdf = ely_pdf_of(ctx, doc);
	fz_stext_page *text = NULL;
	fz_buffer *words = NULL;
	targets *t = fz_malloc_struct(ctx, targets);
	shape->glyphs = t;
	fz_var(text);
	fz_var(words);
	fz_try(ctx)
	{
		fz_stext_block *block;
		fz_stext_line *line;
		fz_stext_char *ch;
		text = read_text(ctx, doc, number);
		block = nth_block(text, index);
		if (!block)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "no text block %d", index);
		words = fz_new_buffer(ctx, 256);
		block_text(ctx, words, block);
		fz_terminate_buffer(ctx, words);
		if (strcmp(fz_string_from_buffer(ctx, words), expected))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "the text changed since it was read");
		if (!upright(block) || !reachable(ctx, pdf, page, block))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "this text is not in the page's own content");
		block_targets(ctx, block, ely_to_user(ctx, page), t);
		ch = block->u.t.first_line->first_char;
		shape->font = fz_keep_font(ctx, ch->font);
		shape->first = ch->origin;
		shape->size = ch->size;
		shape->rgb = (int)(ch->argb & 0xffffff);
		shape->bbox = block->bbox;
		baselines(block, shape);
		for (line = block->u.t.first_line; line; line = line->next)
			shape->count++;
		shape->at = fz_malloc_array(ctx, shape->count, fz_point);
		shape->count = 0;
		for (line = block->u.t.first_line; line; line = line->next)
			shape->at[shape->count++] = line->first_char->origin;
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, words);
		fz_drop_stext_page(ctx, text);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* Takes out the glyphs `ely_read_block` found. */
void ely_remove_block(fz_context *ctx, pdf_document *pdf, pdf_page *page, ely_shape *shape)
{
	targets *t = shape->glyphs;
	filter_glyphs(ctx, pdf, page, t, 0);
	if (t->hit != t->n)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "%d of %d glyphs were found", t->hit, t->n);
}

void ely_drop_shape(fz_context *ctx, ely_shape *shape)
{
	targets *t = shape->glyphs;
	fz_drop_font(ctx, shape->font);
	fz_free(ctx, shape->at);
	shape->at = NULL;
	if (t)
	{
		fz_free(ctx, t->at);
		fz_free(ctx, t->c);
		fz_free(ctx, t);
	}
	shape->font = NULL;
	shape->glyphs = NULL;
}
