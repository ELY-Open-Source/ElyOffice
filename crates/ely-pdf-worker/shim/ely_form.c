/* Form fields: list, add, fill, change (PDF-06). */

#include <stdio.h>
#include <string.h>

#include "ely_shim.h"

static const char *kind_name(enum pdf_widget_type type)
{
	switch (type)
	{
	case PDF_WIDGET_TYPE_TEXT: return "text";
	case PDF_WIDGET_TYPE_CHECKBOX: return "checkbox";
	case PDF_WIDGET_TYPE_RADIOBUTTON: return "radio";
	case PDF_WIDGET_TYPE_COMBOBOX: return "choice";
	case PDF_WIDGET_TYPE_LISTBOX: return "list";
	case PDF_WIDGET_TYPE_SIGNATURE: return "signature";
	default: return "button";
	}
}

/* The field that names `obj`. */
static pdf_obj *head_of(fz_context *ctx, pdf_obj *obj)
{
	while (!pdf_dict_get(ctx, obj, PDF_NAME(T)) && pdf_dict_get(ctx, obj, PDF_NAME(Parent)))
		obj = pdf_dict_get(ctx, obj, PDF_NAME(Parent));
	return obj;
}

static pdf_obj *fields_of(fz_context *ctx, pdf_document *pdf)
{
	pdf_obj *root = pdf_dict_get(ctx, pdf_trailer(ctx, pdf), PDF_NAME(Root));
	pdf_obj *form = pdf_dict_get(ctx, root, PDF_NAME(AcroForm));
	if (!form)
		form = pdf_dict_put_dict(ctx, root, PDF_NAME(AcroForm), 2);
	if (!pdf_is_array(ctx, pdf_dict_get(ctx, form, PDF_NAME(Fields))))
		pdf_dict_put_array(ctx, form, PDF_NAME(Fields), 4);
	return pdf_dict_get(ctx, form, PDF_NAME(Fields));
}

