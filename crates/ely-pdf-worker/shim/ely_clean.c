/* Cleaning into a new file, counted before and after (PDF-10). */

#include <string.h>

#include "ely_shim.h"

enum { CLEAN_META = 1, CLEAN_ANNOTS = 2, CLEAN_FILES = 4, CLEAN_HIDDEN = 8, CLEAN_ALL = 15 };

/* What cleaning reaches, counted. */
typedef struct
{
	int meta, annots, files, hidden_pages, scripts, hidden_annots;
} reach;

/* The filter running a page, and what it drops. */
typedef struct
{
	pdf_processor *top;
	int dropped;
} hiding;

static pdf_processor *hide_factory(fz_context *ctx, pdf_document *doc, pdf_processor *chain, int struct_parents,
	fz_matrix transform, pdf_filter_options *options, void *sopts)
{
	hiding *h = ((pdf_sanitize_filter_options *)sopts)->opaque;
	h->top = pdf_new_sanitize_filter(ctx, doc, chain, struct_parents, transform, options, sopts);
	/* Layers count as hidden only for a viewer's usage. */
	h->top->usage = "View";
	return h->top;
}

/* Unseen text: modes 3 and 7, or zero opacity. */
static int hide_text(fz_context *ctx, void *opaque, int *ucs, int len, fz_matrix trm, fz_matrix ctm, fz_rect bbox,
	int tr, float ca, float CA)
{
	hiding *h = opaque;
	int fills = tr == 0 || tr == 2 || tr == 4 || tr == 6, strokes = tr == 1 || tr == 2 || tr == 5 || tr == 6;
	int drop = h->top->hidden > 0 || tr == 3 || tr == 7 || ((!fills || ca == 0) && (!strokes || CA == 0));
	(void)ctx, (void)ucs, (void)len, (void)trm, (void)ctm, (void)bbox;
	h->dropped += drop;
	return drop;
}

/* Drawing inside a layer that is off. */
static int hide_cull(fz_context *ctx, void *opaque, fz_rect bbox, fz_cull_type type)
{
	hiding *h = opaque;
	(void)ctx, (void)bbox, (void)type;
	h->dropped += h->top->hidden > 0;
	return h->top->hidden > 0;
}

/* Drops hidden text and hidden layers from `page`; how many. */
static int drop_hidden(fz_context *ctx, pdf_document *pdf, pdf_page *page)
{
	hiding h = { NULL, 0 };
	pdf_sanitize_filter_options sopts = { 0 };
	pdf_filter_factory list[2] = { { hide_factory, NULL }, { NULL, NULL } };
	pdf_filter_options opts = { 0 };
	sopts.opaque = &h;
	sopts.text_filter = hide_text;
	sopts.culler = hide_cull;
	list[0].options = &sopts;
	opts.recurse = 1;
	opts.filters = list;
	pdf_filter_page_contents(ctx, pdf, page, &opts);
	if (h.dropped)
		ely_drop_alt(ctx, pdf, page);
	return h.dropped;
}

static int is_script(fz_context *ctx, pdf_obj *action)
{
	return pdf_is_dict(ctx, action) &&
		(pdf_name_eq(ctx, pdf_dict_get(ctx, action, PDF_NAME(S)), PDF_NAME(JavaScript)) || pdf_dict_get(ctx, action, PDF_NAME(JS)));
}

/* `action` without its scripts, their Next chains kept. */
static pdf_obj *spliced(fz_context *ctx, pdf_document *pdf, pdf_obj *action, int *n)
{
	pdf_obj *out = NULL, *each = NULL;
	int i, k;
	fz_var(each);
	if (is_script(ctx, action))
	{
		if (pdf_mark_obj(ctx, action))
			return NULL;
		(*n)++;
		fz_var(out);
		fz_try(ctx)
			out = spliced(ctx, pdf, pdf_dict_get(ctx, action, PDF_NAME(Next)), n);
		fz_always(ctx)
			pdf_unmark_obj(ctx, action);
		fz_catch(ctx)
			fz_rethrow(ctx);
		return out;
	}
	if (!pdf_is_array(ctx, action))
		return pdf_keep_obj(ctx, action);
	out = pdf_new_array(ctx, pdf, pdf_array_len(ctx, action));
	fz_try(ctx)
		for (i = 0; i < pdf_array_len(ctx, action); i++)
		{
			each = spliced(ctx, pdf, pdf_array_get(ctx, action, i), n);
			if (pdf_is_array(ctx, each))
				for (k = 0; k < pdf_array_len(ctx, each); k++)
					pdf_array_push(ctx, out, pdf_array_get(ctx, each, k));
			else if (each)
				pdf_array_push(ctx, out, each);
			pdf_drop_obj(ctx, each);
			each = NULL;
		}
	fz_always(ctx)
		pdf_drop_obj(ctx, each);
	fz_catch(ctx)
	{
		pdf_drop_obj(ctx, out);
		fz_rethrow(ctx);
	}
	return out;
}

