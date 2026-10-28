/* Document properties and attachments (PDF-07). */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "ely_shim.h"

static const char *shown[] = { "Title", "Author", "Subject", "Keywords", "Creator", "Producer", "CreationDate",
	"ModDate", NULL };
/* Only these change; the rest the producer wrote. */
static const char *editable[] = { "Title", "Author", "Subject", "Keywords", NULL };

/* Properties as JSON; null when the file has none. */
int ely_metadata(fz_context *ctx, fz_document *doc, unsigned char **data, size_t *len)
{
	fz_buffer *buf = NULL;
	fz_output *out = NULL;
	char *value = NULL;
	fz_var(buf);
	fz_var(out);
	fz_var(value);
	fz_try(ctx)
	{
		char key[32];
		int i, n;
		buf = fz_new_buffer(ctx, 256);
		out = fz_new_output_with_buffer(ctx, buf);
		fz_write_byte(ctx, out, '{');
		for (i = 0; shown[i]; i++)
		{
			fz_snprintf(key, sizeof key, "info:%s", shown[i]);
			fz_write_printf(ctx, out, "%s\"%s\":", i ? "," : "", shown[i]);
			n = fz_lookup_metadata(ctx, doc, key, NULL, 0);
			if (n < 0)
			{
				fz_write_string(ctx, out, "null");
				continue;
			}
			fz_free(ctx, value);
			value = NULL;
			value = fz_malloc(ctx, n);
			fz_lookup_metadata(ctx, doc, key, value, n);
			ely_json_string(ctx, out, value);
		}
		fz_write_byte(ctx, out, '}');
		fz_close_output(ctx, out);
		*len = fz_buffer_extract(ctx, buf, data);
	}
	fz_always(ctx)
	{
		fz_free(ctx, value);
		fz_drop_output(ctx, out);
		fz_drop_buffer(ctx, buf);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* Sets the given properties, in one step. */
int ely_set_metadata(fz_context *ctx, fz_document *doc, const char *json)
{
	pdf_document *pdf = NULL;
	fz_pool *pool = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(pool);
	fz_var(begun);
	fz_try(ctx)
	{
		fz_json *spec;
		fz_json_object *field;
		char key[32];
		int i;
		pdf = ely_pdf_of(ctx, doc);
		pool = fz_new_pool(ctx);
		spec = fz_parse_json(ctx, pool, json);
		if (spec->type != FZ_JSON_OBJECT || !spec->u.object)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "no properties to set");
		for (field = spec->u.object; field; field = field->next)
		{
			for (i = 0; editable[i] && strcmp(editable[i], field->key); i++)
				continue;
			if (!editable[i] || field->value->type != FZ_JSON_STRING)
				fz_throw(ctx, FZ_ERROR_ARGUMENT, "a property that does not change: %s", field->key);
		}
		pdf_begin_operation(ctx, pdf, "Set properties");
		begun = 1;
		for (field = spec->u.object; field; field = field->next)
		{
			fz_snprintf(key, sizeof key, "info:%s", field->key);
			fz_set_metadata(ctx, doc, key, field->value->u.string);
		}
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
		fz_drop_pool(ctx, pool);
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

static int by_name(const void *a, const void *b)
{
	return strcmp(*(const char *const *)a, *(const char *const *)b);
}

/* The embedded files, by name; NULL names end it. */
static const char **names_of(fz_context *ctx, pdf_obj *tree, int *count)
{
	const char **names;
	int i, n = pdf_dict_len(ctx, tree);
	names = fz_malloc_array(ctx, n + 1, const char *);
	for (i = 0; i < n; i++)
		names[i] = pdf_to_name(ctx, pdf_dict_get_key(ctx, tree, i));
	names[n] = NULL;
	qsort(names, n, sizeof *names, by_name);
	*count = n;
	return names;
}

/* An embedded file's length: its /Size, else its bytes. */
static int64_t file_size(fz_context *ctx, pdf_obj *fs)
{
	pdf_obj *ef = pdf_dict_get(ctx, fs, PDF_NAME(EF));
	pdf_obj *file = pdf_dict_get(ctx, ef, PDF_NAME(F)) ? pdf_dict_get(ctx, ef, PDF_NAME(F)) : pdf_dict_get(ctx, ef, PDF_NAME(UF));
	pdf_obj *size = pdf_dict_getl(ctx, file, PDF_NAME(Params), PDF_NAME(Size), NULL);
	fz_buffer *bytes;
	int64_t n;
	if (pdf_is_int(ctx, size))
		return pdf_to_int64(ctx, size);
	bytes = pdf_load_embedded_file_contents(ctx, fs);
	n = (int64_t)fz_buffer_storage(ctx, bytes, NULL);
	fz_drop_buffer(ctx, bytes);
	return n;
}

static void write_file(fz_context *ctx, fz_output *out, const char *key, const char *name, int page, pdf_obj *fs, int *first)
{
	pdf_filespec_params params;
	pdf_get_filespec_params(ctx, fs, &params);
	fz_write_string(ctx, out, *first ? "{\"key\":" : ",{\"key\":");
	*first = 0;
	ely_json_string(ctx, out, key);
	fz_write_string(ctx, out, ",\"name\":");
	ely_json_string(ctx, out, name);
	fz_write_printf(ctx, out, ",\"page\":%d,\"size\":%ld,\"created\":%ld,\"modified\":%ld,\"type\":", page,
		file_size(ctx, fs), (int64_t)params.created, (int64_t)params.modified);
	ely_json_string(ctx, out, params.mimetype ? params.mimetype : "");
	fz_write_string(ctx, out, ",\"note\":");
	ely_json_string(ctx, out, pdf_dict_get_text_string(ctx, fs, PDF_NAME(Desc)));
	fz_write_byte(ctx, out, '}');
}

/* The page's file annotations, as attachments. */
static void write_page_files(fz_context *ctx, fz_output *out, pdf_document *pdf, int number, int *first)
{
	pdf_page *page = pdf_load_page(ctx, pdf, number);
	char key[48];
	fz_try(ctx)
	{
		pdf_annot *annot;
		for (annot = pdf_first_annot(ctx, page); annot; annot = pdf_next_annot(ctx, annot))
		{
			pdf_obj *fs;
			pdf_filespec_params params;
			if (pdf_annot_type(ctx, annot) != PDF_ANNOT_FILE_ATTACHMENT)
				continue;
			fs = pdf_annot_filespec(ctx, annot);
			if (!pdf_is_embedded_file(ctx, fs))
				continue;
			pdf_get_filespec_params(ctx, fs, &params);
			fz_snprintf(key, sizeof key, "a:%d:%d", number, pdf_to_num(ctx, pdf_annot_obj(ctx, annot)));
			write_file(ctx, out, key, params.filename ? params.filename : "", number, fs, first);
		}
	}
	fz_always(ctx)
		fz_drop_page(ctx, (fz_page *)page);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* Every embedded file: the tree's by name, then pages'. */
int ely_attachments(fz_context *ctx, fz_document *doc, unsigned char **data, size_t *len)
{
	pdf_obj *tree = NULL;
	const char **names = NULL;
	fz_buffer *buf = NULL;
	fz_output *out = NULL;
	char *key = NULL;
	fz_var(tree);
	fz_var(names);
	fz_var(buf);
	fz_var(out);
	fz_var(key);
	fz_try(ctx)
	{
		pdf_document *pdf = ely_pdf_of(ctx, doc);
		int i, n, first = 1;
		tree = pdf_load_name_tree(ctx, pdf, PDF_NAME(EmbeddedFiles));
		names = names_of(ctx, tree, &n);
		buf = fz_new_buffer(ctx, 256);
		out = fz_new_output_with_buffer(ctx, buf);
		fz_write_byte(ctx, out, '[');
		for (i = 0; i < n; i++)
		{
			pdf_obj *fs = pdf_dict_gets(ctx, tree, names[i]);
			if (!pdf_is_embedded_file(ctx, fs))
				continue;
			fz_free(ctx, key);
			key = NULL;
			key = fz_asprintf(ctx, "n:%s", names[i]);
			write_file(ctx, out, key, names[i], -1, fs, &first);
		}
		for (i = 0; i < pdf_count_pages(ctx, pdf); i++)
			write_page_files(ctx, out, pdf, i, &first);
		fz_write_byte(ctx, out, ']');
		fz_close_output(ctx, out);
		*len = fz_buffer_extract(ctx, buf, data);
	}
	fz_always(ctx)
	{
		fz_free(ctx, key);
		fz_drop_output(ctx, out);
		fz_drop_buffer(ctx, buf);
		fz_free(ctx, names);
		pdf_drop_obj(ctx, tree);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* The file `key` names; a page annotation's loads its page. */
static pdf_obj *file_of(fz_context *ctx, pdf_document *pdf, const char *key, pdf_obj *tree, pdf_page **page, pdf_annot **annot)
{
	pdf_obj *fs = NULL;
	int number, id;
	if (!strncmp(key, "n:", 2))
		fs = pdf_dict_gets(ctx, tree, key + 2);
	else if (sscanf(key, "a:%d:%d", &number, &id) == 2 && number >= 0 && number < pdf_count_pages(ctx, pdf))
	{
		*page = pdf_load_page(ctx, pdf, number);
		*annot = ely_find_annot(ctx, *page, id, 0);
		if (pdf_annot_type(ctx, *annot) == PDF_ANNOT_FILE_ATTACHMENT)
			fs = pdf_annot_filespec(ctx, *annot);
	}
	if (!pdf_is_embedded_file(ctx, fs))
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "no attachment %s", key);
	return fs;
}

/* An embedded file's bytes. */
int ely_attachment(fz_context *ctx, fz_document *doc, const char *key, unsigned char **data, size_t *len)
{
	pdf_obj *tree = NULL;
	pdf_page *page = NULL;
	fz_buffer *buf = NULL;
	fz_var(tree);
	fz_var(page);
	fz_var(buf);
	fz_try(ctx)
	{
		pdf_document *pdf = ely_pdf_of(ctx, doc);
		pdf_annot *annot = NULL;
		tree = pdf_load_name_tree(ctx, pdf, PDF_NAME(EmbeddedFiles));
		unsigned char *bytes;
		buf = pdf_load_embedded_file_contents(ctx, file_of(ctx, pdf, key, tree, &page, &annot));
		/* A copy: the document may hold this buffer. */
		*len = fz_buffer_storage(ctx, buf, &bytes);
		*data = fz_malloc(ctx, *len ? *len : 1);
		memcpy(*data, bytes, *len);
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, buf);
		fz_drop_page(ctx, (fz_page *)page);
		pdf_drop_obj(ctx, tree);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* Writes `tree` back flat, its names in order. */
static void put_tree(fz_context *ctx, pdf_document *pdf, pdf_obj *tree)
{
	pdf_obj *root = pdf_dict_get(ctx, pdf_trailer(ctx, pdf), PDF_NAME(Root));
	pdf_obj *names = pdf_dict_get(ctx, root, PDF_NAME(Names)), *list;
	const char **sorted = NULL;
	int i, n;
	fz_var(sorted);
	if (!names)
		names = pdf_dict_put_dict(ctx, root, PDF_NAME(Names), 1);
	list = pdf_dict_put_array(ctx, pdf_dict_put_dict(ctx, names, PDF_NAME(EmbeddedFiles), 1), PDF_NAME(Names), 8);
	fz_try(ctx)
	{
		sorted = names_of(ctx, tree, &n);
		for (i = 0; i < n; i++)
		{
			pdf_array_push_text_string(ctx, list, sorted[i]);
			pdf_array_push(ctx, list, pdf_dict_gets(ctx, tree, sorted[i]));
		}
	}
	fz_always(ctx)
		fz_free(ctx, sorted);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* Embeds the file at `path` as `name`, in one step. */
int ely_attach(fz_context *ctx, fz_document *doc, const char *name, const char *path)
{
	pdf_document *pdf = NULL;
	pdf_obj *tree = NULL, *fs = NULL;
	fz_buffer *contents = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(tree);
	fz_var(fs);
	fz_var(contents);
	fz_var(begun);
	fz_try(ctx)
	{
		pdf = ely_pdf_of(ctx, doc);
		if (!name[0])
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "an attachment with no name");
		tree = pdf_load_name_tree(ctx, pdf, PDF_NAME(EmbeddedFiles));
		if (!tree)
			tree = pdf_new_dict(ctx, pdf, 1);
		if (pdf_dict_gets(ctx, tree, name))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "an attachment named %s exists", name);
		contents = fz_read_file(ctx, path);
		pdf_begin_operation(ctx, pdf, "Attach a file");
		begun = 1;
		fs = pdf_add_embedded_file(ctx, pdf, name, NULL, contents, -1, -1, 1);
		pdf_dict_puts(ctx, tree, name, fs);
		put_tree(ctx, pdf, tree);
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		pdf_drop_obj(ctx, fs);
		fz_drop_buffer(ctx, contents);
		pdf_drop_obj(ctx, tree);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

/* Removes the file `key`: a tree entry or its annotation. */
int ely_detach(fz_context *ctx, fz_document *doc, const char *key)
{
	pdf_document *pdf = NULL;
	pdf_obj *tree = NULL;
	pdf_page *page = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(tree);
	fz_var(page);
	fz_var(begun);
	fz_try(ctx)
	{
		pdf_annot *annot = NULL;
		pdf = ely_pdf_of(ctx, doc);
		tree = pdf_load_name_tree(ctx, pdf, PDF_NAME(EmbeddedFiles));
		file_of(ctx, pdf, key, tree, &page, &annot);
		pdf_begin_operation(ctx, pdf, "Remove an attachment");
		begun = 1;
		if (annot)
			pdf_delete_annot(ctx, page, annot);
		else
		{
			pdf_dict_dels(ctx, tree, key + 2);
			put_tree(ctx, pdf, tree);
		}
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		fz_drop_page(ctx, (fz_page *)page);
		pdf_drop_obj(ctx, tree);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}
