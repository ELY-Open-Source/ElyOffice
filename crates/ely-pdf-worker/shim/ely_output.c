/* Page images, text, optimized copies, repaired copies (PDF-13, PDF-14). */

#include <string.h>

#include "ely_shim.h"

/* Page `number` as a PNG at `dpi`, white, as printed. */
int ely_export_image(fz_context *ctx, fz_document *doc, int number, float dpi, const char *path)
{
	fz_pixmap *pixmap = NULL;
	fz_var(pixmap);
	fz_try(ctx)
	{
		if (dpi < 18 || dpi > 1200)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a resolution outside 18 to 1200 dpi");
		ely_watch_begin(ctx);
		pixmap = fz_new_pixmap_from_page_number(ctx, doc, number, fz_scale(dpi / 72, dpi / 72), fz_device_rgb(ctx), 0);
		if (ely_watch_end(ctx))
			fz_throw(ctx, FZ_ERROR_FORMAT, "page %d did not read whole: %s", number + 1, ely_open_warnings());
		fz_save_pixmap_as_png(ctx, pixmap, path);
	}
	fz_always(ctx)
	{
		ely_watch_end(ctx);
		fz_drop_pixmap(ctx, pixmap);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* Every page's text, UTF-8, pages parted by form feeds. */
int ely_export_text(fz_context *ctx, fz_document *doc, const char *path)
{
	fz_output *out = NULL;
	fz_stext_page *text = NULL;
	fz_buffer *buf = NULL;
	fz_var(out);
	fz_var(text);
	fz_var(buf);
	fz_try(ctx)
	{
		int i;
		ely_watch_begin(ctx);
		out = fz_new_output_with_path(ctx, path, 0);
		for (i = 0; i < fz_count_pages(ctx, doc); i++)
		{
			if (i > 0)
				fz_write_byte(ctx, out, '\f');
			text = fz_new_stext_page_from_page_number(ctx, doc, i, NULL);
			buf = fz_new_buffer_from_stext_page(ctx, text);
			fz_write_buffer(ctx, out, buf);
			fz_drop_buffer(ctx, buf);
			buf = NULL;
			fz_drop_stext_page(ctx, text);
			text = NULL;
		}
		/* Partial text is no export. */
		if (ely_watch_end(ctx))
			fz_throw(ctx, FZ_ERROR_FORMAT, "the text did not read whole: %s", ely_open_warnings());
		fz_close_output(ctx, out);
	}
	fz_always(ctx)
	{
		ely_watch_end(ctx);
		fz_drop_buffer(ctx, buf);
		fz_drop_stext_page(ctx, text);
		fz_drop_output(ctx, out);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

typedef struct
{
	int from, to;
} downsample;

static void shrink_images(fz_context *ctx, fz_document *copy, void *arg)
{
	downsample *d = arg;
	pdf_image_rewriter_options opts = { 0 };
	if (!d->to)
		return;
	opts.color_lossless_image_subsample_method = opts.color_lossy_image_subsample_method = FZ_SUBSAMPLE_BICUBIC;
	opts.gray_lossless_image_subsample_method = opts.gray_lossy_image_subsample_method = FZ_SUBSAMPLE_BICUBIC;
	opts.bitonal_image_subsample_method = FZ_SUBSAMPLE_BICUBIC;
	opts.color_lossless_image_subsample_threshold = opts.color_lossy_image_subsample_threshold = d->from;
	opts.gray_lossless_image_subsample_threshold = opts.gray_lossy_image_subsample_threshold = d->from;
	opts.bitonal_image_subsample_threshold = d->from;
	opts.color_lossless_image_subsample_to = opts.color_lossy_image_subsample_to = d->to;
	opts.gray_lossless_image_subsample_to = opts.gray_lossy_image_subsample_to = d->to;
	opts.bitonal_image_subsample_to = d->to;
	/* Never, the default, drops the smaller picture. */
	opts.color_lossless_image_recompress_method = opts.gray_lossless_image_recompress_method = FZ_RECOMPRESS_LOSSLESS;
	opts.color_lossy_image_recompress_method = opts.gray_lossy_image_recompress_method = FZ_RECOMPRESS_SAME;
	opts.bitonal_image_recompress_method = FZ_RECOMPRESS_SAME;
	pdf_rewrite_images(ctx, ely_pdf_of(ctx, copy), &opts);
}

/* Writes `out` compressed, images above `from` dpi taken to `to`. */
int ely_optimize(fz_context *ctx, fz_document *doc, const char *raw, const char *out, const char *password, int compress,
	int from, int to)
{
	fz_try(ctx)
	{
		pdf_write_options opts = pdf_default_write_options;
		downsample d = { from, to };
		if (to && (to < 36 || from <= to))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a downsampling from %d to %d dpi", from, to);
		opts.do_compress = opts.do_compress_images = opts.do_compress_fonts = compress;
		ely_save_copy(ctx, ely_pdf_of(ctx, doc), raw, out, password, &opts, shrink_images, &d);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* 1 when MuPDF rebuilt the file's structure to read it. */
int ely_repaired(fz_context *ctx, fz_document *doc)
{
	int repaired = 0;
	fz_var(repaired);
	fz_try(ctx)
		repaired = pdf_was_repaired(ctx, ely_pdf_of(ctx, doc));
	fz_catch(ctx)
		return ely_fail(ctx);
	return repaired;
}

/* The repaired document written whole to `path`. */
int ely_write_repaired(fz_context *ctx, fz_document *doc, const char *path)
{
	fz_try(ctx)
	{
		pdf_write_options opts = pdf_default_write_options;
		opts.do_garbage = 1;
		pdf_save_document(ctx, ely_pdf_of(ctx, doc), path, &opts);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* Marked for print and not hidden. */
static int prints(fz_context *ctx, pdf_annot *annot)
{
	int flags = pdf_annot_flags(ctx, annot);
	return (flags & PDF_ANNOT_IS_PRINT) && !(flags & (PDF_ANNOT_IS_HIDDEN | PDF_ANNOT_IS_INVISIBLE));
}

/* Asks MuPDF to make and keep a printable annotation's look. */
static void ask_look(fz_context *ctx, pdf_annot *annot)
{
	if (prints(ctx, annot))
		pdf_annot_request_synthesis(ctx, annot);
}

/* Draws a printable look into `buf`, named in `xobj`. */
static void draw_look(fz_context *ctx, pdf_annot *annot, int page, pdf_obj *xobj, fz_buffer *buf)
{
	pdf_obj *type = pdf_dict_get(ctx, pdf_annot_obj(ctx, annot), PDF_NAME(Subtype));
	pdf_obj *ap;
	fz_matrix m;
	char name[32];
	int n = 0;
	/* Links and popups have no look of their own. */
	if (!prints(ctx, annot) || type == PDF_NAME(Link) || type == PDF_NAME(Popup))
		return;
	if (!pdf_dict_getp(ctx, pdf_annot_obj(ctx, annot), "AP/N"))
		fz_throw(ctx, FZ_ERROR_FORMAT, "a %s annotation on page %d cannot be drawn", pdf_to_name(ctx, type), page + 1);
	/* A state with no look of its own draws nothing. */
	ap = pdf_annot_ap(ctx, annot);
	if (!ap)
		return;
	do
		fz_snprintf(name, sizeof name, "ElyAnnot%d", n++);
	while (pdf_dict_gets(ctx, xobj, name));
	pdf_dict_puts(ctx, xobj, name, ap);
	pdf_dict_put(ctx, ap, PDF_NAME(Type), PDF_NAME(XObject));
	pdf_dict_put(ctx, ap, PDF_NAME(Subtype), PDF_NAME(Form));
	m = pdf_annot_transform(ctx, annot);
	fz_append_printf(ctx, buf, "q %g %g %g %g %g %g cm /%s Do Q\n", m.a, m.b, m.c, m.d, m.e, m.f, name);
}

/* Draws the printable looks in; drops all but links. */
static void flatten(fz_context *ctx, pdf_document *pdf, pdf_page *page, int number)
{
	pdf_obj *annots = pdf_dict_get(ctx, page->obj, PDF_NAME(Annots));
	pdf_obj *res, *xobj, *contents, *links = NULL, *prologue = NULL;
	fz_buffer *buf = NULL;
	pdf_annot *annot;
	int prepend, append, i;
	if (pdf_array_len(ctx, annots) == 0)
		return;
	res = pdf_dict_get(ctx, page->obj, PDF_NAME(Resources));
	if (!res)
	{
		res = pdf_dict_get_inheritable(ctx, page->obj, PDF_NAME(Resources));
		if (res)
			pdf_dict_put(ctx, page->obj, PDF_NAME(Resources), res);
		else
			res = pdf_dict_put_dict(ctx, page->obj, PDF_NAME(Resources), 4);
	}
	xobj = pdf_dict_get(ctx, res, PDF_NAME(XObject));
	if (!xobj)
		xobj = pdf_dict_put_dict(ctx, res, PDF_NAME(XObject), 8);
	fz_var(buf);
	fz_var(links);
	fz_var(prologue);
	fz_try(ctx)
	{
		/* The page's own state is closed before the looks. */
		contents = pdf_dict_get(ctx, page->obj, PDF_NAME(Contents));
		pdf_count_q_balance(ctx, pdf, res, contents, &prepend, &append);
		buf = fz_new_buffer(ctx, 1024);
		while (prepend-- > 0)
			fz_append_string(ctx, buf, "q\n");
		prologue = pdf_add_stream(ctx, pdf, buf, NULL, 0);
		fz_clear_buffer(ctx, buf);
		while (append-- > 0)
			fz_append_string(ctx, buf, "Q\n");
		for (annot = pdf_first_annot(ctx, page); annot; annot = pdf_next_annot(ctx, annot))
			draw_look(ctx, annot, number, xobj, buf);
		for (annot = pdf_first_widget(ctx, page); annot; annot = pdf_next_widget(ctx, annot))
			draw_look(ctx, annot, number, xobj, buf);
		links = pdf_new_array(ctx, pdf, 4);
		for (i = 0; i < pdf_array_len(ctx, annots); i++)
			if (pdf_dict_get(ctx, pdf_array_get(ctx, annots, i), PDF_NAME(Subtype)) == PDF_NAME(Link))
				pdf_array_push(ctx, links, pdf_array_get(ctx, annots, i));
		pdf_dict_put(ctx, page->obj, PDF_NAME(Annots), links);
		if (!pdf_is_array(ctx, contents))
		{
			pdf_obj *all = pdf_dict_put_array(ctx, page->obj, PDF_NAME(Contents), 3);
			if (contents)
				pdf_array_push(ctx, all, contents);
			contents = all;
		}
		pdf_array_insert(ctx, contents, prologue, 0);
		pdf_array_push_drop(ctx, contents, pdf_add_stream(ctx, pdf, buf, NULL, 0));
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, buf);
		pdf_drop_obj(ctx, links);
		pdf_drop_obj(ctx, prologue);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* Every page as it prints; no form is left. */
static void flatten_all(fz_context *ctx, fz_document *copy, void *arg)
{
	pdf_document *pdf = ely_pdf_of(ctx, copy);
	pdf_page *page = NULL;
	pdf_annot *annot;
	int i, n = pdf_count_pages(ctx, pdf);
	fz_var(page);
	fz_try(ctx)
	{
		for (i = 0; i < n; i++)
		{
			page = pdf_load_page(ctx, pdf, i);
			for (annot = pdf_first_annot(ctx, page); annot; annot = pdf_next_annot(ctx, annot))
				ask_look(ctx, annot);
			for (annot = pdf_first_widget(ctx, page); annot; annot = pdf_next_widget(ctx, annot))
				ask_look(ctx, annot);
			pdf_update_page(ctx, page);
			flatten(ctx, pdf, page, i);
			fz_drop_page(ctx, &page->super);
			page = NULL;
		}
		pdf_dict_del(ctx, pdf_dict_get(ctx, pdf_trailer(ctx, pdf), PDF_NAME(Root)), PDF_NAME(AcroForm));
	}
	fz_always(ctx)
		fz_drop_page(ctx, (fz_page *)page);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* Leave to print was checked; the printer gets no lock. */
int ely_print_copy(fz_context *ctx, fz_document *doc, const char *raw, const char *out, const char *password)
{
	fz_try(ctx)
	{
		pdf_write_options opts = pdf_default_write_options;
		opts.do_encrypt = PDF_ENCRYPT_NONE;
		ely_save_copy(ctx, ely_pdf_of(ctx, doc), raw, out, password, &opts, flatten_all, NULL);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* Decodes every picture; 1 if MuPDF warned on any. */
int ely_check_pictures(fz_context *ctx, fz_document *doc)
{
	pdf_document *pdf = ely_pdf_of(ctx, doc);
	int i;
	ely_watch_begin(ctx);
	for (i = 1; i < pdf_xref_len(ctx, pdf); i++)
	{
		pdf_obj *obj = NULL;
		fz_buffer *bytes = NULL;
		fz_var(obj);
		fz_var(bytes);
		fz_try(ctx)
		{
			obj = pdf_new_indirect(ctx, pdf, i, 0);
			if (pdf_is_stream(ctx, obj) && pdf_name_eq(ctx, pdf_dict_get(ctx, obj, PDF_NAME(Subtype)), PDF_NAME(Image)))
				bytes = pdf_load_stream_number(ctx, pdf, i);
		}
		fz_always(ctx)
		{
			fz_drop_buffer(ctx, bytes);
			pdf_drop_obj(ctx, obj);
		}
		fz_catch(ctx)
		{
			fz_warn(ctx, "picture %d: %s", i, fz_caught_message(ctx));
			fz_ignore_error(ctx);
		}
	}
	return ely_watch_end(ctx);
}