/* Splices scripts out of `dict[key]`; how many. */
static int splice_key(fz_context *ctx, pdf_document *pdf, pdf_obj *dict, const char *key)
{
	pdf_obj *now = pdf_dict_gets(ctx, dict, key), *kept;
	int n = 0;
	if (!now)
		return 0;
	kept = spliced(ctx, pdf, now, &n);
	if (n == 0)
		pdf_drop_obj(ctx, kept);
	else if (kept && !(pdf_is_array(ctx, kept) && pdf_array_len(ctx, kept) == 0))
		pdf_dict_puts_drop(ctx, dict, key, kept);
	else
	{
		pdf_drop_obj(ctx, kept);
		pdf_dict_dels(ctx, dict, key);
	}
	return n;
}

static int drop_key(fz_context *ctx, pdf_obj *dict, const char *key)
{
	int had = pdf_dict_gets(ctx, dict, key) != NULL;
	pdf_dict_dels(ctx, dict, key);
	return had;
}

/* Cleans `obj` and the direct values inside it. */
static void clean_obj(fz_context *ctx, pdf_document *pdf, pdf_obj *obj, int mask, reach *r)
{
	pdf_obj *aa;
	int i;
	if (pdf_is_dict(ctx, obj))
	{
		if (mask & CLEAN_META)
			r->meta += drop_key(ctx, obj, "Metadata") + drop_key(ctx, obj, "PieceInfo");
		if (mask & CLEAN_FILES)
		{
			r->files += drop_key(ctx, obj, "EF");
			drop_key(ctx, obj, "RF");
			drop_key(ctx, obj, "AF");
		}
		if (mask & CLEAN_HIDDEN)
		{
			r->scripts += splice_key(ctx, pdf, obj, "A") + splice_key(ctx, pdf, obj, "OpenAction") +
				splice_key(ctx, pdf, obj, "Next");
			aa = pdf_dict_get(ctx, obj, PDF_NAME(AA));
			for (i = pdf_dict_len(ctx, aa) - 1; i >= 0; i--)
				r->scripts += splice_key(ctx, pdf, aa, pdf_to_name(ctx, pdf_dict_get_key(ctx, aa, i)));
			if (pdf_is_dict(ctx, aa) && pdf_dict_len(ctx, aa) == 0)
				pdf_dict_del(ctx, obj, PDF_NAME(AA));
		}
		for (i = 0; i < pdf_dict_len(ctx, obj); i++)
			if (!pdf_is_indirect(ctx, pdf_dict_get_val(ctx, obj, i)))
				clean_obj(ctx, pdf, pdf_dict_get_val(ctx, obj, i), mask, r);
	}
	else if (pdf_is_array(ctx, obj))
		for (i = 0; i < pdf_array_len(ctx, obj); i++)
			if (!pdf_is_indirect(ctx, pdf_array_get(ctx, obj, i)))
				clean_obj(ctx, pdf, pdf_array_get(ctx, obj, i), mask, r);
}

/* Every object: metadata, embedded bytes, scripts. */
static void clean_all(fz_context *ctx, pdf_document *pdf, int mask, reach *r)
{
	int i;
	for (i = 1; i < pdf_xref_len(ctx, pdf); i++)
	{
		pdf_obj *obj = pdf_new_indirect(ctx, pdf, i, 0);
		fz_try(ctx)
			clean_obj(ctx, pdf, pdf_resolve_indirect(ctx, obj), mask, r);
		fz_always(ctx)
			pdf_drop_obj(ctx, obj);
		fz_catch(ctx)
			fz_rethrow(ctx);
	}
}

