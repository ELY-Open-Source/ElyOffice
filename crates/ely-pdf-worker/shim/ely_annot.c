/* Annotations: list, add, change, delete (PDF-05). */

#include <math.h>
#include <string.h>

#include "ely_shim.h"

static const char *stamps[] = { "Approved", "AsIs", "Confidential", "Departmental", "Draft", "Experimental",
	"Expired", "Final", "ForComment", "ForPublicRelease", "NotApproved", "NotForPublicRelease", "Sold",
	"TopSecret", NULL };

/* Valid UTF-8 always: a stray byte becomes U+FFFD. */
void ely_json_string(fz_context *ctx, fz_output *out, const char *s)
{
	char utf8[FZ_UTFMAX];
	int rune;
	fz_write_byte(ctx, out, '"');
	while (*s)
	{
		s += fz_chartorune(&rune, s);
		/* A surrogate pair joins; a lone one is no character. */
		if (rune >= 0xD800 && rune < 0xDC00)
		{
			int low;
			int n = fz_chartorune(&low, s);
			if (low >= 0xDC00 && low < 0xE000)
			{
				rune = 0x10000 + ((rune - 0xD800) << 10) + (low - 0xDC00);
				s += n;
			}
		}
		if ((rune >= 0xD800 && rune < 0xE000) || rune > 0x10FFFF)
			rune = FZ_REPLACEMENT_CHARACTER;
		if (rune == '"' || rune == '\\')
			fz_write_printf(ctx, out, "\\%c", rune);
		else if (rune < 32)
			fz_write_printf(ctx, out, "\\u%04x", rune);
		else
			fz_write_data(ctx, out, utf8, fz_runetochar(utf8, rune));
	}
	fz_write_byte(ctx, out, '"');
}

fz_json *ely_member(fz_context *ctx, fz_json *obj, const char *key, int type)
{
	fz_json *value = fz_json_object_get(ctx, obj, key);
	if (value && value->type != type)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a bad %s", key);
	return value;
}

float ely_number(fz_context *ctx, fz_json *array, int at)
{
	fz_json *value = fz_json_array_get(ctx, array, at);
	if (!value || value->type != FZ_JSON_NUMBER)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a bad number list");
	return (float)value->u.number;
}

/* Four numbers, at least `least` wide and high. */
fz_rect ely_rect(fz_context *ctx, fz_json *array, float least)
{
	fz_rect r;
	if (!array || array->type != FZ_JSON_ARRAY || fz_json_array_length(ctx, array) != 4)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a bad rectangle");
	r = fz_make_rect(ely_number(ctx, array, 0), ely_number(ctx, array, 1), ely_number(ctx, array, 2),
		ely_number(ctx, array, 3));
	if (r.x1 - r.x0 < least || r.y1 - r.y0 < least)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a rectangle too small");
	return r;
}

static enum pdf_annot_type kind_of(fz_context *ctx, const char *kind)
{
	if (!strcmp(kind, "highlight")) return PDF_ANNOT_HIGHLIGHT;
	if (!strcmp(kind, "underline")) return PDF_ANNOT_UNDERLINE;
	if (!strcmp(kind, "strikeout")) return PDF_ANNOT_STRIKE_OUT;
	if (!strcmp(kind, "note")) return PDF_ANNOT_TEXT;
	if (!strcmp(kind, "square")) return PDF_ANNOT_SQUARE;
	if (!strcmp(kind, "circle")) return PDF_ANNOT_CIRCLE;
	if (!strcmp(kind, "line")) return PDF_ANNOT_LINE;
	if (!strcmp(kind, "ink")) return PDF_ANNOT_INK;
	if (!strcmp(kind, "stamp")) return PDF_ANNOT_STAMP;
	fz_throw(ctx, FZ_ERROR_ARGUMENT, "no annotation kind %s", kind);
}

