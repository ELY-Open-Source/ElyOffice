/* Bookmarks and links: change, list (PDF-07). */

#include <string.h>

#include "ely_shim.h"

/* A page point as a link target. */
static char *target(fz_context *ctx, fz_document *doc, fz_json *spec)
{
	fz_json *page = ely_member(ctx, spec, "page", FZ_JSON_NUMBER);
	fz_json *y = ely_member(ctx, spec, "y", FZ_JSON_NUMBER);
	if (!page || page->u.number < 0 || page->u.number >= fz_count_pages(ctx, doc))
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a target with no page");
	return fz_format_link_uri(ctx, doc, fz_make_link_dest_xyz(0, (int)page->u.number, 0, y ? (float)y->u.number : 0, 0));
}

/* Walks `path`; the last step may land past the end. */
static int walk(fz_context *ctx, fz_outline_iterator *iter, fz_json *path)
{
	int i, k, n = fz_json_array_length(ctx, path), at = FZ_OUTLINE_ITERATOR_AT_ITEM;
	if (n == 0)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "an empty bookmark path");
	for (i = 0; i < n; i++)
	{
		int steps = (int)ely_number(ctx, path, i);
		if (i > 0)
		{
			at = fz_outline_iterator_down(ctx, iter);
			if (at < 0)
				fz_throw(ctx, FZ_ERROR_ARGUMENT, "no bookmark at that path");
		}
		at = fz_outline_iterator_item(ctx, iter) ? FZ_OUTLINE_ITERATOR_AT_ITEM : FZ_OUTLINE_ITERATOR_AT_EMPTY;
		for (k = 0; k < steps; k++)
			if (at != FZ_OUTLINE_ITERATOR_AT_ITEM || (at = fz_outline_iterator_next(ctx, iter)) < 0)
				fz_throw(ctx, FZ_ERROR_ARGUMENT, "no bookmark at that path");
		if (i < n - 1 && at != FZ_OUTLINE_ITERATOR_AT_ITEM)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "no bookmark at that path");
	}
	return at;
}