static int unseen(fz_context *ctx, pdf_obj *annot)
{
	return (pdf_dict_get_int(ctx, annot, PDF_NAME(F)) & (PDF_ANNOT_IS_HIDDEN | PDF_ANNOT_IS_INVISIBLE)) != 0;
}

/* Hidden fields go with their values. */
static int drop_hidden_widgets(fz_context *ctx, pdf_page *page)
{
	pdf_annot *widget;
	int n = 0;
restart:
	for (widget = pdf_first_widget(ctx, page); widget; widget = pdf_next_widget(ctx, widget))
		if (unseen(ctx, pdf_annot_obj(ctx, widget)))
		{
			pdf_obj *field = pdf_keep_obj(ctx, pdf_annot_obj(ctx, widget));
			fz_var(widget);
			fz_var(n);
			fz_try(ctx)
			{
				pdf_delete_annot(ctx, page, widget);
				ely_unlink_field(ctx, page->doc, field);
			}
			fz_always(ctx)
				pdf_drop_obj(ctx, field);
			fz_catch(ctx)
				fz_rethrow(ctx);
			n++;
			goto restart;
		}
	return n;
}

/* Links are not in MuPDF's annotation list. */
static int drop_hidden_links(fz_context *ctx, pdf_page *page)
{
	pdf_obj *annots = pdf_dict_get(ctx, page->obj, PDF_NAME(Annots));
	int i, n = 0;
	for (i = pdf_array_len(ctx, annots) - 1; i >= 0; i--)
	{
		pdf_obj *annot = pdf_array_get(ctx, annots, i);
		if (pdf_name_eq(ctx, pdf_dict_get(ctx, annot, PDF_NAME(Subtype)), PDF_NAME(Link)) && unseen(ctx, annot))
		{
			pdf_array_delete(ctx, annots, i);
			n++;
		}
	}
	return n;
}

static void sweep_page(fz_context *ctx, pdf_document *pdf, pdf_page *page, int mask, reach *r)
{
	pdf_annot *annot;
	if (mask & CLEAN_HIDDEN)
		r->hidden_pages += drop_hidden(ctx, pdf, page) > 0;
restart:
	for (annot = pdf_first_annot(ctx, page); annot; annot = pdf_next_annot(ctx, annot))
	{
		int file = pdf_annot_type(ctx, annot) == PDF_ANNOT_FILE_ATTACHMENT;
		int hidden = (pdf_annot_flags(ctx, annot) & (PDF_ANNOT_IS_HIDDEN | PDF_ANNOT_IS_INVISIBLE)) != 0;
		if ((file && (mask & CLEAN_FILES)) || (!file && (mask & CLEAN_ANNOTS)) || (hidden && (mask & CLEAN_HIDDEN)))
		{
			r->hidden_annots += hidden && !file;
			r->annots += !file && !hidden;
			pdf_delete_annot(ctx, page, annot);
			goto restart;
		}
	}
	if (mask & CLEAN_HIDDEN)
		r->hidden_annots += drop_hidden_widgets(ctx, page) + drop_hidden_links(ctx, page);
}

/* Drops what `mask` names from `doc`, counting what was there. */
static reach sweep(fz_context *ctx, fz_document *doc, int mask)
{
	pdf_document *pdf = ely_pdf_of(ctx, doc);
	pdf_obj *trailer = pdf_trailer(ctx, pdf), *root = pdf_dict_get(ctx, trailer, PDF_NAME(Root));
	pdf_obj *names = pdf_dict_get(ctx, root, PDF_NAME(Names)), *tree;
	pdf_page *page = NULL;
	reach r = { 0 };
	int i;
	fz_var(page);
	if (mask & CLEAN_META)
	{
		r.meta = pdf_dict_len(ctx, pdf_dict_get(ctx, trailer, PDF_NAME(Info)));
		pdf_dict_del(ctx, trailer, PDF_NAME(Info));
	}
	if ((mask & CLEAN_FILES) && names)
		pdf_dict_del(ctx, names, PDF_NAME(EmbeddedFiles));
	if (mask & CLEAN_HIDDEN)
	{
		tree = pdf_load_name_tree(ctx, pdf, PDF_NAME(JavaScript));
		r.scripts = pdf_dict_len(ctx, tree);
		pdf_drop_obj(ctx, tree);
		if (names)
			pdf_dict_del(ctx, names, PDF_NAME(JavaScript));
	}
	clean_all(ctx, pdf, mask, &r);
	fz_try(ctx)
	{
		for (i = 0; i < pdf_count_pages(ctx, pdf); i++)
		{
			page = pdf_load_page(ctx, pdf, i);
			sweep_page(ctx, pdf, page, mask, &r);
			fz_drop_page(ctx, (fz_page *)page);
			page = NULL;
		}
	}
	fz_always(ctx)
		fz_drop_page(ctx, (fz_page *)page);
	fz_catch(ctx)
		fz_rethrow(ctx);
	return r;
}

