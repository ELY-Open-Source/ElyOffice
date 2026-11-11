/* Alternate text dropped from redacted pages (PDF-09). */

#include "ely_shim.h"

typedef void (bdc_fn)(fz_context *ctx, pdf_processor *proc, const char *tag, pdf_obj *raw, pdf_obj *cooked);

static void drop_said(fz_context *ctx, pdf_obj *dict)
{
	if (!pdf_is_dict(ctx, dict))
		return;
	pdf_dict_del(ctx, dict, PDF_NAME(ActualText));
	pdf_dict_del(ctx, dict, PDF_NAME(Alt));
	pdf_dict_del(ctx, dict, PDF_NAME(E));
}

static void quiet_bdc(fz_context *ctx, pdf_processor *proc, const char *tag, pdf_obj *raw, pdf_obj *cooked)
{
	bdc_fn **out = fz_user_context(ctx);
	drop_said(ctx, raw);
	drop_said(ctx, cooked);
	(*out)(ctx, proc, tag, raw, cooked);
}

/* Each output stream writes marked content through `quiet_bdc`. */
static pdf_processor *quiet_factory(fz_context *ctx, pdf_document *doc, pdf_processor *chain, int struct_parents,
	fz_matrix transform, pdf_filter_options *options, void *sopts)
{
	bdc_fn **out = fz_user_context(ctx);
	if (chain->op_BDC != quiet_bdc)
	{
		*out = chain->op_BDC;
		chain->op_BDC = quiet_bdc;
	}
	return pdf_new_sanitize_filter(ctx, doc, chain, struct_parents, transform, options, sopts);
}

/* Strips elements on `page`, by own, inherited or kids' `Pg`. */
static int quiet_elem(fz_context *ctx, pdf_obj *elem, int page, int inherited)
{
	pdf_obj *kids;
	int pg, i, on = 0;
	if (!pdf_is_dict(ctx, elem) || pdf_mark_obj(ctx, elem))
		return 0;
	fz_var(on);
	fz_try(ctx)
	{
		pg = pdf_dict_gets(ctx, elem, "Pg") ? pdf_to_num(ctx, pdf_dict_gets(ctx, elem, "Pg")) : inherited;
		on = pg == page;
		kids = pdf_dict_get(ctx, elem, PDF_NAME(K));
		if (pdf_is_array(ctx, kids))
			for (i = 0; i < pdf_array_len(ctx, kids); i++)
				on |= quiet_elem(ctx, pdf_array_get(ctx, kids, i), page, pg);
		else
			on |= quiet_elem(ctx, kids, page, pg);
		if (on)
		{
			drop_said(ctx, elem);
			pdf_dict_del(ctx, elem, PDF_NAME(T));
		}
	}
	fz_always(ctx)
		pdf_unmark_obj(ctx, elem);
	fz_catch(ctx)
		fz_rethrow(ctx);
	return on;
}

/* Drops what marked content says on `page`, forms too. */
void ely_drop_alt(fz_context *ctx, pdf_document *pdf, pdf_page *page)
{
	pdf_sanitize_filter_options sopts = { 0 };
	pdf_filter_factory list[2] = { { quiet_factory, NULL }, { NULL, NULL } };
	pdf_filter_options opts = { 0 };
	void *before = fz_user_context(ctx);
	bdc_fn *out = NULL;
	list[0].options = &sopts;
	opts.recurse = 1;
	opts.filters = list;
	fz_set_user_context(ctx, &out);
	fz_try(ctx)
		pdf_filter_page_contents(ctx, pdf, page, &opts);
	fz_always(ctx)
		fz_set_user_context(ctx, before);
	fz_catch(ctx)
		fz_rethrow(ctx);
	quiet_elem(ctx, pdf_dict_getp(ctx, pdf_trailer(ctx, pdf), "Root/StructTreeRoot"), pdf_to_num(ctx, page->obj), 0);
}

/* Marks the annotations still on a page. */
static unsigned char *live_annots(fz_context *ctx, pdf_document *pdf, int n)
{
	unsigned char *live = fz_calloc(ctx, n, 1);
	int i, k;
	fz_try(ctx)
		for (i = 0; i < pdf_count_pages(ctx, pdf); i++)
		{
			pdf_obj *annots = pdf_dict_get(ctx, pdf_lookup_page_obj(ctx, pdf, i), PDF_NAME(Annots));
			for (k = 0; k < pdf_array_len(ctx, annots); k++)
			{
				int num = pdf_to_num(ctx, pdf_array_get(ctx, annots, k));
				if (num > 0 && num < n)
					live[num] = 1;
			}
		}
	fz_catch(ctx)
	{
		fz_free(ctx, live);
		fz_rethrow(ctx);
	}
	return live;
}

/* An object reference to a removed annotation. */
static int dead_objr(fz_context *ctx, pdf_obj *kid, const unsigned char *live, int n)
{
	pdf_obj *obj;
	int num;
	if (!pdf_is_dict(ctx, kid) || !pdf_name_eq(ctx, pdf_dict_get(ctx, kid, PDF_NAME(Type)), PDF_NAME(OBJR)))
		return 0;
	obj = pdf_dict_gets(ctx, kid, "Obj");
	num = pdf_to_num(ctx, obj);
	/* Forms carry a BBox; annotations a Rect. */
	return num > 0 && num < n && !live[num] && pdf_dict_get(ctx, obj, PDF_NAME(Rect));
}

static void drop_objr(fz_context *ctx, pdf_obj *elem, const unsigned char *live, int n)
{
	pdf_obj *kids;
	int i;
	if (!pdf_is_dict(ctx, elem) || pdf_mark_obj(ctx, elem))
		return;
	fz_try(ctx)
	{
		kids = pdf_dict_get(ctx, elem, PDF_NAME(K));
		if (dead_objr(ctx, kids, live, n))
			pdf_dict_del(ctx, elem, PDF_NAME(K));
		else if (pdf_is_array(ctx, kids))
		{
			for (i = pdf_array_len(ctx, kids) - 1; i >= 0; i--)
				if (dead_objr(ctx, pdf_array_get(ctx, kids, i), live, n))
					pdf_array_delete(ctx, kids, i);
				else
					drop_objr(ctx, pdf_array_get(ctx, kids, i), live, n);
		}
		else
			drop_objr(ctx, kids, live, n);
	}
	fz_always(ctx)
		pdf_unmark_obj(ctx, elem);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* The structure tree lets go of removed annotations. */
void ely_drop_dead_objr(fz_context *ctx, pdf_document *pdf)
{
	pdf_obj *tree = pdf_dict_getp(ctx, pdf_trailer(ctx, pdf), "Root/StructTreeRoot");
	int n = pdf_xref_len(ctx, pdf);
	unsigned char *live;
	if (!tree)
		return;
	live = live_annots(ctx, pdf, n);
	fz_try(ctx)
		drop_objr(ctx, tree, live, n);
	fz_always(ctx)
		fz_free(ctx, live);
	fz_catch(ctx)
		fz_rethrow(ctx);
}
