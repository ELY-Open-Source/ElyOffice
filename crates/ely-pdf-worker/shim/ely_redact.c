/* True removal into a new file, then checked (PDF-09, QA-14). */

#include <math.h>
#include <string.h>

#include "ely_shim.h"

/* Regions by page, page space. */
typedef struct
{
	int page;
	fz_rect rect;
} region;

static int touches(fz_rect a, fz_rect b)
{
	return !fz_is_empty_rect(fz_intersect_rect(a, b));
}

/* Throws unless every region is whole and on a page. */
static region *regions_of(fz_context *ctx, fz_json *list, int pages, int *n)
{
	region *out;
	int i;
	*n = list && list->type == FZ_JSON_ARRAY ? fz_json_array_length(ctx, list) : 0;
	if (*n == 0)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "nothing marked to remove");
	out = fz_malloc_array(ctx, *n, region);
	fz_try(ctx)
	{
		for (i = 0; i < *n; i++)
		{
			fz_json *spec = fz_json_array_get(ctx, list, i);
			fz_json *page = ely_member(ctx, spec, "page", FZ_JSON_NUMBER);
			if (!page || page->u.number < 0 || page->u.number >= pages)
				fz_throw(ctx, FZ_ERROR_ARGUMENT, "a region off the pages");
			out[i].page = (int)page->u.number;
			out[i].rect = ely_rect(ctx, ely_member(ctx, spec, "rect", FZ_JSON_ARRAY), 1);
		}
	}
	fz_catch(ctx)
	{
		fz_free(ctx, out);
		fz_rethrow(ctx);
	}
	return out;
}

/* Drops annotations and fields that touch `area`; they carry text. */
static void drop_touching(fz_context *ctx, pdf_page *page, fz_rect area)
{
	pdf_annot *annot;
	int widgets;
	for (widgets = 0; widgets < 2; widgets++)
	{
restart:
		for (annot = widgets ? pdf_first_widget(ctx, page) : pdf_first_annot(ctx, page); annot;
			annot = widgets ? pdf_next_widget(ctx, annot) : pdf_next_annot(ctx, annot))
			if (touches(pdf_bound_annot(ctx, annot), area))
			{
				pdf_obj *field = pdf_keep_obj(ctx, pdf_annot_obj(ctx, annot));
				fz_var(annot);
				fz_var(widgets);
				fz_try(ctx)
				{
					pdf_delete_annot(ctx, page, annot);
					/* A field left with no widget still holds its value. */
					if (widgets)
						ely_unlink_field(ctx, page->doc, field);
				}
				fz_always(ctx)
					pdf_drop_obj(ctx, field);
				fz_catch(ctx)
					fz_rethrow(ctx);
				goto restart;
			}
	}
}

