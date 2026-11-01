/* Moves a text block as drawn: fonts, spacing, colour (PDF-03). */

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

/* The page's content drawing only the block's glyphs. */
static fz_buffer *block_content(fz_context *ctx, pdf_document *pdf, pdf_page *page, ely_shape *shape)
{
	pdf_sanitize_filter_options sopts = { 0 };
	pdf_filter_factory list[2] = { { pdf_new_sanitize_filter, NULL }, { NULL, NULL } };
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
	}
	fz_always(ctx)
		fz_drop_buffer(ctx, all);
	fz_catch(ctx)
		fz_rethrow(ctx);
	return out;
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
		ely_remove_block(ctx, pdf, page, &shape);
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
