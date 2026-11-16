/* Saving: a compacted copy of the open document. */

#include "ely_shim.h"

/* New and rewritten streams compress; compressed ones stay. */
pdf_write_options ely_save_options(void)
{
	pdf_write_options opts = pdf_default_write_options;
	opts.do_compress = opts.do_compress_images = opts.do_compress_fonts = 1;
	return opts;
}

/* A snapshot, reopened, changed, unsigned, compacted to `out`. */
void ely_save_copy(fz_context *ctx, pdf_document *pdf, const char *raw, const char *out, const char *password,
	const pdf_write_options *opts, ely_change *change, void *arg)
{
	fz_document *copy = NULL;
	fz_var(copy);
	fz_try(ctx)
	{
		pdf_write_options compact = opts ? *opts : ely_save_options();
		pdf_save_snapshot(ctx, pdf, raw);
		copy = fz_open_document(ctx, raw);
		if (fz_needs_password(ctx, copy) && !fz_authenticate_password(ctx, copy, password))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "the written copy refused its password");
		if (change)
			change(ctx, copy, arg);
		ely_unsign(ctx, ely_pdf_of(ctx, copy));
		/* Unused objects, such as deleted pages, go. */
		compact.do_garbage = 2;
		pdf_save_document(ctx, ely_pdf_of(ctx, copy), out, &compact);
	}
	fz_always(ctx)
		fz_drop_document(ctx, copy);
	fz_catch(ctx)
		fz_rethrow(ctx);
}