/* Applies `regions` to the copy, page by page. */
static void redact(fz_context *ctx, pdf_document *pdf, region *regions, int n)
{
	pdf_redact_options opts = { 1, PDF_REDACT_IMAGE_PIXELS, PDF_REDACT_LINE_ART_REMOVE_IF_TOUCHED, PDF_REDACT_TEXT_REMOVE };
	pdf_page *page = NULL;
	int i, k;
	fz_var(page);
	fz_try(ctx)
	{
		for (i = 0; i < pdf_count_pages(ctx, pdf); i++)
		{
			int any = 0;
			for (k = 0; k < n; k++)
				any |= regions[k].page == i;
			if (!any)
				continue;
			page = pdf_load_page(ctx, pdf, i);
			for (k = 0; k < n; k++)
				if (regions[k].page == i)
					drop_touching(ctx, page, regions[k].rect);
			/* Marks after drops, or a drop takes a mark. */
			for (k = 0; k < n; k++)
				if (regions[k].page == i)
				{
					pdf_annot *mark = pdf_create_annot(ctx, page, PDF_ANNOT_REDACT);
					pdf_set_annot_rect(ctx, mark, regions[k].rect);
					pdf_drop_annot(ctx, mark);
				}
			pdf_redact_page(ctx, pdf, page, &opts);
			ely_drop_alt(ctx, pdf, page);
			pdf_dict_del(ctx, page->obj, PDF_NAME(Thumb));
			fz_drop_page(ctx, (fz_page *)page);
			page = NULL;
		}
	}
	fz_always(ctx)
		fz_drop_page(ctx, (fz_page *)page);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* A glyph box shrunk a tenth, as redaction judges. */
static fz_rect shrunk(fz_rect r)
{
	float w = (r.x1 - r.x0) / 10, h = (r.y1 - r.y0) / 10;
	return fz_make_rect(r.x0 + w, r.y0 + h, r.x1 - w, r.y1 - h);
}

/* Characters on `page` inside `area`, hidden ones too. */
static int chars_in(fz_context *ctx, fz_document *doc, int number, fz_rect area)
{
	fz_stext_page *text = NULL;
	int n = 0;
	fz_var(text);
	fz_var(n);
	fz_try(ctx)
	{
		fz_stext_block *block;
		fz_stext_line *line;
		fz_stext_char *ch;
		text = fz_new_stext_page_from_page_number(ctx, doc, number, NULL);
		for (block = text->first_block; block; block = block->next)
			if (block->type == FZ_STEXT_BLOCK_TEXT)
				for (line = block->u.t.first_line; line; line = line->next)
					for (ch = line->first_char; ch; ch = ch->next)
						n += touches(shrunk(fz_rect_from_quad(ch->quad)), area);
	}
	fz_always(ctx)
		fz_drop_stext_page(ctx, text);
	fz_catch(ctx)
		fz_rethrow(ctx);
	return n;
}

/* Pixels in `area` that are not black once drawn. */
static int light_in(fz_context *ctx, fz_document *doc, int number, fz_rect area)
{
	fz_pixmap *pix = NULL;
	int n = 0;
	fz_var(pix);
	fz_var(n);
	fz_try(ctx)
	{
		fz_page *page = fz_load_page(ctx, doc, number);
		fz_irect box = fz_round_rect(fz_intersect_rect(area, fz_bound_page(ctx, page)));
		unsigned char *s;
		int x, y;
		fz_drop_page(ctx, page);
		/* One pixel in from each edge: boxes antialias. */
		box.x0++, box.y0++, box.x1--, box.y1--;
		if (box.x1 <= box.x0 || box.y1 <= box.y0)
			break;
		pix = fz_new_pixmap_with_bbox(ctx, fz_device_rgb(ctx), box, NULL, 0);
		fz_clear_pixmap_with_value(ctx, pix, 255);
		page = fz_load_page(ctx, doc, number);
		fz_try(ctx)
		{
			fz_device *dev = fz_new_draw_device(ctx, fz_identity, pix);
			fz_try(ctx)
			{
				fz_run_page(ctx, page, dev, fz_identity, NULL);
				fz_close_device(ctx, dev);
			}
			fz_always(ctx)
				fz_drop_device(ctx, dev);
			fz_catch(ctx)
				fz_rethrow(ctx);
		}
		fz_always(ctx)
			fz_drop_page(ctx, page);
		fz_catch(ctx)
			fz_rethrow(ctx);
		for (y = 0, s = pix->samples; y < pix->h; y++, s += pix->stride - pix->w * 3)
			for (x = 0; x < pix->w; x++, s += 3)
				n += s[0] > 40 || s[1] > 40 || s[2] > 40;
	}
	fz_always(ctx)
		fz_drop_pixmap(ctx, pix);
	fz_catch(ctx)
		fz_rethrow(ctx);
	return n;
}

/* Counts drawings in the regions, our boxes aside. */
typedef struct
{
	fz_device super;
	region *regions;
	int n, page, leaks;
} leak_dev;

static int in_regions(leak_dev *d, fz_rect r)
{
	int k;
	for (k = 0; k < d->n; k++)
		if (d->regions[k].page == d->page && touches(r, d->regions[k].rect))
			return 1;
	return 0;
}

static int black(fz_context *ctx, fz_colorspace *cs, const float *color)
{
	float rgb[3];
	if (!cs)
		return 0;
	fz_convert_color(ctx, cs, color, fz_device_rgb(ctx), rgb, NULL, fz_default_color_params);
	return rgb[0] <= 0.02f && rgb[1] <= 0.02f && rgb[2] <= 0.02f;
}

/* Whether `r`, black, is one of the region boxes. */
static int is_box(fz_context *ctx, leak_dev *d, fz_rect r, fz_colorspace *cs, const float *color)
{
	int k;
	if (!black(ctx, cs, color))
		return 0;
	for (k = 0; k < d->n; k++)
	{
		fz_rect b = d->regions[k].rect;
		if (d->regions[k].page == d->page && fabsf(r.x0 - b.x0) < 1 && fabsf(r.y0 - b.y0) < 1 &&
			fabsf(r.x1 - b.x1) < 1 && fabsf(r.y1 - b.y1) < 1)
			return 1;
	}
	return 0;
}

static void leak_fill_path(fz_context *ctx, fz_device *dev, const fz_path *path, int even_odd, fz_matrix ctm,
	fz_colorspace *cs, const float *color, float alpha, fz_color_params cp)
{
	leak_dev *d = (leak_dev *)dev;
	fz_rect r = fz_bound_path(ctx, path, NULL, ctm);
	(void)even_odd, (void)alpha, (void)cp;
	if (in_regions(d, r) && !is_box(ctx, d, r, cs, color))
		d->leaks++;
}

static void leak_stroke_path(fz_context *ctx, fz_device *dev, const fz_path *path, const fz_stroke_state *stroke,
	fz_matrix ctm, fz_colorspace *cs, const float *color, float alpha, fz_color_params cp)
{
	leak_dev *d = (leak_dev *)dev;
	(void)cs, (void)color, (void)alpha, (void)cp;
	d->leaks += in_regions(d, fz_bound_path(ctx, path, stroke, ctm));
}

static void leak_fill_shade(fz_context *ctx, fz_device *dev, fz_shade *shade, fz_matrix ctm, float alpha, fz_color_params cp)
{
	leak_dev *d = (leak_dev *)dev;
	(void)alpha, (void)cp;
	d->leaks += in_regions(d, fz_bound_shade(ctx, shade, ctm));
}

static void leak_fill_image_mask(fz_context *ctx, fz_device *dev, fz_image *image, fz_matrix ctm, fz_colorspace *cs,
	const float *color, float alpha, fz_color_params cp)
{
	leak_dev *d = (leak_dev *)dev;
	(void)image, (void)alpha, (void)cp;
	d->leaks += in_regions(d, fz_transform_rect(fz_unit_rect, ctm)) && !black(ctx, cs, color);
}

/* Image pixels in `area` neither white nor black. */
static int image_light(fz_context *ctx, fz_image *image, fz_matrix ctm, fz_rect area)
{
	fz_irect box = fz_round_rect(fz_intersect_rect(area, fz_transform_rect(fz_unit_rect, ctm)));
	fz_pixmap *pix = NULL;
	fz_device *draw = NULL;
	int n = 0, x, y;
	box.x0++, box.y0++, box.x1--, box.y1--;
	if (box.x1 <= box.x0 || box.y1 <= box.y0)
		return 0;
	fz_var(pix);
	fz_var(draw);
	fz_var(box);
	fz_try(ctx)
	{
		unsigned char *s;
		pix = fz_new_pixmap_with_bbox(ctx, fz_device_rgb(ctx), box, NULL, 1);
		fz_clear_pixmap(ctx, pix);
		draw = fz_new_draw_device(ctx, fz_identity, pix);
		fz_fill_image(ctx, draw, image, ctm, 1, fz_default_color_params);
		fz_close_device(ctx, draw);
		for (y = 0, s = pix->samples; y < pix->h; y++, s += pix->stride - pix->w * 4)
			for (x = 0; x < pix->w; x++, s += 4)
			{
				int high = s[0] > 215 && s[1] > 215 && s[2] > 215, low = s[0] < 40 && s[1] < 40 && s[2] < 40;
				n += s[3] > 200 && !high && !low;
			}
	}
	fz_always(ctx)
	{
		fz_drop_device(ctx, draw);
		fz_drop_pixmap(ctx, pix);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
	return n;
}

static void leak_fill_image(fz_context *ctx, fz_device *dev, fz_image *image, fz_matrix ctm, float alpha, fz_color_params cp)
{
	leak_dev *d = (leak_dev *)dev;
	int k;
	(void)alpha, (void)cp;
	for (k = 0; k < d->n; k++)
		if (d->regions[k].page == d->page)
			d->leaks += image_light(ctx, image, ctm, d->regions[k].rect);
}

/* Drawings left in the regions of page `number`. */
static int leaks_on(fz_context *ctx, fz_document *doc, int number, region *regions, int n)
{
	leak_dev *d = fz_new_derived_device(ctx, leak_dev);
	fz_page *page = NULL;
	int leaks = 0;
	fz_var(page);
	d->super.fill_path = leak_fill_path;
	d->super.stroke_path = leak_stroke_path;
	d->super.fill_shade = leak_fill_shade;
	d->super.fill_image = leak_fill_image;
	d->super.fill_image_mask = leak_fill_image_mask;
	d->regions = regions, d->n = n, d->page = number;
	fz_try(ctx)
	{
		page = fz_load_page(ctx, doc, number);
		fz_run_page(ctx, page, &d->super, fz_identity, NULL);
		fz_close_device(ctx, &d->super);
		leaks = d->leaks;
	}
	fz_always(ctx)
	{
		fz_drop_page(ctx, page);
		fz_drop_device(ctx, &d->super);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
	return leaks;
}

typedef struct
{
	region *regions;
	int n;
} marked;

static void apply_marks(fz_context *ctx, fz_document *copy, void *arg)
{
	marked *m = arg;
	redact(ctx, ely_pdf_of(ctx, copy), m->regions, m->n);
}

/* Checks `out`: no text, no light in any region. */
static void check_redacted(fz_context *ctx, const char *out, const char *password, region *regions, int n, fz_buffer *report)
{
	fz_document *doc = fz_open_document(ctx, out);
	fz_try(ctx)
	{
		int i, text = 0, light = 0, objects = 0, versions;
		if (fz_needs_password(ctx, doc) && !fz_authenticate_password(ctx, doc, password))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "the written file refused its password");
		for (i = 0; i < n; i++)
		{
			text += chars_in(ctx, doc, regions[i].page, regions[i].rect);
			light += light_in(ctx, doc, regions[i].page, regions[i].rect);
			objects += pdf_dict_get(ctx, pdf_lookup_page_obj(ctx, ely_pdf_of(ctx, doc), regions[i].page), PDF_NAME(Thumb)) != NULL;
		}
		for (i = 0; i < fz_count_pages(ctx, doc); i++)
			objects += leaks_on(ctx, doc, i, regions, n);
		versions = pdf_count_versions(ctx, ely_pdf_of(ctx, doc));
		fz_append_printf(ctx, report, "{\"regions\":%d,\"text\":%d,\"light\":%d,\"objects\":%d,\"versions\":%d}", n, text,
			light, objects, versions);
		if (text || light || objects || versions != 1)
			fz_throw(ctx, FZ_ERROR_FORMAT, "the written file still holds what was removed: %d characters, %d light pixels, %d drawings, %d versions",
				text, light, objects, versions);
	}
	fz_always(ctx)
		fz_drop_document(ctx, doc);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* Removes the marked regions into `out`, then checks it. */
int ely_redact(fz_context *ctx, fz_document *doc, const char *json, const char *raw, const char *out,
	const char *password, unsigned char **data, size_t *len)
{
	fz_pool *pool = NULL;
	marked m = { NULL, 0 };
	fz_buffer *report = NULL;
	fz_var(pool);
	fz_var(m);
	fz_var(report);
	fz_try(ctx)
	{
		pdf_document *pdf = ely_pdf_of(ctx, doc);
		pool = fz_new_pool(ctx);
		m.regions = regions_of(ctx, fz_parse_json(ctx, pool, json), pdf_count_pages(ctx, pdf), &m.n);
		ely_save_copy(ctx, pdf, raw, out, password, NULL, apply_marks, &m);
		report = fz_new_buffer(ctx, 128);
		check_redacted(ctx, out, password, m.regions, m.n, report);
		*len = fz_buffer_extract(ctx, report, data);
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, report);
		fz_free(ctx, m.regions);
		fz_drop_pool(ctx, pool);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

