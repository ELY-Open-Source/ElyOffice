/* Pages from another PDF, with their links, notes and fields. */

#include <stdio.h>
#include <string.h>

#include "ely_shim.h"

/* A source link to its own page `page`. */
typedef struct
{
	pdf_obj *annot;
	pdf_obj *rest;
	int page;
} inner_link;

/* Takes page references off; links noted, `*n` counted as kept. */
static void unhook(fz_context *ctx, pdf_document *src, inner_link *links, int room, int *n)
{
	int i, j, count = pdf_count_pages(ctx, src);
	for (i = 0; i < count; i++)
	{
		pdf_obj *annots = pdf_dict_get(ctx, pdf_lookup_page_obj(ctx, src, i), PDF_NAME(Annots));
		for (j = 0; j < pdf_array_len(ctx, annots); j++)
		{
			pdf_obj *annot = pdf_array_get(ctx, annots, j);
			pdf_obj *page = ely_target(ctx, src, annot);
			int to = page ? pdf_lookup_page_number(ctx, src, page) : -1;
			pdf_dict_del(ctx, annot, PDF_NAME(P));
			if (to < 0 || *n == room)
				continue;
			/* The view after the page: /XYZ, /Fit and so on. */
			pdf_obj *dest = pdf_dict_get(ctx, annot, PDF_NAME(Dest));
			if (!dest)
				dest = pdf_dict_getp(ctx, annot, "A/D");
			if (pdf_is_name(ctx, dest) || pdf_is_string(ctx, dest))
				dest = pdf_lookup_dest(ctx, src, dest);
			if (pdf_is_dict(ctx, dest))
				dest = pdf_dict_get(ctx, dest, PDF_NAME(D));
			/* Counted as soon as kept: a throw still frees it. */
			links[*n].annot = pdf_keep_obj(ctx, annot);
			links[*n].rest = NULL;
			links[*n].page = to;
			(*n)++;
			links[*n - 1].rest = pdf_copy_array(ctx, dest);
			pdf_array_delete(ctx, links[*n - 1].rest, 0);
			pdf_dict_del(ctx, annot, PDF_NAME(Dest));
			pdf_dict_del(ctx, annot, PDF_NAME(A));
		}
	}
}

/* The top of the field tree `widget` belongs to. */
static pdf_obj *root_field(fz_context *ctx, pdf_obj *widget)
{
	pdf_obj *parent;
	while ((parent = pdf_dict_get(ctx, widget, PDF_NAME(Parent))) != NULL)
		widget = parent;
	return widget;
}

/* The source form's default look and its font renames. */
typedef struct
{
	pdf_obj *da;
	pdf_obj *q;
	pdf_obj *renames;
} form_look;

static int hex(int c)
{
	return c >= '0' && c <= '9' ? c - '0' : (c | 32) >= 'a' && (c | 32) <= 'f' ? (c | 32) - 'a' + 10 : -1;
}

/* A name token's bytes, #xx escapes decoded. */
static void decode_name(const char *token, size_t len, char *out)
{
	size_t i, k = 0;
	for (i = 0; i < len; i++)
	{
		if (token[i] == '#' && i + 2 < len && hex(token[i + 1]) >= 0 && hex(token[i + 2]) >= 0)
		{
			out[k++] = (char)(hex(token[i + 1]) * 16 + hex(token[i + 2]));
			i += 2;
		}
		else
			out[k++] = token[i];
	}
	out[k] = 0;
}

/* Writes `name` as a name token, odd bytes escaped. */
static void encode_name(fz_context *ctx, fz_buffer *buf, const char *name)
{
	const unsigned char *c;
	fz_append_byte(ctx, buf, '/');
	for (c = (const unsigned char *)name; *c; c++)
		if (*c < 0x21 || *c > 0x7e || strchr("#/[]()<>{}%", *c))
			fz_append_printf(ctx, buf, "#%02X", *c);
		else
			fz_append_byte(ctx, buf, *c);
}

/* `name`, or `name` and `n` when n is above one. */
static char *candidate(fz_context *ctx, const char *name, const char *gap, int n)
{
	return n == 1 ? fz_strdup(ctx, name) : fz_asprintf(ctx, "%s%s%d", name, gap, n);
}

