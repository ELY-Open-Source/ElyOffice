/* Moves a text block as drawn: fonts, spacing, colour (PDF-03). */

#include <stdlib.h>
#include <string.h>

#include "ely_shim.h"

static void hold_buffer(fz_context *ctx, fz_buffer *buf, void *opaque)
{
	*(fz_buffer **)opaque = fz_keep_buffer(ctx, buf);
}

/* `in` without its form drawing: each "/Name Do" line. */
static fz_buffer *without_forms(fz_context *ctx, fz_buffer *in)
{
	fz_buffer *out = fz_new_buffer(ctx, in->len + 1);
	size_t at = 0, end;
	fz_try(ctx)
		for (; at < in->len; at = end + 1)
		{
			for (end = at; end < in->len && in->data[end] != '\n'; end++)
				;
			if (end - at > 4 && in->data[at] == '/' && !memcmp(in->data + end - 3, " Do", 3))
				continue;
			fz_append_data(ctx, out, in->data + at, end - at);
			fz_append_byte(ctx, out, '\n');
		}
	fz_catch(ctx)
	{
		fz_drop_buffer(ctx, out);
		fz_rethrow(ctx);
	}
	return out;
}

/* Operators that mark nothing: state, text state, clip paths. */
static const char *quiet[] = { "q", "Q", "cm", "gs", "w", "J", "j", "M", "d", "ri", "i", "BT", "ET", "Tc", "Tw",
	"Tz", "TL", "Tf", "Tr", "Ts", "Td", "TD", "Tm", "T*", "CS", "cs", "SC", "SCN", "sc", "scn", "G", "g", "RG", "rg",
	"K", "k", "m", "l", "c", "v", "y", "h", "re", "W", "W*", "n", "BX", "EX" };

/* Whether the line `s`, `n` bytes, ends with operator `op`. */
static int is_op(const unsigned char *s, size_t n, const char *op)
{
	size_t k = strlen(op);
	return n >= k && !memcmp(s + n - k, op, k) && (n == k || memchr(" )>]", s[n - k - 1], 4));
}

static int is_quiet(const unsigned char *s, size_t n)
{
	size_t i;
	for (i = 0; i < nelem(quiet); i++)
		if (is_op(s, n, quiet[i]))
			return 1;
	return 0;
}

/* The MCID a BDC line names, or -1. */
static int mcid_of(const unsigned char *s, size_t n)
{
	size_t i;
	if (!is_op(s, n, "BDC"))
		return -1;
	for (i = 0; i + 6 < n; i++)
		if (!memcmp(s + i, "/MCID ", 6))
			return atoi((const char *)s + i + 6);
	return -1;
}

static size_t line_end(fz_buffer *buf, size_t at)
{
	while (at < buf->len && buf->data[at] != '\n')
		at++;
	return at;
}

typedef struct
{
	size_t line, at, n;
	int marks, mcid;
} open_tag;

/* Whether `buf` opens the same tag: MCID, or named properties. */
static int repeats(fz_buffer *buf, const unsigned char *s, open_tag *tag)
{
	size_t at, end;
	int named = tag->mcid < 0 && is_op(s, tag->n, "BDC") && !memchr(s, '<', tag->n);
	for (at = 0; at < buf->len; at = end + 1)
	{
		end = line_end(buf, at);
		if (tag->mcid >= 0 && mcid_of(buf->data + at, end - at) == tag->mcid)
			return 1;
		if (named && end - at == tag->n && !memcmp(buf->data + at, s, tag->n))
			return 1;
	}
	return 0;
}

