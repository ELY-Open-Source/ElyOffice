/* Images and form XObjects: list, move, scale, swap, add (PDF-04). */

#include <math.h>

#include "ely_shim.h"

/* One drawn object: its kind and where it lands. */
typedef struct
{
	int form;
	fz_matrix ctm;
	fz_rect rect;
} drawn;

/* Counts top-level draws, following q, Q and cm. */
typedef struct
{
	pdf_processor super;
	fz_matrix ctm;
	fz_matrix *stack;
	int depth, room;
	fz_matrix page;
	drawn *found;
	int n, cap;
} inventory;

static void inv_q(fz_context *ctx, pdf_processor *proc)
{
	inventory *inv = (inventory *)proc;
	if (inv->depth == inv->room)
	{
		inv->room = inv->room * 2 + 8;
		inv->stack = fz_realloc_array(ctx, inv->stack, inv->room, fz_matrix);
	}
	inv->stack[inv->depth++] = inv->ctm;
}

static void inv_Q(fz_context *ctx, pdf_processor *proc)
{
	inventory *inv = (inventory *)proc;
	(void)ctx;
	if (inv->depth > 0)
		inv->ctm = inv->stack[--inv->depth];
}

static void inv_cm(fz_context *ctx, pdf_processor *proc, float a, float b, float c, float d, float e, float f)
{
	inventory *inv = (inventory *)proc;
	(void)ctx;
	inv->ctm = fz_concat(fz_make_matrix(a, b, c, d, e, f), inv->ctm);
}

static void found(fz_context *ctx, inventory *inv, int form, fz_rect box)
{
	if (inv->n == inv->cap)
	{
		inv->cap = inv->cap * 2 + 8;
		inv->found = fz_realloc_array(ctx, inv->found, inv->cap, drawn);
	}
	inv->found[inv->n].form = form;
	inv->found[inv->n].ctm = inv->ctm;
	inv->found[inv->n++].rect = fz_transform_rect(box, fz_concat(inv->ctm, inv->page));
}

static void inv_image(fz_context *ctx, pdf_processor *proc, const char *name, fz_image *image)
{
	(void)name;
	(void)image;
	found(ctx, (inventory *)proc, 0, fz_unit_rect);
}

static void inv_form(fz_context *ctx, pdf_processor *proc, const char *name, pdf_obj *form)
{
	fz_rect box = fz_transform_rect(pdf_xobject_bbox(ctx, form), pdf_xobject_matrix(ctx, form));
	(void)name;
	found(ctx, (inventory *)proc, 1, box);
}

