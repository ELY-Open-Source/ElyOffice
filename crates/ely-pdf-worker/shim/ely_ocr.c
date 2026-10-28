/* OCR: recognise a page, add a hidden text layer (PDF-08). */

#include <string.h>

#include "ely_shim.h"

static const char to_unicode[] =
	"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n"
	"/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n"
	"/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n"
	"1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n"
	"1 beginbfrange\n<0000> <FFFF> <0000>\nendbfrange\n"
	"endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n";

/* Lines Tesseract reads on a page, page space, as JSON. */
int ely_ocr_page(fz_context *ctx, fz_document *doc, int number, const char *lang, const char *datadir, float dpi,
	unsigned char **data, size_t *len)
{
	fz_page *page = NULL;
	fz_pixmap *pix = NULL;
	fz_image *image = NULL;
	fz_stext_page *text = NULL;
	fz_device *sdev = NULL, *ocr = NULL;
	fz_buffer *buf = NULL, *words = NULL;
	fz_output *out = NULL;
	fz_var(page);
	fz_var(pix);
	fz_var(image);
	fz_var(text);
	fz_var(sdev);
	fz_var(ocr);
	fz_var(buf);
	fz_var(words);
	fz_var(out);
	fz_try(ctx)
	{
		float zoom = dpi / 72;
		fz_rect area;
		fz_stext_block *block;
		fz_stext_line *line;
		fz_stext_char *ch;
		int first = 1;
		if (dpi < 72 || dpi > 600)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a resolution of %g dpi", dpi);
		page = fz_load_page(ctx, doc, number);
		pix = fz_new_pixmap_from_page(ctx, page, fz_scale(zoom, zoom), fz_device_gray(ctx), 0);
		image = fz_new_image_from_pixmap(ctx, pix, NULL);
		area = fz_make_rect(0, 0, (float)pix->w, (float)pix->h);
		text = fz_new_stext_page(ctx, area);
		sdev = fz_new_stext_device(ctx, text, NULL);
		ocr = fz_new_ocr_device(ctx, sdev, fz_identity, area, 0, lang, datadir, NULL, NULL);
		fz_fill_image(ctx, ocr, image, fz_make_matrix(area.x1, 0, 0, area.y1, 0, 0), 1, fz_default_color_params);
		fz_close_device(ctx, ocr);
		fz_close_device(ctx, sdev);
		buf = fz_new_buffer(ctx, 1024);
		out = fz_new_output_with_buffer(ctx, buf);
		fz_write_byte(ctx, out, '[');
		for (block = text->first_block; block; block = block->next)
		{
			if (block->type != FZ_STEXT_BLOCK_TEXT)
				continue;
			for (line = block->u.t.first_line; line; line = line->next)
			{
				fz_rect r = fz_transform_rect(line->bbox, fz_scale(1 / zoom, 1 / zoom));
				if (!line->first_char)
					continue;
				fz_drop_buffer(ctx, words);
				words = NULL;
				words = fz_new_buffer(ctx, 128);
				for (ch = line->first_char; ch; ch = ch->next)
					fz_append_rune(ctx, words, ch->c);
				fz_terminate_buffer(ctx, words);
				fz_write_printf(ctx, out, "%s{\"rect\":[%g,%g,%g,%g],\"text\":", first ? "" : ",", r.x0, r.y0, r.x1, r.y1);
				ely_json_string(ctx, out, fz_string_from_buffer(ctx, words));
				fz_write_byte(ctx, out, '}');
				first = 0;
			}
		}
		fz_write_byte(ctx, out, ']');
		fz_close_output(ctx, out);
		*len = fz_buffer_extract(ctx, buf, data);
	}
	fz_always(ctx)
	{
		fz_drop_output(ctx, out);
		fz_drop_buffer(ctx, buf);
		fz_drop_buffer(ctx, words);
		fz_drop_device(ctx, ocr);
		fz_drop_device(ctx, sdev);
		fz_drop_stext_page(ctx, text);
		fz_drop_image(ctx, image);
		fz_drop_pixmap(ctx, pix);
		fz_drop_page(ctx, page);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* A font with no glyphs; codes are UTF-16 units. */
static pdf_obj *glyphless(fz_context *ctx, pdf_document *pdf)
{
	pdf_obj *font = NULL, *cid = NULL, *desc = NULL, *info, *bbox, *kids;
	fz_buffer *cmap = NULL;
	fz_var(font);
	fz_var(cid);
	fz_var(desc);
	fz_var(cmap);
	fz_try(ctx)
	{
		desc = pdf_add_new_dict(ctx, pdf, 8);
		pdf_dict_put(ctx, desc, PDF_NAME(Type), PDF_NAME(FontDescriptor));
		pdf_dict_put_name(ctx, desc, PDF_NAME(FontName), "GlyphLessFont");
		pdf_dict_put_int(ctx, desc, PDF_NAME(Flags), 5);
		bbox = pdf_dict_put_array(ctx, desc, PDF_NAME(FontBBox), 4);
		pdf_array_push_int(ctx, bbox, 0), pdf_array_push_int(ctx, bbox, 0);
		pdf_array_push_int(ctx, bbox, 500), pdf_array_push_int(ctx, bbox, 1000);
		pdf_dict_put_int(ctx, desc, PDF_NAME(ItalicAngle), 0);
		pdf_dict_put_int(ctx, desc, PDF_NAME(Ascent), 1000);
		pdf_dict_put_int(ctx, desc, PDF_NAME(Descent), 0);
		pdf_dict_put_int(ctx, desc, PDF_NAME(CapHeight), 1000);
		pdf_dict_put_int(ctx, desc, PDF_NAME(StemV), 80);
		cid = pdf_add_new_dict(ctx, pdf, 8);
		pdf_dict_put(ctx, cid, PDF_NAME(Type), PDF_NAME(Font));
		pdf_dict_put(ctx, cid, PDF_NAME(Subtype), PDF_NAME(CIDFontType2));
		pdf_dict_put_name(ctx, cid, PDF_NAME(BaseFont), "GlyphLessFont");
		info = pdf_dict_put_dict(ctx, cid, PDF_NAME(CIDSystemInfo), 3);
		pdf_dict_put_text_string(ctx, info, PDF_NAME(Registry), "Adobe");
		pdf_dict_put_text_string(ctx, info, PDF_NAME(Ordering), "Identity");
		pdf_dict_put_int(ctx, info, PDF_NAME(Supplement), 0);
		pdf_dict_put(ctx, cid, PDF_NAME(FontDescriptor), desc);
		pdf_dict_put_int(ctx, cid, PDF_NAME(DW), 500);
		pdf_dict_put(ctx, cid, PDF_NAME(CIDToGIDMap), PDF_NAME(Identity));
		font = pdf_add_new_dict(ctx, pdf, 6);
		pdf_dict_put(ctx, font, PDF_NAME(Type), PDF_NAME(Font));
		pdf_dict_put(ctx, font, PDF_NAME(Subtype), PDF_NAME(Type0));
		pdf_dict_put_name(ctx, font, PDF_NAME(BaseFont), "GlyphLessFont");
		pdf_dict_put(ctx, font, PDF_NAME(Encoding), PDF_NAME(Identity_H));
		kids = pdf_dict_put_array(ctx, font, PDF_NAME(DescendantFonts), 1);
		pdf_array_push(ctx, kids, cid);
		cmap = fz_new_buffer_from_copied_data(ctx, (const unsigned char *)to_unicode, strlen(to_unicode));
		pdf_dict_put_drop(ctx, font, PDF_NAME(ToUnicode), pdf_add_stream(ctx, pdf, cmap, NULL, 0));
	}
	fz_always(ctx)
	{
		pdf_drop_obj(ctx, desc);
		pdf_drop_obj(ctx, cid);
		fz_drop_buffer(ctx, cmap);
	}
	fz_catch(ctx)
	{
		pdf_drop_obj(ctx, font);
		fz_rethrow(ctx);
	}
	return font;
}

/* Drops the page's earlier Ely OCR layers. */
static void drop_layers(fz_context *ctx, pdf_obj *page)
{
	pdf_obj *contents = pdf_dict_get(ctx, page, PDF_NAME(Contents));
	int i;
	for (i = pdf_array_len(ctx, contents) - 1; i >= 0; i--)
		if (pdf_dict_gets(ctx, pdf_array_get(ctx, contents, i), "ElyOCR"))
			pdf_array_delete(ctx, contents, i);
}

/* Throws unless every line is whole and in the BMP. */
static void check_layer(fz_context *ctx, fz_json *pages, int count)
{
	int i, k;
	for (i = 0; i < fz_json_array_length(ctx, pages); i++)
	{
		fz_json *spec = fz_json_array_get(ctx, pages, i);
		fz_json *page = ely_member(ctx, spec, "page", FZ_JSON_NUMBER);
		fz_json *lines = ely_member(ctx, spec, "lines", FZ_JSON_ARRAY);
		if (!page || page->u.number < 0 || page->u.number >= count || !lines)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a layer with no page or lines");
		for (k = 0; k < fz_json_array_length(ctx, lines); k++)
		{
			fz_json *line = fz_json_array_get(ctx, lines, k);
			fz_json *words = ely_member(ctx, line, "text", FZ_JSON_STRING);
			const char *s;
			int c;
			ely_rect(ctx, ely_member(ctx, line, "rect", FZ_JSON_ARRAY), 0.5f);
			if (!words || !words->u.string[0])
				fz_throw(ctx, FZ_ERROR_ARGUMENT, "a line with no text");
			for (s = words->u.string; *s; )
			{
				s += fz_chartorune(&c, s);
				if (c > 0xFFFF || c == FZ_REPLACEMENT_CHARACTER)
					fz_throw(ctx, FZ_ERROR_ARGUMENT, "a character the layer cannot hold: U+%X", c);
			}
		}
	}
}

/* One line: hidden text stretched over its box. */
static void write_line(fz_context *ctx, fz_buffer *buf, fz_json *line, fz_matrix to_user, const char *font)
{
	fz_rect r = ely_rect(ctx, ely_member(ctx, line, "rect", FZ_JSON_ARRAY), 0.5f);
	const char *s = ely_member(ctx, line, "text", FZ_JSON_STRING)->u.string, *p;
	float size = r.y1 - r.y0;
	int n = 0, c;
	fz_matrix m;
	for (p = s; *p; n++)
		p += fz_chartorune(&c, p);
	/* Each code is half an em wide. */
	m = fz_concat(fz_make_matrix((r.x1 - r.x0) / (0.5f * n), 0, 0, -size, r.x0, r.y1 - 0.2f * size), to_user);
	fz_append_printf(ctx, buf, "BT 3 Tr /%s 1 Tf %g %g %g %g %g %g Tm <", font, m.a, m.b, m.c, m.d, m.e, m.f);
	for (p = s; *p; )
	{
		p += fz_chartorune(&c, p);
		fz_append_printf(ctx, buf, "%04x", c);
	}
	fz_append_string(ctx, buf, "> Tj ET\n");
}

/* Hidden text on pages, earlier layers first dropped if asked. */
int ely_text_layer(fz_context *ctx, fz_document *doc, const char *json, int replace)
{
	pdf_document *pdf = NULL;
	pdf_page *page = NULL;
	pdf_obj *font = NULL;
	fz_buffer *buf = NULL;
	fz_pool *pool = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(page);
	fz_var(font);
	fz_var(buf);
	fz_var(pool);
	fz_var(begun);
	fz_try(ctx)
	{
		fz_json *pages;
		int i, k;
		pdf = ely_pdf_of(ctx, doc);
		pool = fz_new_pool(ctx);
		pages = fz_parse_json(ctx, pool, json);
		if (pages->type != FZ_JSON_ARRAY || fz_json_array_length(ctx, pages) == 0)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "no pages to lay text on");
		check_layer(ctx, pages, pdf_count_pages(ctx, pdf));
		pdf_begin_operation(ctx, pdf, "Add recognised text");
		begun = 1;
		for (i = 0; i < fz_json_array_length(ctx, pages); i++)
		{
			fz_json *spec = fz_json_array_get(ctx, pages, i);
			fz_json *lines = ely_member(ctx, spec, "lines", FZ_JSON_ARRAY);
			char name[32];
			page = pdf_load_page(ctx, pdf, (int)ely_member(ctx, spec, "page", FZ_JSON_NUMBER)->u.number);
			if (replace)
				drop_layers(ctx, page->obj);
			if (fz_json_array_length(ctx, lines) > 0)
			{
				if (!font)
					font = glyphless(ctx, pdf);
				ely_add_resource(ctx, page->obj, PDF_NAME(Font), "ElyOCR", font, name, sizeof name);
				buf = fz_new_buffer(ctx, 1024);
				/* Its own state, so later text stays seen. */
				fz_append_string(ctx, buf, "q\n");
				for (k = 0; k < fz_json_array_length(ctx, lines); k++)
					write_line(ctx, buf, fz_json_array_get(ctx, lines, k), ely_to_user(ctx, page), name);
				fz_append_string(ctx, buf, "Q\n");
				pdf_dict_puts(ctx, ely_append_content(ctx, pdf, page->obj, buf), "ElyOCR", PDF_TRUE);
				fz_drop_buffer(ctx, buf);
				buf = NULL;
			}
			fz_drop_page(ctx, (fz_page *)page);
			page = NULL;
		}
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, buf);
		pdf_drop_obj(ctx, font);
		fz_drop_page(ctx, (fz_page *)page);
		fz_drop_pool(ctx, pool);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}