/* A value as text; names and streams too. */
static void write_value(fz_context *ctx, fz_output *out, pdf_obj *v)
{
	char *text = NULL;
	fz_var(text);
	fz_try(ctx)
	{
		if (pdf_is_name(ctx, v))
			ely_json_string(ctx, out, pdf_to_name(ctx, v));
		else if (pdf_is_stream(ctx, v))
		{
			text = pdf_new_utf8_from_pdf_stream_obj(ctx, v);
			ely_json_string(ctx, out, text);
		}
		else
			ely_json_string(ctx, out, pdf_to_text_string(ctx, v));
	}
	fz_always(ctx)
		fz_free(ctx, text);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

static void write_options(fz_context *ctx, fz_output *out, pdf_obj *obj, int export)
{
	int i, n = pdf_choice_field_option_count(ctx, obj);
	fz_write_byte(ctx, out, '[');
	for (i = 0; i < n; i++)
	{
		if (i)
			fz_write_byte(ctx, out, ',');
		ely_json_string(ctx, out, pdf_choice_field_option(ctx, obj, export, i));
	}
	fz_write_byte(ctx, out, ']');
}

void ely_write_fields(fz_context *ctx, fz_output *out, pdf_page *page)
{
	pdf_annot *widget;
	char *name = NULL;
	fz_var(name);
	fz_try(ctx)
	{
		for (widget = pdf_first_widget(ctx, page); widget; widget = pdf_next_widget(ctx, widget))
		{
			pdf_obj *obj = pdf_annot_obj(ctx, widget);
			enum pdf_widget_type type = pdf_widget_type(ctx, widget);
			fz_rect r = pdf_bound_widget(ctx, widget);
			int button = type == PDF_WIDGET_TYPE_CHECKBOX || type == PDF_WIDGET_TYPE_RADIOBUTTON;
			pdf_obj *as = pdf_dict_get(ctx, obj, PDF_NAME(AS));
			fz_free(ctx, name);
			name = NULL;
			name = pdf_load_field_name(ctx, obj);
			fz_write_printf(ctx, out, "%s{\"id\":%d,\"kind\":\"%s\",\"name\":", widget == pdf_first_widget(ctx, page) ? "" : ",",
				pdf_to_num(ctx, obj), kind_name(type));
			ely_json_string(ctx, out, name);
			fz_write_printf(ctx, out, ",\"rect\":[%g,%g,%g,%g],\"value\":", r.x0, r.y0, r.x1, r.y1);
			write_value(ctx, out, pdf_dict_get_inheritable(ctx, obj, PDF_NAME(V)));
			fz_write_string(ctx, out, ",\"on\":");
			ely_json_string(ctx, out, button ? pdf_to_name(ctx, pdf_button_field_on_state(ctx, obj)) : "");
			fz_write_printf(ctx, out, ",\"checked\":%s,\"readonly\":%s,\"options\":",
				button && pdf_is_name(ctx, as) && !pdf_name_eq(ctx, as, PDF_NAME(Off)) ? "true" : "false",
				pdf_widget_is_readonly(ctx, widget) ? "true" : "false");
			write_options(ctx, out, obj, 1);
			fz_write_string(ctx, out, ",\"labels\":");
			write_options(ctx, out, obj, 0);
			fz_write_byte(ctx, out, '}');
		}
	}
	fz_always(ctx)
		fz_free(ctx, name);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* Shows `state` on each button under `node`, else Off. */
static void show_state(fz_context *ctx, pdf_obj *node, pdf_obj *state)
{
	pdf_obj *kids = pdf_dict_get(ctx, node, PDF_NAME(Kids));
	int i;
	if (kids)
	{
		for (i = 0; i < pdf_array_len(ctx, kids); i++)
			show_state(ctx, pdf_array_get(ctx, kids, i), state);
		return;
	}
	pdf_dict_put(ctx, node, PDF_NAME(AS), pdf_dict_get(ctx, pdf_dict_getp(ctx, node, "AP/N"), state) ? state : PDF_NAME(Off));
}

/* Fills field widget `id` with `value`, in one step. */
int ely_fill(fz_context *ctx, fz_document *doc, int number, int id, const char *value)
{
	pdf_document *pdf = NULL;
	pdf_page *page = NULL;
	pdf_obj *state = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(page);
	fz_var(state);
	fz_var(begun);
	fz_try(ctx)
	{
		pdf_annot *widget;
		pdf_obj *obj;
		enum pdf_widget_type type;
		pdf = ely_pdf_of(ctx, doc);
		page = pdf_load_page(ctx, pdf, number);
		widget = ely_find_annot(ctx, page, id, 1);
		obj = pdf_annot_obj(ctx, widget);
		type = pdf_widget_type(ctx, widget);
		if (pdf_widget_is_readonly(ctx, widget))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "the field is read only");
		if (type == PDF_WIDGET_TYPE_CHECKBOX || type == PDF_WIDGET_TYPE_RADIOBUTTON)
		{
			state = pdf_new_name(ctx, value);
			if (!pdf_name_eq(ctx, state, PDF_NAME(Off)) && !pdf_name_eq(ctx, state, pdf_button_field_on_state(ctx, obj)))
				fz_throw(ctx, FZ_ERROR_ARGUMENT, "the button has no state %s", value);
		}
		else if (type != PDF_WIDGET_TYPE_TEXT && type != PDF_WIDGET_TYPE_COMBOBOX && type != PDF_WIDGET_TYPE_LISTBOX)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a %s field cannot be filled", kind_name(type));
		pdf_begin_operation(ctx, pdf, "Fill a field");
		begun = 1;
		if (state)
		{
			pdf_dict_put(ctx, head_of(ctx, obj), PDF_NAME(V), state);
			show_state(ctx, head_of(ctx, obj), state);
		}
		else if (!pdf_set_field_value(ctx, pdf, obj, value, 1))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "the field refused the value");
		pdf_update_annot(ctx, widget);
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		pdf_drop_obj(ctx, state);
		fz_drop_page(ctx, (fz_page *)page);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

/* Throws unless `prefix` plus `name` is free for `kind`. */
static pdf_obj *check_name(fz_context *ctx, pdf_obj *fields, const char *prefix, const char *name, const char *kind,
	pdf_obj *self)
{
	pdf_obj *found = NULL;
	char *full = NULL;
	if (!name[0] || strchr(name, '.'))
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a bad field name");
	fz_var(full);
	fz_var(found);
	fz_try(ctx)
	{
		full = fz_asprintf(ctx, "%s%s", prefix, name);
		found = pdf_lookup_field(ctx, fields, full);
	}
	fz_always(ctx)
		fz_free(ctx, full);
	fz_catch(ctx)
		fz_rethrow(ctx);
	if (!found || (self && !pdf_objcmp(ctx, found, self)))
		return NULL;
	if (!strcmp(kind, "radio") && pdf_field_type(ctx, found) == PDF_WIDGET_TYPE_RADIOBUTTON &&
		pdf_dict_get(ctx, found, PDF_NAME(Kids)))
		return found;
	fz_throw(ctx, FZ_ERROR_ARGUMENT, "a field named %s%s exists", prefix, name);
}

/* The full name of `field`'s parent, with a dot. */
static char *parent_path(fz_context *ctx, pdf_obj *field)
{
	pdf_obj *parent = pdf_dict_get(ctx, field, PDF_NAME(Parent));
	char *name = parent ? pdf_load_field_name(ctx, parent) : NULL, *path = NULL;
	fz_try(ctx)
		path = fz_asprintf(ctx, "%s%s", name ? name : "", name && name[0] ? "." : "");
	fz_always(ctx)
		fz_free(ctx, name);
	fz_catch(ctx)
		fz_rethrow(ctx);
	return path;
}

static void put_options(fz_context *ctx, pdf_obj *field, fz_json *options)
{
	pdf_obj *opt = pdf_dict_put_array(ctx, field, PDF_NAME(Opt), 4);
	int i, n = fz_json_array_length(ctx, options);
	for (i = 0; i < n; i++)
		pdf_array_push_text_string(ctx, opt, fz_json_array_get(ctx, options, i)->u.string);
}

static void check_options(fz_context *ctx, fz_json *options)
{
	int i, n = options ? fz_json_array_length(ctx, options) : 0;
	if (n == 0)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a choice with no options");
	for (i = 0; i < n; i++)
		if (fz_json_array_get(ctx, options, i)->type != FZ_JSON_STRING || !fz_json_array_get(ctx, options, i)->u.string[0])
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a bad option");
}

/* An on state no button in `group` has. */
static pdf_obj *new_state(fz_context *ctx, pdf_obj *group)
{
	pdf_obj *kids = pdf_dict_get(ctx, group, PDF_NAME(Kids));
	char name[32];
	int n, i, taken;
	for (n = pdf_array_len(ctx, kids) + 1;; n++)
	{
		snprintf(name, sizeof name, "Choice%d", n);
		for (i = 0, taken = 0; i < pdf_array_len(ctx, kids); i++)
			taken |= !strcmp(pdf_to_name(ctx, pdf_button_field_on_state(ctx, pdf_array_get(ctx, kids, i))), name);
		if (!taken)
			return pdf_new_name(ctx, name);
	}
}

/* Adds a field of four kinds, in one step. */
int ely_add_field(fz_context *ctx, fz_document *doc, int number, const char *json)
{
	pdf_document *pdf = NULL;
	pdf_page *page = NULL;
	pdf_annot *widget = NULL;
	pdf_obj *state = NULL;
	fz_pool *pool = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(page);
	fz_var(widget);
	fz_var(state);
	fz_var(pool);
	fz_var(begun);
	fz_try(ctx)
	{
		fz_json *spec, *options, *kindj, *namej;
		const char *kind, *name;
		pdf_obj *fields, *group, *obj, *mk;
		fz_rect rect;
		pdf = ely_pdf_of(ctx, doc);
		page = pdf_load_page(ctx, pdf, number);
		pool = fz_new_pool(ctx);
		spec = fz_parse_json(ctx, pool, json);
		kindj = ely_member(ctx, spec, "kind", FZ_JSON_STRING);
		namej = ely_member(ctx, spec, "name", FZ_JSON_STRING);
		options = ely_member(ctx, spec, "options", FZ_JSON_ARRAY);
		if (!kindj || !namej)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a field with no kind or name");
		kind = kindj->u.string, name = namej->u.string;
		if (strcmp(kind, "text") && strcmp(kind, "checkbox") && strcmp(kind, "radio") && strcmp(kind, "choice"))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "no field kind %s", kind);
		if (!strcmp(kind, "choice"))
			check_options(ctx, options);
		rect = ely_rect(ctx, ely_member(ctx, spec, "rect", FZ_JSON_ARRAY), 4);
		group = check_name(ctx, pdf_dict_getp(ctx, pdf_trailer(ctx, pdf), "Root/AcroForm/Fields"), "", name, kind, NULL);
		pdf_begin_operation(ctx, pdf, "Add a field");
		begun = 1;
		fields = fields_of(ctx, pdf);
		widget = pdf_create_annot_raw(ctx, page, PDF_ANNOT_WIDGET);
		obj = pdf_annot_obj(ctx, widget);
		pdf_dict_put(ctx, obj, PDF_NAME(P), page->obj);
		pdf_dict_put_int(ctx, obj, PDF_NAME(F), PDF_ANNOT_IS_PRINT);
		pdf_dict_put_rect(ctx, obj, PDF_NAME(Rect), fz_transform_rect(rect, ely_to_user(ctx, page)));
		pdf_dict_put_text_string(ctx, obj, PDF_NAME(DA), "/Helv 0 Tf 0 g");
		mk = pdf_dict_put_dict(ctx, obj, PDF_NAME(MK), 1);
		pdf_array_push_real(ctx, pdf_dict_put_array(ctx, mk, PDF_NAME(BC), 1), 0);
		if (!strcmp(kind, "radio"))
		{
			if (!group)
			{
				group = pdf_add_new_dict(ctx, pdf, 4);
				pdf_dict_put(ctx, group, PDF_NAME(FT), PDF_NAME(Btn));
				pdf_dict_put_int(ctx, group, PDF_NAME(Ff), PDF_BTN_FIELD_IS_RADIO | PDF_BTN_FIELD_IS_NO_TOGGLE_TO_OFF);
				pdf_dict_put_text_string(ctx, group, PDF_NAME(T), name);
				pdf_dict_put(ctx, group, PDF_NAME(V), PDF_NAME(Off));
				pdf_dict_put_array(ctx, group, PDF_NAME(Kids), 2);
				pdf_array_push_drop(ctx, fields, group);
			}
			state = new_state(ctx, group);
			pdf_dict_put(ctx, obj, PDF_NAME(Parent), group);
			pdf_array_push(ctx, pdf_dict_get(ctx, group, PDF_NAME(Kids)), obj);
		}
		else
		{
			pdf_dict_put_text_string(ctx, obj, PDF_NAME(T), name);
			pdf_array_push(ctx, fields, obj);
		}
		if (!strcmp(kind, "text"))
			pdf_dict_put(ctx, obj, PDF_NAME(FT), PDF_NAME(Tx));
		else if (!strcmp(kind, "choice"))
		{
			pdf_dict_put(ctx, obj, PDF_NAME(FT), PDF_NAME(Ch));
			pdf_dict_put_int(ctx, obj, PDF_NAME(Ff), PDF_CH_FIELD_IS_COMBO);
			put_options(ctx, obj, options);
		}
		else if (!strcmp(kind, "checkbox"))
		{
			pdf_dict_put(ctx, obj, PDF_NAME(FT), PDF_NAME(Btn));
			pdf_dict_put(ctx, obj, PDF_NAME(V), PDF_NAME(Off));
			state = pdf_new_name(ctx, "Yes");
		}
		/* The on look is drawn under its own state. */
		if (state)
			pdf_dict_put(ctx, obj, PDF_NAME(AS), state);
		pdf_annot_request_resynthesis(ctx, widget);
		pdf_update_annot(ctx, widget);
		if (state)
			pdf_dict_put(ctx, obj, PDF_NAME(AS), PDF_NAME(Off));
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		fz_drop_pool(ctx, pool);
		pdf_drop_obj(ctx, state);
		pdf_drop_annot(ctx, widget);
		fz_drop_page(ctx, (fz_page *)page);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

/* Renames a field or sets its options. */
int ely_change_field(fz_context *ctx, fz_document *doc, int number, int id, const char *json)
{
	pdf_document *pdf = NULL;
	pdf_page *page = NULL;
	fz_pool *pool = NULL;
	char *path = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(page);
	fz_var(pool);
	fz_var(path);
	fz_var(begun);
	fz_try(ctx)
	{
		pdf_annot *widget;
		pdf_obj *obj, *head;
		fz_json *spec, *name, *options;
		enum pdf_widget_type type;
		pdf = ely_pdf_of(ctx, doc);
		page = pdf_load_page(ctx, pdf, number);
		widget = ely_find_annot(ctx, page, id, 1);
		obj = pdf_annot_obj(ctx, widget);
		head = head_of(ctx, obj);
		type = pdf_widget_type(ctx, widget);
		pool = fz_new_pool(ctx);
		spec = fz_parse_json(ctx, pool, json);
		name = ely_member(ctx, spec, "name", FZ_JSON_STRING);
		options = ely_member(ctx, spec, "options", FZ_JSON_ARRAY);
		if (!name && !options)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "no change");
		if (name)
		{
			path = parent_path(ctx, head);
			check_name(ctx, pdf_dict_getp(ctx, pdf_trailer(ctx, pdf), "Root/AcroForm/Fields"), path, name->u.string, "", head);
		}
		if (options && type != PDF_WIDGET_TYPE_COMBOBOX && type != PDF_WIDGET_TYPE_LISTBOX)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "only a choice has options");
		if (options)
			check_options(ctx, options);
		pdf_begin_operation(ctx, pdf, "Change a field");
		begun = 1;
		if (name)
			pdf_dict_put_text_string(ctx, head, PDF_NAME(T), name->u.string);
		if (options)
		{
			int i, kept = 0;
			const char *value = pdf_to_text_string(ctx, pdf_dict_get_inheritable(ctx, obj, PDF_NAME(V)));
			for (i = 0; i < fz_json_array_length(ctx, options); i++)
				kept |= !strcmp(fz_json_array_get(ctx, options, i)->u.string, value);
			if (obj != head)
				pdf_dict_del(ctx, obj, PDF_NAME(Opt));
			put_options(ctx, head, options);
			if (!kept)
				pdf_dict_del(ctx, head, PDF_NAME(V));
		}
		pdf_annot_request_resynthesis(ctx, widget);
		pdf_update_annot(ctx, widget);
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		fz_drop_pool(ctx, pool);
		fz_free(ctx, path);
		fz_drop_page(ctx, (fz_page *)page);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

/* Writes each named node under `kids`, with its full name. */
static void write_names(fz_context *ctx, fz_output *out, pdf_obj *kids, const char *prefix, int depth, int *first)
{
	int i;
	if (depth > 32)
		fz_throw(ctx, FZ_ERROR_FORMAT, "a field tree too deep");
	for (i = 0; i < pdf_array_len(ctx, kids); i++)
	{
		pdf_obj *node = pdf_array_get(ctx, kids, i);
		pdf_obj *t = pdf_dict_get(ctx, node, PDF_NAME(T));
		char *name = NULL;
		fz_var(name);
		fz_try(ctx)
		{
			name = t ? fz_asprintf(ctx, "%s%s%s", prefix, prefix[0] ? "." : "", pdf_to_text_string(ctx, t)) : fz_strdup(ctx, prefix);
			if (t)
			{
				fz_write_string(ctx, out, *first ? "" : ",");
				ely_json_string(ctx, out, name);
				*first = 0;
			}
			write_names(ctx, out, pdf_dict_get(ctx, node, PDF_NAME(Kids)), name, depth + 1, first);
		}
		fz_always(ctx)
			fz_free(ctx, name);
		fz_catch(ctx)
			fz_rethrow(ctx);
	}
}

/* Every field name in the form, widgets or not. */
int ely_field_names(fz_context *ctx, fz_document *doc, unsigned char **data, size_t *len)
{
	fz_buffer *buf = NULL;
	fz_output *out = NULL;
	fz_var(buf);
	fz_var(out);
	fz_try(ctx)
	{
		int first = 1;
		pdf_document *pdf = ely_pdf_of(ctx, doc);
		buf = fz_new_buffer(ctx, 256);
		out = fz_new_output_with_buffer(ctx, buf);
		fz_write_byte(ctx, out, '[');
		write_names(ctx, out, pdf_dict_getp(ctx, pdf_trailer(ctx, pdf), "Root/AcroForm/Fields"), "", 0, &first);
		fz_write_byte(ctx, out, ']');
		fz_close_output(ctx, out);
		*len = fz_buffer_extract(ctx, buf, data);
	}
	fz_always(ctx)
	{
		fz_drop_output(ctx, out);
		fz_drop_buffer(ctx, buf);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}