static int rgb_of(fz_context *ctx, pdf_annot *annot)
{
	float c[4], k;
	int n;
	pdf_annot_color(ctx, annot, &n, c);
	if (n == 1)
		c[1] = c[2] = c[0];
	else if (n == 4)
	{
		k = 1 - c[3];
		c[0] = (1 - c[0]) * k, c[1] = (1 - c[1]) * k, c[2] = (1 - c[2]) * k;
	}
	else if (n != 3)
		return -1;
	return ((int)(c[0] * 255 + 0.5f) << 16) | ((int)(c[1] * 255 + 0.5f) << 8) | (int)(c[2] * 255 + 0.5f);
}

static int color_of(fz_context *ctx, fz_json *spec)
{
	fz_json *color = ely_member(ctx, spec, "color", FZ_JSON_NUMBER);
	if (!color)
		return -1;
	if (color->u.number < 0 || color->u.number > 0xffffff)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a bad color");
	return (int)color->u.number;
}

static const char *text_of(fz_context *ctx, fz_json *spec, const char *key)
{
	fz_json *text = ely_member(ctx, spec, key, FZ_JSON_STRING);
	return text ? text->u.string : NULL;
}

static int is_stamp(const char *name)
{
	int i;
	for (i = 0; name && stamps[i]; i++)
		if (!strcmp(stamps[i], name))
			return 1;
	return 0;
}

/* Throws unless `spec` makes a whole annotation. */
static enum pdf_annot_type check(fz_context *ctx, fz_json *spec, int pages)
{
	fz_json *page = ely_member(ctx, spec, "page", FZ_JSON_NUMBER);
	fz_json *kind = ely_member(ctx, spec, "kind", FZ_JSON_STRING);
	fz_json *list;
	enum pdf_annot_type type;
	int i, n;
	if (!page || page->u.number < 0 || page->u.number >= pages || !kind)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "an annotation with no page or kind");
	type = kind_of(ctx, kind->u.string);
	color_of(ctx, spec);
	text_of(ctx, spec, "contents");
	text_of(ctx, spec, "author");
	switch (type)
	{
	case PDF_ANNOT_HIGHLIGHT:
	case PDF_ANNOT_UNDERLINE:
	case PDF_ANNOT_STRIKE_OUT:
		list = ely_member(ctx, spec, "rects", FZ_JSON_ARRAY);
		n = list ? fz_json_array_length(ctx, list) : 0;
		if (n == 0)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a mark over nothing");
		for (i = 0; i < n; i++)
			ely_rect(ctx, fz_json_array_get(ctx, list, i), 0.1f);
		break;
	case PDF_ANNOT_TEXT:
		list = ely_member(ctx, spec, "at", FZ_JSON_ARRAY);
		if (!list || fz_json_array_length(ctx, list) != 2)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a note with no place");
		ely_number(ctx, list, 0), ely_number(ctx, list, 1);
		break;
	case PDF_ANNOT_LINE:
		list = ely_member(ctx, spec, "line", FZ_JSON_ARRAY);
		if (!list || fz_json_array_length(ctx, list) != 4)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a line with no ends");
		if (fabsf(ely_number(ctx, list, 2) - ely_number(ctx, list, 0)) +
			fabsf(ely_number(ctx, list, 3) - ely_number(ctx, list, 1)) < 2)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a line too short");
		break;
	case PDF_ANNOT_INK:
		list = ely_member(ctx, spec, "strokes", FZ_JSON_ARRAY);
		n = list ? fz_json_array_length(ctx, list) : 0;
		if (n == 0)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "ink with no strokes");
		for (i = 0; i < n; i++)
		{
			fz_json *stroke = fz_json_array_get(ctx, list, i);
			int k, m = stroke->type == FZ_JSON_ARRAY ? fz_json_array_length(ctx, stroke) : 0;
			if (m < 4 || m % 2)
				fz_throw(ctx, FZ_ERROR_ARGUMENT, "a bad stroke");
			for (k = 0; k < m; k++)
				ely_number(ctx, stroke, k);
		}
		break;
	case PDF_ANNOT_STAMP:
		if (!is_stamp(text_of(ctx, spec, "icon")))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "no such stamp");
		ely_rect(ctx, ely_member(ctx, spec, "rect", FZ_JSON_ARRAY), 2);
		break;
	default:
		ely_rect(ctx, ely_member(ctx, spec, "rect", FZ_JSON_ARRAY), 2);
	}
	return type;
}