/* Merges the source's form fonts; taken names get numbered. */
static void merge_fonts(fz_context *ctx, pdf_document *pdf, pdf_graft_map *map, pdf_document *src, form_look *look)
{
	pdf_obj *from = pdf_dict_getp(ctx, pdf_trailer(ctx, src), "Root/AcroForm/DR/Font");
	pdf_obj *to, *font = NULL;
	char *fresh = NULL;
	int i, n;
	fz_var(font);
	fz_var(fresh);
	if (!from)
		return;
	to = pdf_dict_getp(ctx, pdf_trailer(ctx, pdf), "Root/AcroForm/DR/Font");
	if (!to)
	{
		to = pdf_new_dict(ctx, pdf, 4);
		pdf_dict_putp_drop(ctx, pdf_trailer(ctx, pdf), "Root/AcroForm/DR/Font", to);
	}
	fz_try(ctx)
		for (i = 0; i < pdf_dict_len(ctx, from); i++)
		{
			const char *name = pdf_to_name(ctx, pdf_dict_get_key(ctx, from, i));
			font = pdf_graft_mapped_object(ctx, map, pdf_dict_get_val(ctx, from, i));
			for (n = 1;; n++)
			{
				pdf_obj *held;
				fz_free(ctx, fresh);
				fresh = candidate(ctx, name, "_", n);
				held = pdf_dict_gets(ctx, to, fresh);
				if (!held)
					pdf_dict_puts(ctx, to, fresh, font);
				if (!held || (pdf_is_indirect(ctx, held) && pdf_to_num(ctx, held) == pdf_to_num(ctx, font)))
					break;
			}
			if (n > 1)
				pdf_dict_puts_drop(ctx, look->renames, name, pdf_new_name(ctx, fresh));
			pdf_drop_obj(ctx, font);
			font = NULL;
		}
	fz_always(ctx)
	{
		pdf_drop_obj(ctx, font);
		fz_free(ctx, fresh);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* `node`'s /DA with renamed fonts renamed. */
static void rename_fonts(fz_context *ctx, pdf_obj *node, pdf_obj *renames)
{
	const char *da = pdf_dict_get_text_string(ctx, node, PDF_NAME(DA));
	fz_buffer *buf = NULL;
	char *name = NULL;
	fz_var(buf);
	fz_var(name);
	if (!*da || !pdf_dict_len(ctx, renames))
		return;
	fz_try(ctx)
	{
		unsigned char *data;
		size_t len;
		buf = fz_new_buffer(ctx, 64);
		while (*da)
		{
			size_t word = 1;
			pdf_obj *to;
			if (*da != '/')
			{
				fz_append_byte(ctx, buf, *da++);
				continue;
			}
			while (da[word] && !strchr(" \t\r\n\f/[]()<>{}%", da[word]))
				word++;
			fz_free(ctx, name);
			name = fz_malloc(ctx, word);
			decode_name(da + 1, word - 1, name);
			to = pdf_dict_gets(ctx, renames, name);
			if (to)
				encode_name(ctx, buf, pdf_to_name(ctx, to));
			else
				fz_append_data(ctx, buf, da, word);
			da += word;
		}
		len = fz_buffer_storage(ctx, buf, &data);
		pdf_dict_put_string(ctx, node, PDF_NAME(DA), (const char *)data, len);
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, buf);
		fz_free(ctx, name);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* Gives a field tree the source's look, fonts renamed. */
static void restyle(fz_context *ctx, pdf_obj *node, form_look *look, int root)
{
	pdf_obj *kids = pdf_dict_get(ctx, node, PDF_NAME(Kids));
	int i;
	/* The form's defaults a field inherits: look and alignment. */
	if (root && look->da && !pdf_dict_get(ctx, node, PDF_NAME(DA)))
		pdf_dict_put(ctx, node, PDF_NAME(DA), look->da);
	if (root && look->q && !pdf_dict_get(ctx, node, PDF_NAME(Q)))
		pdf_dict_put(ctx, node, PDF_NAME(Q), look->q);
	rename_fonts(ctx, node, look->renames);
	for (i = 0; i < pdf_array_len(ctx, kids); i++)
		restyle(ctx, pdf_array_get(ctx, kids, i), look, 0);
}

/* Joins `field` to the form; a taken name gets numbered. */
static void add_field(fz_context *ctx, pdf_document *pdf, pdf_obj *field, form_look *look)
{
	pdf_obj *fields = pdf_dict_getp(ctx, pdf_trailer(ctx, pdf), "Root/AcroForm/Fields");
	const char *name = pdf_dict_get_text_string(ctx, field, PDF_NAME(T));
	char *renamed = NULL;
	int i, n, taken;
	fz_var(renamed);
	if (!fields)
	{
		fields = pdf_new_array(ctx, pdf, 1);
		pdf_dict_putp_drop(ctx, pdf_trailer(ctx, pdf), "Root/AcroForm/Fields", fields);
	}
	for (i = 0; i < pdf_array_len(ctx, fields); i++)
		if (pdf_to_num(ctx, pdf_array_get(ctx, fields, i)) == pdf_to_num(ctx, field))
			return;
	fz_try(ctx)
	{
		for (n = 1, taken = 1; taken; n++)
		{
			fz_free(ctx, renamed);
			renamed = candidate(ctx, name, " ", n);
			for (taken = 0, i = 0; i < pdf_array_len(ctx, fields) && !taken; i++)
				taken = !strcmp(pdf_dict_get_text_string(ctx, pdf_array_get(ctx, fields, i), PDF_NAME(T)), renamed);
		}
		if (strcmp(renamed, name))
			pdf_dict_put_text_string(ctx, field, PDF_NAME(T), renamed);
		restyle(ctx, field, look, 1);
		pdf_array_push(ctx, fields, field);
	}
	fz_always(ctx)
		fz_free(ctx, renamed);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* Grafts each page's annotations and reconnects them. */
static void reconnect(fz_context *ctx, pdf_document *pdf, pdf_graft_map *map, pdf_document *src, int at, form_look *look)
{
	int i, j, count = pdf_count_pages(ctx, src);
	for (i = 0; i < count; i++)
	{
		pdf_obj *annots = pdf_dict_get(ctx, pdf_lookup_page_obj(ctx, src, i), PDF_NAME(Annots));
		pdf_obj *page = pdf_lookup_page_obj(ctx, pdf, at + i);
		pdf_obj *grafted;
		if (!annots)
			continue;
		grafted = pdf_graft_mapped_object(ctx, map, annots);
		pdf_dict_put_drop(ctx, page, PDF_NAME(Annots), grafted);
		for (j = 0; j < pdf_array_len(ctx, grafted); j++)
		{
			pdf_obj *annot = pdf_array_get(ctx, grafted, j);
			pdf_dict_put(ctx, annot, PDF_NAME(P), page);
			if (pdf_name_eq(ctx, pdf_dict_get(ctx, annot, PDF_NAME(Subtype)), PDF_NAME(Widget)))
				add_field(ctx, pdf, root_field(ctx, annot), look);
		}
	}
}

/* Annotations and form look come over; inner links point home. */
static void graft_annots(fz_context *ctx, pdf_document *pdf, pdf_graft_map *map, pdf_document *src, int at,
	inner_link *links, int n)
{
	form_look look = { NULL, NULL, NULL };
	pdf_obj *da = pdf_dict_getp(ctx, pdf_trailer(ctx, src), "Root/AcroForm/DA");
	pdf_obj *q = pdf_dict_getp(ctx, pdf_trailer(ctx, src), "Root/AcroForm/Q");
	int i, j;
	fz_var(look.da);
	fz_var(look.renames);
	fz_var(look.q);
	fz_try(ctx)
	{
		look.renames = pdf_new_dict(ctx, pdf, 1);
		look.da = da ? pdf_graft_mapped_object(ctx, map, da) : NULL;
		look.q = q ? pdf_graft_mapped_object(ctx, map, q) : NULL;
		merge_fonts(ctx, pdf, map, src, &look);
		reconnect(ctx, pdf, map, src, at, &look);
	}
	fz_always(ctx)
	{
		pdf_drop_obj(ctx, look.renames);
		pdf_drop_obj(ctx, look.da);
		pdf_drop_obj(ctx, look.q);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
	for (i = 0; i < n; i++)
	{
		pdf_obj *annot = pdf_graft_mapped_object(ctx, map, links[i].annot);
		pdf_obj *dest = pdf_new_array(ctx, pdf, 1 + pdf_array_len(ctx, links[i].rest));
		pdf_array_push(ctx, dest, pdf_lookup_page_obj(ctx, pdf, at + links[i].page));
		for (j = 0; j < pdf_array_len(ctx, links[i].rest); j++)
			pdf_array_push(ctx, dest, pdf_array_get(ctx, links[i].rest, j));
		pdf_dict_put_drop(ctx, annot, PDF_NAME(Dest), dest);
		pdf_drop_obj(ctx, annot);
	}
}

/* Every page of the PDF at `path`, inserted at `at`. */
int ely_insert_file(fz_context *ctx, fz_document *doc, int at, const char *path, int *count)
{
	pdf_document *pdf = NULL, *src = NULL;
	pdf_graft_map *map = NULL;
	inner_link *links = NULL;
	int begun = 0, i, n = 0, room = 0;
	fz_var(pdf);
	fz_var(src);
	fz_var(map);
	fz_var(links);
	fz_var(n);
	fz_var(begun);
	fz_var(room);
	fz_try(ctx)
	{
		pdf = ely_pdf_of(ctx, doc);
		if (at < 0 || at > pdf_count_pages(ctx, pdf))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "no place %d to insert at", at);
		src = pdf_open_document(ctx, path);
		if (pdf_needs_password(ctx, src))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "the file to insert is locked");
		*count = pdf_count_pages(ctx, src);
		for (i = 0; i < *count; i++)
			room += pdf_array_len(ctx, pdf_dict_get(ctx, pdf_lookup_page_obj(ctx, src, i), PDF_NAME(Annots)));
		links = fz_calloc(ctx, room + 1, sizeof *links);
		unhook(ctx, src, links, room, &n);
		map = pdf_new_graft_map(ctx, pdf);
		pdf_begin_operation(ctx, pdf, "Insert pages");
		begun = 1;
		for (i = 0; i < *count; i++)
			pdf_graft_mapped_page(ctx, map, at + i, src, i);
		graft_annots(ctx, pdf, map, src, at, links, n);
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		for (i = 0; links && i < n; i++)
		{
			pdf_drop_obj(ctx, links[i].annot);
			pdf_drop_obj(ctx, links[i].rest);
		}
		fz_free(ctx, links);
		pdf_drop_graft_map(ctx, map);
		pdf_drop_document(ctx, src);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}