/* Drops empty tags, with `other` just those it repeats. */
static void unmark_empty(fz_context *ctx, fz_buffer *buf, void *other)
{
	size_t at, end, lines = 1, line = 0, depth = 0;
	open_tag *open = NULL;
	char *drop = NULL;
	fz_buffer *out = NULL;
	fz_var(open);
	fz_var(drop);
	fz_var(out);
	fz_try(ctx)
	{
		for (at = 0; at < buf->len; at++)
			lines += buf->data[at] == '\n';
		open = fz_malloc_array(ctx, lines, open_tag);
		drop = fz_calloc(ctx, lines, 1);
		for (at = 0; at < buf->len; at = end + 1, line++)
		{
			const unsigned char *s = buf->data + at;
			end = line_end(buf, at);
			if (is_op(s, end - at, "BDC") || is_op(s, end - at, "BMC"))
			{
				open[depth].line = line;
				open[depth].at = at;
				open[depth].n = end - at;
				open[depth].marks = 0;
				open[depth++].mcid = mcid_of(s, end - at);
			}
			else if (is_op(s, end - at, "EMC") && depth > 0)
			{
				open_tag *tag = &open[--depth];
				int twice = other && repeats(other, buf->data + tag->at, tag);
				if (tag->marks && twice)
					fz_throw(ctx, FZ_ERROR_ARGUMENT, "this text shares its tag with other content");
				if (!tag->marks && (twice || !other))
					drop[tag->line] = drop[line] = 1;
				if (tag->marks && depth > 0)
					open[depth - 1].marks = 1;
			}
			else if (depth > 0 && !is_quiet(s, end - at))
				open[depth - 1].marks = 1;
		}
		out = fz_new_buffer(ctx, buf->len + 1);
		for (at = 0, line = 0; at < buf->len; at = end + 1, line++)
		{
			end = line_end(buf, at);
			if (!drop[line])
			{
				fz_append_data(ctx, out, buf->data + at, end - at);
				fz_append_byte(ctx, out, '\n');
			}
		}
		fz_clear_buffer(ctx, buf);
		fz_append_buffer(ctx, buf, out);
	}
	fz_always(ctx)
	{
		fz_free(ctx, open);
		fz_free(ctx, drop);
		fz_drop_buffer(ctx, out);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* The sanitizer, kept off the structure tree. */
static pdf_processor *untagged(fz_context *ctx, pdf_document *doc, pdf_processor *chain, int struct_parents,
	fz_matrix transform, pdf_filter_options *options, void *sopts)
{
	(void)struct_parents;
	return pdf_new_sanitize_filter(ctx, doc, chain, -1, transform, options, sopts);
}

/* Notes text that clips: here only text painted by patterns. */
typedef struct
{
	fz_device super;
	int patterned;
} pattern_dev;

static void clip_text(fz_context *ctx, fz_device *dev, const fz_text *text, fz_matrix ctm, fz_rect scissor)
{
	(void)ctx;
	(void)text;
	(void)ctm;
	(void)scissor;
	((pattern_dev *)dev)->patterned = 1;
}

static void clip_stroke_text(fz_context *ctx, fz_device *dev, const fz_text *text, const fz_stroke_state *stroke,
	fz_matrix ctm, fz_rect scissor)
{
	(void)stroke;
	clip_text(ctx, dev, text, ctm, scissor);
}

/* Pattern space stays with the page; such paint cannot move. */
static void refuse_patterns(fz_context *ctx, pdf_document *pdf, pdf_obj *res, fz_buffer *drawn)
{
	pattern_dev *d = fz_new_derived_device(ctx, pattern_dev);
	int patterned = 0;
	d->super.clip_text = clip_text;
	d->super.clip_stroke_text = clip_stroke_text;
	fz_try(ctx)
	{
		pdf_run_glyph(ctx, pdf, res, drawn, &d->super, fz_identity, NULL, NULL, NULL, NULL);
		fz_close_device(ctx, &d->super);
		patterned = d->patterned;
	}
	fz_always(ctx)
		fz_drop_device(ctx, &d->super);
	fz_catch(ctx)
		fz_rethrow(ctx);
	if (patterned)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "this text is painted with a pattern");
}

/* The page's content drawing only the block's glyphs. */
static fz_buffer *block_content(fz_context *ctx, pdf_document *pdf, pdf_page *page, ely_shape *shape)
{
	pdf_sanitize_filter_options sopts = { 0 };
	pdf_filter_factory list[2] = { { untagged, NULL }, { NULL, NULL } };
	pdf_filter_options opts = { 0 };
	fz_buffer *all = NULL, *out = NULL;
	fz_var(all);
	fz_var(out);
	list[0].options = &sopts;
	opts.filters = list;
	opts.no_update = 1;
	opts.ascii = 1;
	opts.newlines = 1;
	opts.opaque = &all;
	opts.complete = hold_buffer;
	fz_try(ctx)
	{
		ely_keep_block(ctx, pdf, page, shape, &opts);
		out = without_forms(ctx, all);
		unmark_empty(ctx, out, NULL);
	}
	fz_always(ctx)
		fz_drop_buffer(ctx, all);
	fz_catch(ctx)
		fz_rethrow(ctx);
	return out;
}

/* Takes the block out, tags kept off the structure tree. */
static void take_block(fz_context *ctx, pdf_document *pdf, pdf_page *page, ely_shape *shape, fz_buffer *drawn)
{
	pdf_sanitize_filter_options sopts = { 0 };
	pdf_filter_factory list[2] = { { untagged, NULL }, { NULL, NULL } };
	pdf_filter_options opts = { 0 };
	list[0].options = &sopts;
	opts.filters = list;
	opts.ascii = 1;
	opts.newlines = 1;
	opts.opaque = drawn;
	opts.complete = unmark_empty;
	ely_take_block(ctx, pdf, page, shape, &opts);
}

int ely_move_text(fz_context *ctx, fz_document *doc, int number, int block, const char *expected, float dx, float dy)
{
	pdf_document *pdf = NULL;
	pdf_page *page = NULL;
	ely_shape shape = { NULL };
	fz_buffer *drawn = NULL, *buf = NULL;
	pdf_obj *res = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(page);
	fz_var(drawn);
	fz_var(buf);
	fz_var(res);
	fz_var(begun);
	fz_try(ctx)
	{
		fz_point by;
		pdf = ely_pdf_of(ctx, doc);
		page = pdf_load_page(ctx, pdf, number);
		ely_read_block(ctx, doc, page, number, block, expected, &shape);
		/* Page space down is user space up. */
		by = fz_transform_vector(fz_make_point(dx, dy), ely_to_user(ctx, page));
		pdf_begin_operation(ctx, pdf, "Move text");
		begun = 1;
		res = pdf_keep_obj(ctx, pdf_page_resources(ctx, page));
		drawn = block_content(ctx, pdf, page, &shape);
		refuse_patterns(ctx, pdf, res, drawn);
		take_block(ctx, pdf, page, &shape, drawn);
		/* Both streams name only what the old resources hold. */
		pdf_dict_put(ctx, page->obj, PDF_NAME(Resources), res);
		buf = fz_new_buffer(ctx, drawn->len + 64);
		fz_append_printf(ctx, buf, "q 1 0 0 1 %g %g cm\n", by.x, by.y);
		fz_append_buffer(ctx, buf, drawn);
		fz_append_string(ctx, buf, "Q\n");
		ely_append_content(ctx, pdf, page->obj, buf);
		pdf_end_operation(ctx, pdf);
		begun = 0;
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, drawn);
		fz_drop_buffer(ctx, buf);
		pdf_drop_obj(ctx, res);
		ely_drop_shape(ctx, &shape);
		fz_drop_page(ctx, (fz_page *)page);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}