/* The page's top-level draws, in content order. */
static int take_inventory(fz_context *ctx, pdf_document *pdf, pdf_page *page, drawn **out)
{
	inventory *inv = pdf_new_processor(ctx, sizeof *inv);
	int n = 0;
	fz_try(ctx)
	{
		inv->super.op_q = inv_q;
		inv->super.op_Q = inv_Q;
		inv->super.op_cm = inv_cm;
		inv->super.op_Do_image = inv_image;
		inv->super.op_Do_form = inv_form;
		inv->ctm = fz_identity;
		pdf_page_transform(ctx, page, NULL, &inv->page);
		pdf_process_contents(ctx, &inv->super, pdf, pdf_page_resources(ctx, page), pdf_page_contents(ctx, page), NULL, NULL);
		pdf_close_processor(ctx, &inv->super);
		*out = inv->found;
		n = inv->n;
		inv->found = NULL;
	}
	fz_always(ctx)
	{
		fz_free(ctx, inv->stack);
		fz_free(ctx, inv->found);
		pdf_drop_processor(ctx, &inv->super);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
	return n;
}

/* JSON of the page's images and forms. */
int ely_objects(fz_context *ctx, fz_document *doc, int number, unsigned char **data, size_t *len)
{
	pdf_page *page = NULL;
	drawn *list = NULL;
	fz_buffer *buf = NULL;
	fz_var(page);
	fz_var(list);
	fz_var(buf);
	fz_try(ctx)
	{
		pdf_document *pdf = ely_pdf_of(ctx, doc);
		int i, n;
		page = pdf_load_page(ctx, pdf, number);
		n = take_inventory(ctx, pdf, page, &list);
		buf = fz_new_buffer(ctx, 256);
		fz_append_byte(ctx, buf, '[');
		for (i = 0; i < n; i++)
			fz_append_printf(ctx, buf, "%s{\"kind\":\"%s\",\"rect\":[%g,%g,%g,%g]}", i ? "," : "",
				list[i].form ? "form" : "image", list[i].rect.x0, list[i].rect.y0, list[i].rect.x1, list[i].rect.y1);
		fz_append_byte(ctx, buf, ']');
		*len = fz_buffer_extract(ctx, buf, data);
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, buf);
		fz_free(ctx, list);
		fz_drop_page(ctx, (fz_page *)page);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* What the rewrite does at draw `target`. */
typedef struct
{
	int target, seen, drop;
	fz_matrix x;
	const char *rename;
	pdf_processor *out;
	void (*image)(fz_context *, pdf_processor *, const char *, fz_image *);
	void (*form)(fz_context *, pdf_processor *, const char *, pdf_obj *);
} hook;

/* Wraps the chosen draw in `x`, renames or drops it. */
static int at_target(fz_context *ctx, hook *h, const char **name)
{
	if (h->seen++ != h->target)
		return 0;
	if (!h->drop)
	{
		h->out->op_q(ctx, h->out);
		h->out->op_cm(ctx, h->out, h->x.a, h->x.b, h->x.c, h->x.d, h->x.e, h->x.f);
		if (h->rename)
			*name = h->rename;
	}
	return 1;
}

static void hook_image(fz_context *ctx, pdf_processor *proc, const char *name, fz_image *image)
{
	hook *h = fz_user_context(ctx);
	int hit = at_target(ctx, h, &name);
	if (!(hit && h->drop))
		h->image(ctx, proc, name, image);
	if (hit && !h->drop)
		h->out->op_Q(ctx, h->out);
}

static void hook_form(fz_context *ctx, pdf_processor *proc, const char *name, pdf_obj *form)
{
	hook *h = fz_user_context(ctx);
	int hit = at_target(ctx, h, &name);
	if (!(hit && h->drop))
		h->form(ctx, proc, name, form);
	if (hit && !h->drop)
		h->out->op_Q(ctx, h->out);
}

/* Rewrites the page's content with `h` at its target draw. */
static void rewrite(fz_context *ctx, pdf_document *pdf, pdf_page *page, hook *h)
{
	fz_buffer *buf = fz_new_buffer(ctx, 1024);
	pdf_processor *out = NULL;
	void *before = fz_user_context(ctx);
	fz_var(out);
	fz_try(ctx)
	{
		out = pdf_new_buffer_processor(ctx, buf, 0, 0);
		h->out = out;
		h->image = out->op_Do_image;
		h->form = out->op_Do_form;
		out->op_Do_image = hook_image;
		out->op_Do_form = hook_form;
		fz_set_user_context(ctx, h);
		pdf_process_contents(ctx, out, pdf, pdf_page_resources(ctx, page), pdf_page_contents(ctx, page), NULL, NULL);
		pdf_close_processor(ctx, out);
		if (h->seen <= h->target)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "no drawn object %d", h->target);
		pdf_dict_put_drop(ctx, page->obj, PDF_NAME(Contents), pdf_add_stream(ctx, pdf, buf, NULL, 0));
	}
	fz_always(ctx)
	{
		fz_set_user_context(ctx, before);
		pdf_drop_processor(ctx, out);
		fz_drop_buffer(ctx, buf);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* The draw's matrix in page space. */
static fz_matrix placed_at(fz_context *ctx, pdf_document *pdf, pdf_page *page, int index, fz_rect *rect)
{
	drawn *list = NULL;
	fz_matrix at = fz_identity, page_ctm;
	fz_var(list);
	fz_try(ctx)
	{
		int n = take_inventory(ctx, pdf, page, &list);
		if (index < 0 || index >= n)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "no drawn object %d", index);
		pdf_page_transform(ctx, page, NULL, &page_ctm);
		at = fz_concat(list[index].ctm, page_ctm);
		*rect = list[index].rect;
	}
	fz_always(ctx)
		fz_free(ctx, list);
	fz_catch(ctx)
		fz_rethrow(ctx);
	return at;
}

/* Draw `index` moved by page-space `t`, swapped, or dropped. */
int ely_place(fz_context *ctx, fz_document *doc, int number, int index, const float t[6], int drop, const char *path)
{
	pdf_document *pdf = NULL;
	pdf_page *page = NULL;
	fz_image *image = NULL;
	pdf_obj *ref = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(page);
	fz_var(image);
	fz_var(ref);
	fz_var(begun);
	fz_try(ctx)
	{
		hook h = { index, 0, drop, fz_identity, NULL, NULL, NULL, NULL };
		fz_matrix change = fz_make_matrix(t[0], t[1], t[2], t[3], t[4], t[5]), at;
		fz_rect rect;
		char name[32];
		pdf = ely_pdf_of(ctx, doc);
		page = pdf_load_page(ctx, pdf, number);
		pdf_begin_operation(ctx, pdf, drop ? "Delete an object" : path ? "Replace an image" : "Move an object");
		begun = 1;
		at = placed_at(ctx, pdf, page, index, &rect);
		if (path)
		{
			/* The new picture fits inside the old one's box. */
			float box = (rect.x1 - rect.x0) / (rect.y1 - rect.y0), own;
			float cx = (rect.x0 + rect.x1) / 2, cy = (rect.y0 + rect.y1) / 2;
			image = fz_new_image_from_file(ctx, path);
			own = (float)image->w / image->h;
			change = fz_concat(fz_concat(fz_translate(-cx, -cy), fz_scale(fminf(1, own / box), fminf(1, box / own))), fz_translate(cx, cy));
			ref = pdf_add_image(ctx, pdf, image);
			h.rename = ely_add_resource(ctx, page->obj, PDF_NAME(XObject), "ElyIm", ref, name, sizeof name);
		}
		if (fz_matrix_expansion(change) < 0.001f)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a change that leaves nothing");
		h.x = fz_concat(fz_concat(at, change), fz_invert_matrix(at));
		rewrite(ctx, pdf, page, &h);
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		pdf_drop_obj(ctx, ref);
		fz_drop_image(ctx, image);
		fz_drop_page(ctx, (fz_page *)page);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

/* An image file drawn into page-space `rect`, in one step. */
int ely_add_image(fz_context *ctx, fz_document *doc, int number, const char *path, const float rect[4])
{
	pdf_document *pdf = NULL;
	pdf_page *page = NULL;
	fz_image *image = NULL;
	pdf_obj *ref = NULL;
	fz_buffer *buf = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(page);
	fz_var(image);
	fz_var(ref);
	fz_var(buf);
	fz_var(begun);
	fz_try(ctx)
	{
		fz_matrix m;
		char name[32];
		pdf = ely_pdf_of(ctx, doc);
		page = pdf_load_page(ctx, pdf, number);
		if (rect[2] - rect[0] < 1 || rect[3] - rect[1] < 1)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "an image box of nothing");
		image = fz_new_image_from_file(ctx, path);
		pdf_begin_operation(ctx, pdf, "Add an image");
		begun = 1;
		ref = pdf_add_image(ctx, pdf, image);
		ely_add_resource(ctx, page->obj, PDF_NAME(XObject), "ElyIm", ref, name, sizeof name);
		/* The unit square, bottom left first, onto the box. */
		m = fz_concat(fz_make_matrix(rect[2] - rect[0], 0, 0, rect[1] - rect[3], rect[0], rect[3]), ely_to_user(ctx, page));
		buf = fz_new_buffer(ctx, 128);
		fz_append_printf(ctx, buf, "q %g %g %g %g %g %g cm /%s Do Q\n", m.a, m.b, m.c, m.d, m.e, m.f, name);
		ely_append_content(ctx, pdf, page->obj, buf);
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, buf);
		pdf_drop_obj(ctx, ref);
		fz_drop_image(ctx, image);
		fz_drop_page(ctx, (fz_page *)page);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}