/* Adds, renames, retargets or deletes one bookmark, in one step. */
int ely_outline_edit(fz_context *ctx, fz_document *doc, const char *json)
{
	pdf_document *pdf = NULL;
	fz_outline_iterator *iter = NULL;
	fz_pool *pool = NULL;
	char *uri = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(iter);
	fz_var(pool);
	fz_var(uri);
	fz_var(begun);
	fz_try(ctx)
	{
		fz_json *spec, *op, *path, *title, *expect;
		fz_outline_item *now, item = { 0 };
		int at;
		pdf = ely_pdf_of(ctx, doc);
		pool = fz_new_pool(ctx);
		spec = fz_parse_json(ctx, pool, json);
		op = ely_member(ctx, spec, "op", FZ_JSON_STRING);
		path = ely_member(ctx, spec, "path", FZ_JSON_ARRAY);
		title = ely_member(ctx, spec, "title", FZ_JSON_STRING);
		expect = ely_member(ctx, spec, "expect", FZ_JSON_STRING);
		if (!op || !path || (strcmp(op->u.string, "add") && strcmp(op->u.string, "edit") && strcmp(op->u.string, "delete")))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a bad bookmark change");
		if (title && !title->u.string[0])
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a bookmark with no title");
		if (!strcmp(op->u.string, "add") && !title)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a new bookmark with no title");
		if (strcmp(op->u.string, "add") && !expect)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a change that does not name its bookmark");
		if (ely_member(ctx, spec, "page", FZ_JSON_NUMBER))
			uri = target(ctx, doc, spec);
		else if (!strcmp(op->u.string, "add"))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a new bookmark with no target");
		pdf_begin_operation(ctx, pdf, "Change bookmarks");
		begun = 1;
		iter = fz_new_outline_iterator(ctx, doc);
		at = walk(ctx, iter, path);
		if (!strcmp(op->u.string, "add"))
		{
			item.title = (char *)title->u.string;
			item.uri = uri;
			fz_outline_iterator_insert(ctx, iter, &item);
		}
		else
		{
			now = at == FZ_OUTLINE_ITERATOR_AT_ITEM ? fz_outline_iterator_item(ctx, iter) : NULL;
			if (!now || strcmp(now->title ? now->title : "", expect->u.string))
				fz_throw(ctx, FZ_ERROR_ARGUMENT, "the bookmark changed meanwhile");
			if (!strcmp(op->u.string, "delete"))
				fz_outline_iterator_delete(ctx, iter);
			else
			{
				item = *now;
				if (title)
					item.title = (char *)title->u.string;
				if (uri)
					item.uri = uri;
				fz_outline_iterator_update(ctx, iter, &item);
			}
		}
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		fz_free(ctx, uri);
		fz_drop_outline_iterator(ctx, iter);
		fz_drop_pool(ctx, pool);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

/* The page's links: box, address, and page if inside. */
int ely_links(fz_context *ctx, fz_document *doc, int number, unsigned char **data, size_t *len)
{
	fz_page *page = NULL;
	fz_link *links = NULL, *link;
	fz_buffer *buf = NULL;
	fz_output *out = NULL;
	fz_var(page);
	fz_var(links);
	fz_var(buf);
	fz_var(out);
	fz_try(ctx)
	{
		page = fz_load_page(ctx, doc, number);
		links = fz_load_links(ctx, page);
		buf = fz_new_buffer(ctx, 256);
		out = fz_new_output_with_buffer(ctx, buf);
		fz_write_byte(ctx, out, '[');
		for (link = links; link; link = link->next)
		{
			int to = -1;
			float x = 0, y = 0;
			if (!fz_is_external_link(ctx, link->uri))
				to = fz_page_number_from_location(ctx, doc, fz_resolve_link(ctx, doc, link->uri, &x, &y));
			fz_write_printf(ctx, out, "%s{\"rect\":[%g,%g,%g,%g],\"uri\":", link == links ? "" : ",",
				link->rect.x0, link->rect.y0, link->rect.x1, link->rect.y1);
			ely_json_string(ctx, out, link->uri ? link->uri : "");
			fz_write_printf(ctx, out, ",\"page\":%d,\"y\":%g}", to, y);
		}
		fz_write_byte(ctx, out, ']');
		fz_close_output(ctx, out);
		*len = fz_buffer_extract(ctx, buf, data);
	}
	fz_always(ctx)
	{
		fz_drop_output(ctx, out);
		fz_drop_buffer(ctx, buf);
		fz_drop_link(ctx, links);
		fz_drop_page(ctx, page);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* Only web and mail addresses leave the document. */
static int outward(const char *uri)
{
	return !strncmp(uri, "https://", 8) || !strncmp(uri, "http://", 7) || !strncmp(uri, "mailto:", 7);
}

/* Adds, retargets or deletes one link, in one step. */
int ely_link_edit(fz_context *ctx, fz_document *doc, int number, const char *json)
{
	pdf_document *pdf = NULL;
	pdf_page *page = NULL;
	fz_pool *pool = NULL;
	char *uri = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(page);
	fz_var(pool);
	fz_var(uri);
	fz_var(begun);
	fz_try(ctx)
	{
		fz_json *spec, *op, *index, *expect, *web;
		fz_link *link = NULL;
		fz_rect box = fz_empty_rect;
		int i;
		pdf = ely_pdf_of(ctx, doc);
		page = pdf_load_page(ctx, pdf, number);
		pool = fz_new_pool(ctx);
		spec = fz_parse_json(ctx, pool, json);
		op = ely_member(ctx, spec, "op", FZ_JSON_STRING);
		index = ely_member(ctx, spec, "index", FZ_JSON_NUMBER);
		expect = ely_member(ctx, spec, "expect", FZ_JSON_STRING);
		web = ely_member(ctx, spec, "uri", FZ_JSON_STRING);
		if (!op || (strcmp(op->u.string, "add") && strcmp(op->u.string, "edit") && strcmp(op->u.string, "delete")))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a bad link change");
		if (web && !outward(web->u.string))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "only web and mail addresses: %s", web->u.string);
		if (web)
			uri = fz_strdup(ctx, web->u.string);
		else if (ely_member(ctx, spec, "page", FZ_JSON_NUMBER))
			uri = target(ctx, doc, spec);
		else if (strcmp(op->u.string, "delete"))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a link with no target");
		if (strcmp(op->u.string, "add"))
		{
			if (!index || !expect)
				fz_throw(ctx, FZ_ERROR_ARGUMENT, "a change that does not name its link");
			for (link = page->links, i = 0; link && i < (int)index->u.number; link = link->next)
				i++;
			if (!link || strcmp(link->uri ? link->uri : "", expect->u.string))
				fz_throw(ctx, FZ_ERROR_ARGUMENT, "the link changed meanwhile");
		}
		if (!strcmp(op->u.string, "add"))
			box = ely_rect(ctx, ely_member(ctx, spec, "rect", FZ_JSON_ARRAY), 2);
		pdf_begin_operation(ctx, pdf, "Change links");
		begun = 1;
		if (!strcmp(op->u.string, "add"))
			fz_drop_link(ctx, pdf_create_link(ctx, page, box, uri));
		else if (!strcmp(op->u.string, "delete"))
			pdf_delete_link(ctx, page, link);
		else
			fz_set_link_uri(ctx, link, uri);
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		fz_free(ctx, uri);
		fz_drop_pool(ctx, pool);
		fz_drop_page(ctx, (fz_page *)page);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}