static fz_point point_at(fz_context *ctx, fz_json *list, int at)
{
	return fz_make_point(ely_number(ctx, list, at), ely_number(ctx, list, at + 1));
}

/* Makes a checked `spec` inside the open step. */
static void make(fz_context *ctx, pdf_document *pdf, fz_json *spec, enum pdf_annot_type type)
{
	pdf_page *page = NULL;
	pdf_annot *annot = NULL;
	fz_point *points = NULL;
	fz_var(page);
	fz_var(annot);
	fz_var(points);
	fz_try(ctx)
	{
		fz_json *list;
		int i, k, n, color = color_of(ctx, spec);
		page = pdf_load_page(ctx, pdf, (int)ely_member(ctx, spec, "page", FZ_JSON_NUMBER)->u.number);
		annot = pdf_create_annot(ctx, page, type);
		switch (type)
		{
		case PDF_ANNOT_HIGHLIGHT:
		case PDF_ANNOT_UNDERLINE:
		case PDF_ANNOT_STRIKE_OUT:
			list = ely_member(ctx, spec, "rects", FZ_JSON_ARRAY);
			n = fz_json_array_length(ctx, list);
			for (i = 0; i < n; i++)
				pdf_add_annot_quad_point(ctx, annot, fz_quad_from_rect(ely_rect(ctx, fz_json_array_get(ctx, list, i), 0.1f)));
			break;
		case PDF_ANNOT_TEXT:
		{
			fz_point at = point_at(ctx, ely_member(ctx, spec, "at", FZ_JSON_ARRAY), 0);
			pdf_set_annot_rect(ctx, annot, fz_make_rect(at.x, at.y, at.x + 16, at.y + 16));
			pdf_set_annot_popup(ctx, annot, fz_make_rect(at.x + 20, at.y, at.x + 220, at.y + 100));
			break;
		}
		case PDF_ANNOT_LINE:
			list = ely_member(ctx, spec, "line", FZ_JSON_ARRAY);
			pdf_set_annot_line(ctx, annot, point_at(ctx, list, 0), point_at(ctx, list, 2));
			break;
		case PDF_ANNOT_INK:
			list = ely_member(ctx, spec, "strokes", FZ_JSON_ARRAY);
			n = fz_json_array_length(ctx, list);
			for (i = 0; i < n; i++)
			{
				fz_json *stroke = fz_json_array_get(ctx, list, i);
				int m = fz_json_array_length(ctx, stroke) / 2;
				fz_free(ctx, points);
				points = NULL;
				points = fz_malloc_array(ctx, m, fz_point);
				for (k = 0; k < m; k++)
					points[k] = point_at(ctx, stroke, k * 2);
				pdf_add_annot_ink_list(ctx, annot, m, points);
			}
			break;
		case PDF_ANNOT_STAMP:
			pdf_set_annot_icon_name(ctx, annot, text_of(ctx, spec, "icon"));
			pdf_set_annot_rect(ctx, annot, ely_rect(ctx, ely_member(ctx, spec, "rect", FZ_JSON_ARRAY), 2));
			break;
		default:
			pdf_set_annot_rect(ctx, annot, ely_rect(ctx, ely_member(ctx, spec, "rect", FZ_JSON_ARRAY), 2));
		}
		if (color >= 0)
		{
			float rgb[3] = { (color >> 16) / 255.0f, ((color >> 8) & 255) / 255.0f, (color & 255) / 255.0f };
			pdf_set_annot_color(ctx, annot, 3, rgb);
		}
		if (text_of(ctx, spec, "contents"))
			pdf_set_annot_contents(ctx, annot, text_of(ctx, spec, "contents"));
		if (text_of(ctx, spec, "author"))
			pdf_set_annot_author(ctx, annot, text_of(ctx, spec, "author"));
		pdf_update_annot(ctx, annot);
	}
	fz_always(ctx)
	{
		fz_free(ctx, points);
		pdf_drop_annot(ctx, annot);
		fz_drop_page(ctx, (fz_page *)page);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* A JSON list of annotations, added in one step. */
int ely_annotate(fz_context *ctx, fz_document *doc, const char *json)
{
	pdf_document *pdf = NULL;
	fz_pool *pool = NULL;
	enum pdf_annot_type *types = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(pool);
	fz_var(types);
	fz_var(begun);
	fz_try(ctx)
	{
		fz_json *specs;
		int i, n, pages;
		pdf = ely_pdf_of(ctx, doc);
		pages = pdf_count_pages(ctx, pdf);
		pool = fz_new_pool(ctx);
		specs = fz_parse_json(ctx, pool, json);
		n = specs->type == FZ_JSON_ARRAY ? fz_json_array_length(ctx, specs) : 0;
		if (n == 0)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "no annotations to add");
		types = fz_malloc_array(ctx, n, enum pdf_annot_type);
		for (i = 0; i < n; i++)
			types[i] = check(ctx, fz_json_array_get(ctx, specs, i), pages);
		pdf_begin_operation(ctx, pdf, "Annotate");
		begun = 1;
		for (i = 0; i < n; i++)
			make(ctx, pdf, fz_json_array_get(ctx, specs, i), types[i]);
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		fz_free(ctx, types);
		fz_drop_pool(ctx, pool);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

/* The annotation or widget object `id` on `page`. */
pdf_annot *ely_find_annot(fz_context *ctx, pdf_page *page, int id, int widget)
{
	pdf_annot *annot = widget ? pdf_first_widget(ctx, page) : pdf_first_annot(ctx, page);
	for (; annot; annot = widget ? pdf_next_widget(ctx, annot) : pdf_next_annot(ctx, annot))
		if (pdf_to_num(ctx, pdf_annot_obj(ctx, annot)) == id)
			return annot;
	fz_throw(ctx, FZ_ERROR_ARGUMENT, "no %s %d on the page", widget ? "field" : "annotation", id);
}

/* Sets an annotation's colour or contents, in one step. */
int ely_change_annot(fz_context *ctx, fz_document *doc, int number, int id, const char *json)
{
	pdf_document *pdf = NULL;
	pdf_page *page = NULL;
	fz_pool *pool = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(page);
	fz_var(pool);
	fz_var(begun);
	fz_try(ctx)
	{
		pdf_annot *annot;
		fz_json *spec;
		const char *contents;
		int color;
		pdf = ely_pdf_of(ctx, doc);
		page = pdf_load_page(ctx, pdf, number);
		annot = ely_find_annot(ctx, page, id, 0);
		pool = fz_new_pool(ctx);
		spec = fz_parse_json(ctx, pool, json);
		color = color_of(ctx, spec);
		contents = text_of(ctx, spec, "contents");
		if (color < 0 && !contents)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "no change");
		pdf_begin_operation(ctx, pdf, "Change an annotation");
		begun = 1;
		if (color >= 0)
		{
			float rgb[3] = { (color >> 16) / 255.0f, ((color >> 8) & 255) / 255.0f, (color & 255) / 255.0f };
			pdf_set_annot_color(ctx, annot, 3, rgb);
		}
		if (contents)
			pdf_set_annot_contents(ctx, annot, contents);
		pdf_update_annot(ctx, annot);
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		fz_drop_pool(ctx, pool);
		fz_drop_page(ctx, (fz_page *)page);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

/* Unlinks `field`; an emptied parent goes too. */
void ely_unlink_field(fz_context *ctx, pdf_document *pdf, pdf_obj *field)
{
	pdf_obj *parent = pdf_dict_get(ctx, field, PDF_NAME(Parent));
	pdf_obj *kids = parent ? pdf_dict_get(ctx, parent, PDF_NAME(Kids)) : pdf_dict_getp(ctx, pdf_trailer(ctx, pdf), "Root/AcroForm/Fields");
	int at = pdf_array_find(ctx, kids, field);
	if (at >= 0)
		pdf_array_delete(ctx, kids, at);
	if (parent && pdf_array_len(ctx, kids) == 0)
		ely_unlink_field(ctx, pdf, parent);
}

/* Deletes annotation or field `id`, in one step. */
int ely_delete_annot(fz_context *ctx, fz_document *doc, int number, int id, int widget)
{
	pdf_document *pdf = NULL;
	pdf_page *page = NULL;
	pdf_obj *field = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(page);
	fz_var(field);
	fz_var(begun);
	fz_try(ctx)
	{
		pdf_annot *annot;
		pdf = ely_pdf_of(ctx, doc);
		page = pdf_load_page(ctx, pdf, number);
		annot = ely_find_annot(ctx, page, id, widget);
		field = pdf_keep_obj(ctx, pdf_annot_obj(ctx, annot));
		pdf_begin_operation(ctx, pdf, widget ? "Delete a field" : "Delete an annotation");
		begun = 1;
		pdf_delete_annot(ctx, page, annot);
		if (widget)
			ely_unlink_field(ctx, pdf, field);
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		pdf_drop_obj(ctx, field);
		fz_drop_page(ctx, (fz_page *)page);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

static void write_annot(fz_context *ctx, fz_output *out, pdf_annot *annot)
{
	fz_rect r = pdf_bound_annot(ctx, annot);
	fz_write_printf(ctx, out, "{\"id\":%d,\"kind\":", pdf_to_num(ctx, pdf_annot_obj(ctx, annot)));
	ely_json_string(ctx, out, pdf_string_from_annot_type(ctx, pdf_annot_type(ctx, annot)));
	fz_write_printf(ctx, out, ",\"rect\":[%g,%g,%g,%g],\"color\":%d,\"contents\":", r.x0, r.y0, r.x1, r.y1,
		rgb_of(ctx, annot));
	ely_json_string(ctx, out, pdf_annot_contents(ctx, annot));
	fz_write_string(ctx, out, ",\"author\":");
	ely_json_string(ctx, out, pdf_annot_has_author(ctx, annot) ? pdf_annot_author(ctx, annot) : "");
	fz_write_string(ctx, out, ",\"icon\":");
	ely_json_string(ctx, out, pdf_annot_has_icon_name(ctx, annot) ? pdf_annot_icon_name(ctx, annot) : "");
	fz_write_byte(ctx, out, '}');
}

/* JSON of a page's annotations and form fields. */
int ely_annots(fz_context *ctx, fz_document *doc, int number, unsigned char **data, size_t *len)
{
	pdf_page *page = NULL;
	fz_buffer *buf = NULL;
	fz_output *out = NULL;
	fz_var(page);
	fz_var(buf);
	fz_var(out);
	fz_try(ctx)
	{
		pdf_annot *annot;
		page = pdf_load_page(ctx, ely_pdf_of(ctx, doc), number);
		buf = fz_new_buffer(ctx, 1024);
		out = fz_new_output_with_buffer(ctx, buf);
		fz_write_string(ctx, out, "{\"annots\":[");
		for (annot = pdf_first_annot(ctx, page); annot; annot = pdf_next_annot(ctx, annot))
		{
			if (annot != pdf_first_annot(ctx, page))
				fz_write_byte(ctx, out, ',');
			write_annot(ctx, out, annot);
		}
		fz_write_string(ctx, out, "],\"fields\":[");
		ely_write_fields(ctx, out, page);
		fz_write_string(ctx, out, "]}");
		fz_close_output(ctx, out);
		*len = fz_buffer_extract(ctx, buf, data);
	}
	fz_always(ctx)
	{
		fz_drop_output(ctx, out);
		fz_drop_buffer(ctx, buf);
		fz_drop_page(ctx, (fz_page *)page);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}
