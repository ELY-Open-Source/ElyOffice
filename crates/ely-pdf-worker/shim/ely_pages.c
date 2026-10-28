/* Page edits, undo and saving; one undo step each. */

#include <stdlib.h>

#include "ely_shim.h"

pdf_document *ely_pdf_of(fz_context *ctx, fz_document *doc)
{
	pdf_document *pdf = pdf_specifics(ctx, doc);
	if (!pdf)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "not a PDF");
	return pdf;
}

/* MuPDF's font and image caches outlive an undo; they go. */
void ely_forget_resources(fz_context *ctx, pdf_document *pdf)
{
	pdf_drop_resource_tables(ctx, pdf);
	pdf->resources.fonts = NULL;
	pdf->resources.colorspaces = NULL;
	pdf->resources.images = NULL;
}

/* Reports the failure; a begun step is rolled back. */
int ely_edit_failed(fz_context *ctx, pdf_document *pdf, int begun)
{
	int code = ely_fail(ctx);
	if (begun)
	{
		fz_try(ctx)
		{
			pdf_abandon_operation(ctx, pdf);
			ely_forget_resources(ctx, pdf);
		}
		fz_catch(ctx)
			ely_fail_also(ctx, "rolling back");
	}
	return code;
}

int ely_journal(fz_context *ctx, fz_document *doc)
{
	fz_try(ctx)
		pdf_enable_journal(ctx, ely_pdf_of(ctx, doc));
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

int ely_undo(fz_context *ctx, fz_document *doc, int redo)
{
	fz_try(ctx)
	{
		pdf_document *pdf = ely_pdf_of(ctx, doc);
		if (redo)
			pdf_redo(ctx, pdf);
		else
			pdf_undo(ctx, pdf);
		ely_forget_resources(ctx, pdf);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* MuPDF's undo position and step count; saves add steps. */
int ely_state(fz_context *ctx, fz_document *doc, int *at, int *count)
{
	fz_try(ctx)
		*at = pdf_undoredo_state(ctx, ely_pdf_of(ctx, doc), count);
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

static int by_number(const void *a, const void *b)
{
	return *(const int *)a - *(const int *)b;
}

/* Object numbers, sorted for lookups. */
typedef struct
{
	int *nums;
	int n;
} gone_set;

static int is_gone(fz_context *ctx, gone_set *gone, pdf_obj *obj)
{
	int num = pdf_to_num(ctx, obj);
	return num && bsearch(&num, gone->nums, gone->n, sizeof(int), by_number) != NULL;
}

/* Deleted pages, and the annotations they held. */
typedef struct
{
	gone_set pages;
	gone_set annots;
} gone_pages;

/* The page a link, bookmark or action points to. */
pdf_obj *ely_target(fz_context *ctx, pdf_document *pdf, pdf_obj *obj)
{
	pdf_obj *dest = pdf_dict_get(ctx, obj, PDF_NAME(Dest));
	if (!dest)
	{
		pdf_obj *action = pdf_dict_get(ctx, obj, PDF_NAME(A));
		if (!pdf_name_eq(ctx, pdf_dict_get(ctx, action, PDF_NAME(S)), PDF_NAME(GoTo)))
			return NULL;
		dest = pdf_dict_get(ctx, action, PDF_NAME(D));
	}
	if (pdf_is_name(ctx, dest) || pdf_is_string(ctx, dest))
		dest = pdf_lookup_dest(ctx, pdf, dest);
	if (pdf_is_dict(ctx, dest))
		dest = pdf_dict_get(ctx, dest, PDF_NAME(D));
	return pdf_is_array(ctx, dest) ? pdf_array_get(ctx, dest, 0) : NULL;
}

/* Bookmarks to gone pages keep their place, lose the jump. */
static void forget_in_outline(fz_context *ctx, pdf_document *pdf, pdf_obj *item, gone_set *gone)
{
	for (; item; item = pdf_dict_get(ctx, item, PDF_NAME(Next)))
	{
		pdf_obj *page = ely_target(ctx, pdf, item);
		if (page && is_gone(ctx, gone, page))
		{
			pdf_dict_del(ctx, item, PDF_NAME(Dest));
			pdf_dict_del(ctx, item, PDF_NAME(A));
		}
		forget_in_outline(ctx, pdf, pdf_dict_get(ctx, item, PDF_NAME(First)), gone);
	}
}

/* Whether a field keeps a widget on a kept page. */
static int prune_field(fz_context *ctx, pdf_obj *field, gone_pages *gone)
{
	pdf_obj *kids = pdf_dict_get(ctx, field, PDF_NAME(Kids));
	int i;
	if (!kids)
	{
		/* A deleted page held it, or its /P names one. */
		pdf_obj *page = pdf_dict_get(ctx, field, PDF_NAME(P));
		if (!pdf_name_eq(ctx, pdf_dict_get(ctx, field, PDF_NAME(Subtype)), PDF_NAME(Widget)))
			return 1;
		return !is_gone(ctx, &gone->annots, field) && !(page && is_gone(ctx, &gone->pages, page));
	}
	for (i = pdf_array_len(ctx, kids) - 1; i >= 0; i--)
		if (!prune_field(ctx, pdf_array_get(ctx, kids, i), gone))
			pdf_array_delete(ctx, kids, i);
	return pdf_array_len(ctx, kids) > 0;
}

/* Drops what pointed into deleted pages. */
static void forget_pages(fz_context *ctx, pdf_document *pdf, gone_pages *gone)
{
	pdf_obj *root = pdf_dict_get(ctx, pdf_trailer(ctx, pdf), PDF_NAME(Root));
	pdf_obj *outlines = pdf_dict_get(ctx, root, PDF_NAME(Outlines));
	pdf_obj *fields = pdf_dict_getp(ctx, root, "AcroForm/Fields");
	int i, j, count = pdf_count_pages(ctx, pdf);
	forget_in_outline(ctx, pdf, pdf_dict_get(ctx, outlines, PDF_NAME(First)), &gone->pages);
	for (i = 0; i < count; i++)
	{
		pdf_obj *annots = pdf_dict_get(ctx, pdf_lookup_page_obj(ctx, pdf, i), PDF_NAME(Annots));
		for (j = pdf_array_len(ctx, annots) - 1; j >= 0; j--)
		{
			pdf_obj *annot = pdf_array_get(ctx, annots, j);
			pdf_obj *page;
			if (!pdf_name_eq(ctx, pdf_dict_get(ctx, annot, PDF_NAME(Subtype)), PDF_NAME(Link)))
				continue;
			page = ely_target(ctx, pdf, annot);
			if (page && is_gone(ctx, &gone->pages, page))
				pdf_array_delete(ctx, annots, j);
		}
	}
	for (i = pdf_array_len(ctx, fields) - 1; i >= 0; i--)
		if (!prune_field(ctx, pdf_array_get(ctx, fields, i), gone))
			pdf_array_delete(ctx, fields, i);
}

/* Checks `pages` are sorted page numbers without repeats. */
static void check_pages(fz_context *ctx, pdf_document *pdf, const int *pages, int n)
{
	int i, count = pdf_count_pages(ctx, pdf);
	if (n <= 0)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "no pages given");
	for (i = 0; i < n; i++)
		if (pages[i] < 0 || pages[i] >= count || (i > 0 && pages[i] <= pages[i - 1]))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "page %d is out of order or range", pages[i]);
}

/* Deletes checked `pages` and what pointed there. */
static void delete_pages(fz_context *ctx, pdf_document *pdf, const int *pages, int n)
{
	gone_pages gone = { { NULL, n }, { NULL, 0 } };
	int i, j, held = 0;
	fz_var(gone.pages.nums);
	fz_var(gone.annots.nums);
	fz_try(ctx)
	{
		gone.pages.nums = fz_malloc_array(ctx, n, int);
		for (i = 0; i < n; i++)
		{
			pdf_obj *page = pdf_lookup_page_obj(ctx, pdf, pages[i]);
			gone.pages.nums[i] = pdf_to_num(ctx, page);
			held += pdf_array_len(ctx, pdf_dict_get(ctx, page, PDF_NAME(Annots)));
		}
		gone.annots.nums = fz_malloc_array(ctx, held + 1, int);
		for (i = 0; i < n; i++)
		{
			pdf_obj *annots = pdf_dict_get(ctx, pdf_lookup_page_obj(ctx, pdf, pages[i]), PDF_NAME(Annots));
			for (j = 0; j < pdf_array_len(ctx, annots); j++)
				gone.annots.nums[gone.annots.n++] = pdf_to_num(ctx, pdf_array_get(ctx, annots, j));
		}
		qsort(gone.pages.nums, n, sizeof(int), by_number);
		qsort(gone.annots.nums, gone.annots.n, sizeof(int), by_number);
		for (i = n - 1; i >= 0; i--)
			pdf_delete_page(ctx, pdf, pages[i]);
		forget_pages(ctx, pdf, &gone);
	}
	fz_always(ctx)
	{
		fz_free(ctx, gone.pages.nums);
		fz_free(ctx, gone.annots.nums);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
}

int ely_delete_pages(fz_context *ctx, fz_document *doc, const int *pages, int n)
{
	pdf_document *pdf = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(begun);
	fz_try(ctx)
	{
		pdf = ely_pdf_of(ctx, doc);
		check_pages(ctx, pdf, pages, n);
		if (n == pdf_count_pages(ctx, pdf))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a PDF keeps at least one page");
		pdf_begin_operation(ctx, pdf, "Delete pages");
		begun = 1;
		delete_pages(ctx, pdf, pages, n);
		pdf_end_operation(ctx, pdf);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

/* Moves `pages` before page `to`, in their order. */
int ely_move_pages(fz_context *ctx, fz_document *doc, const int *pages, int n, int to)
{
	pdf_document *pdf = NULL;
	pdf_obj **moved = NULL;
	int begun = 0, i, at;
	fz_var(pdf);
	fz_var(begun);
	fz_var(moved);
	fz_try(ctx)
	{
		pdf = ely_pdf_of(ctx, doc);
		check_pages(ctx, pdf, pages, n);
		if (to < 0 || to > pdf_count_pages(ctx, pdf))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "no place %d to move to", to);
		moved = fz_calloc(ctx, n, sizeof *moved);
		for (i = 0; i < n; i++)
			moved[i] = pdf_keep_obj(ctx, pdf_lookup_page_obj(ctx, pdf, pages[i]));
		pdf_begin_operation(ctx, pdf, "Move pages");
		begun = 1;
		for (i = n - 1; i >= 0; i--)
			pdf_delete_page(ctx, pdf, pages[i]);
		for (at = to, i = 0; i < n; i++)
			at -= pages[i] < to;
		for (i = 0; i < n; i++)
			pdf_insert_page(ctx, pdf, at + i, moved[i]);
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
	{
		for (i = 0; moved && i < n; i++)
			pdf_drop_obj(ctx, moved[i]);
		fz_free(ctx, moved);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

/* Turns `pages` by a multiple of 90 degrees. */
int ely_rotate_pages(fz_context *ctx, fz_document *doc, const int *pages, int n, int by)
{
	pdf_document *pdf = NULL;
	int begun = 0, i;
	fz_var(pdf);
	fz_var(begun);
	fz_try(ctx)
	{
		pdf = ely_pdf_of(ctx, doc);
		check_pages(ctx, pdf, pages, n);
		if (by % 90)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a turn of %d degrees", by);
		pdf_begin_operation(ctx, pdf, "Rotate pages");
		begun = 1;
		for (i = 0; i < n; i++)
		{
			pdf_obj *page = pdf_lookup_page_obj(ctx, pdf, pages[i]);
			int now = pdf_to_int(ctx, pdf_dict_get_inheritable(ctx, page, PDF_NAME(Rotate)));
			pdf_dict_put_int(ctx, page, PDF_NAME(Rotate), ((now + by) % 360 + 360) % 360);
		}
		pdf_end_operation(ctx, pdf);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

/* Seen margins (top, right, bottom, left) to user sides. */
static fz_rect inset(fz_rect box, int rotate, const float seen[4])
{
	/* User sides: 0 y1, 1 x1, 2 y0, 3 x0. */
	static const int under[4][4] = { { 0, 1, 2, 3 }, { 3, 0, 1, 2 }, { 2, 3, 0, 1 }, { 1, 2, 3, 0 } };
	float side[4] = { 0, 0, 0, 0 };
	int i, turn = ((rotate % 360 + 360) % 360) / 90;
	for (i = 0; i < 4; i++)
		side[under[turn][i]] = seen[i];
	box.y1 -= side[0];
	box.x1 -= side[1];
	box.y0 += side[2];
	box.x0 += side[3];
	return box;
}

/* Crops `pages` by seen margins in points; zeros uncrop. */
int ely_crop_pages(fz_context *ctx, fz_document *doc, const int *pages, int n, const float margins[4])
{
	pdf_document *pdf = NULL;
	int begun = 0, i;
	fz_var(pdf);
	fz_var(begun);
	fz_try(ctx)
	{
		int none = margins[0] == 0 && margins[1] == 0 && margins[2] == 0 && margins[3] == 0;
		pdf = ely_pdf_of(ctx, doc);
		check_pages(ctx, pdf, pages, n);
		pdf_begin_operation(ctx, pdf, "Crop pages");
		begun = 1;
		for (i = 0; i < n; i++)
		{
			pdf_obj *page = pdf_lookup_page_obj(ctx, pdf, pages[i]);
			fz_rect media = pdf_to_rect(ctx, pdf_dict_get_inheritable(ctx, page, PDF_NAME(MediaBox)));
			int rotate = pdf_to_int(ctx, pdf_dict_get_inheritable(ctx, page, PDF_NAME(Rotate)));
			/* Points as shown; user units may be larger. */
			float unit = pdf_to_real(ctx, pdf_dict_get(ctx, page, PDF_NAME(UserUnit)));
			float user[4];
			int side;
			for (side = 0; side < 4; side++)
				user[side] = margins[side] / (unit > 0 ? unit : 1);
			fz_rect crop = inset(media, rotate, user);
			if (none)
			{
				pdf_dict_del(ctx, page, PDF_NAME(CropBox));
				continue;
			}
			if (crop.x1 - crop.x0 < 1 || crop.y1 - crop.y0 < 1)
				fz_throw(ctx, FZ_ERROR_ARGUMENT, "the margins leave nothing of page %d", pages[i] + 1);
			pdf_dict_put_rect(ctx, page, PDF_NAME(CropBox), crop);
		}
		pdf_end_operation(ctx, pdf);
	}
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

/* A blank page of w by h points at `at`. */
int ely_insert_blank(fz_context *ctx, fz_document *doc, int at, float w, float h)
{
	pdf_document *pdf = NULL;
	pdf_obj *page = NULL;
	int begun = 0;
	fz_var(pdf);
	fz_var(page);
	fz_var(begun);
	fz_try(ctx)
	{
		pdf = ely_pdf_of(ctx, doc);
		if (at < 0 || at > pdf_count_pages(ctx, pdf) || w < 1 || h < 1)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "a blank page at %d of %g by %g", at, w, h);
		pdf_begin_operation(ctx, pdf, "Insert a blank page");
		begun = 1;
		page = pdf_add_page(ctx, pdf, fz_make_rect(0, 0, w, h), 0, NULL, NULL);
		pdf_insert_page(ctx, pdf, at, page);
		pdf_end_operation(ctx, pdf);
	}
	fz_always(ctx)
		pdf_drop_obj(ctx, page);
	fz_catch(ctx)
		return ely_edit_failed(ctx, pdf, begun);
	return 0;
}

/* Pages of `pdf` other than the sorted `keep`. */
static int *others(fz_context *ctx, pdf_document *pdf, const int *keep, int n, int *k)
{
	int i, count = pdf_count_pages(ctx, pdf);
	int *rest = fz_malloc_array(ctx, count + 1, int);
	*k = 0;
	for (i = 0; i < count; i++)
		if (bsearch(&i, keep, n, sizeof(int), by_number) == NULL)
			rest[(*k)++] = i;
	return rest;
}

/* A snapshot, reopened, changed, unsigned, compacted to `out`. */
void ely_save_copy(fz_context *ctx, pdf_document *pdf, const char *raw, const char *out, const char *password,
	const pdf_write_options *opts, ely_change *change, void *arg)
{
	fz_document *copy = NULL;
	fz_var(copy);
	fz_try(ctx)
	{
		pdf_write_options compact = opts ? *opts : pdf_default_write_options;
		pdf_save_snapshot(ctx, pdf, raw);
		copy = fz_open_document(ctx, raw);
		if (fz_needs_password(ctx, copy) && !fz_authenticate_password(ctx, copy, password))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "the written copy refused its password");
		if (change)
			change(ctx, copy, arg);
		ely_unsign(ctx, ely_pdf_of(ctx, copy));
		/* Unused objects, such as deleted pages, go. */
		compact.do_garbage = 2;
		pdf_save_document(ctx, ely_pdf_of(ctx, copy), out, &compact);
	}
	fz_always(ctx)
		fz_drop_document(ctx, copy);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

typedef struct
{
	const int *pages;
	int n;
} kept;

/* Deletes every page not kept. */
static void keep_only(fz_context *ctx, fz_document *copy, void *arg)
{
	kept *keep = arg;
	pdf_document *written = ely_pdf_of(ctx, copy);
	int k, *rest = others(ctx, written, keep->pages, keep->n, &k);
	fz_try(ctx)
	{
		if (k > 0)
			delete_pages(ctx, written, rest, k);
	}
	fz_always(ctx)
		fz_free(ctx, rest);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

int ely_save(fz_context *ctx, fz_document *doc, const char *raw, const char *out, const char *password)
{
	fz_try(ctx)
		ely_save_copy(ctx, ely_pdf_of(ctx, doc), raw, out, password, NULL, NULL, NULL);
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* Saves only `pages` to `out`; the document is untouched. */
int ely_extract(fz_context *ctx, fz_document *doc, const int *pages, int n,
	const char *raw, const char *out, const char *password)
{
	fz_try(ctx)
	{
		pdf_document *pdf = ely_pdf_of(ctx, doc);
		kept keep = { pages, n };
		check_pages(ctx, pdf, pages, n);
		ely_save_copy(ctx, pdf, raw, out, password, NULL, keep_only, &keep);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}