static void write_reach(fz_context *ctx, fz_buffer *buf, reach r)
{
	fz_append_printf(ctx, buf, "{\"meta\":%d,\"annots\":%d,\"files\":%d,\"hidden\":%d,\"scripts\":%d,\"hidden_annots\":%d}",
		r.meta, r.annots, r.files, r.hidden_pages, r.scripts, r.hidden_annots);
}

/* Counts by sweeping a throwaway copy of `pdf`. */
static reach count_copy(fz_context *ctx, pdf_document *pdf, const char *raw, const char *password)
{
	fz_document *copy = NULL;
	reach r;
	fz_var(copy);
	fz_try(ctx)
	{
		pdf_save_snapshot(ctx, pdf, raw);
		copy = fz_open_document(ctx, raw);
		if (fz_needs_password(ctx, copy) && !fz_authenticate_password(ctx, copy, password))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "the copy refused its password");
		r = sweep(ctx, copy, CLEAN_ALL);
	}
	fz_always(ctx)
		fz_drop_document(ctx, copy);
	fz_catch(ctx)
		fz_rethrow(ctx);
	return r;
}

/* What cleaning would reach, as JSON. */
int ely_clean_scan(fz_context *ctx, fz_document *doc, const char *raw, const char *password, unsigned char **data, size_t *len)
{
	fz_buffer *buf = NULL;
	fz_var(buf);
	fz_try(ctx)
	{
		buf = fz_new_buffer(ctx, 128);
		write_reach(ctx, buf, count_copy(ctx, ely_pdf_of(ctx, doc), raw, password));
		*len = fz_buffer_extract(ctx, buf, data);
	}
	fz_always(ctx)
		fz_drop_buffer(ctx, buf);
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

static void apply_clean(fz_context *ctx, fz_document *copy, void *arg)
{
	sweep(ctx, copy, *(int *)arg);
}

/* Cleans `mask` into `out`, then counts what is left. */
int ely_clean(fz_context *ctx, fz_document *doc, int mask, const char *raw, const char *out, const char *password,
	unsigned char **data, size_t *len)
{
	fz_document *written = NULL;
	fz_buffer *buf = NULL;
	fz_var(written);
	fz_var(buf);
	fz_try(ctx)
	{
		reach left;
		if (mask <= 0 || mask > CLEAN_ALL)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "nothing chosen to clean");
		ely_save_copy(ctx, ely_pdf_of(ctx, doc), raw, out, password, NULL, apply_clean, &mask);
		written = fz_open_document(ctx, out);
		if (fz_needs_password(ctx, written) && !fz_authenticate_password(ctx, written, password))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "the written file refused its password");
		left = count_copy(ctx, ely_pdf_of(ctx, written), raw, password);
		buf = fz_new_buffer(ctx, 128);
		write_reach(ctx, buf, left);
		if (((mask & CLEAN_META) && left.meta) || ((mask & CLEAN_ANNOTS) && left.annots) || ((mask & CLEAN_FILES) && left.files) ||
			((mask & CLEAN_HIDDEN) && (left.hidden_pages || left.scripts || left.hidden_annots)))
			fz_throw(ctx, FZ_ERROR_FORMAT, "the written file still holds what was cleaned");
		*len = fz_buffer_extract(ctx, buf, data);
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, buf);
		fz_drop_document(ctx, written);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}
