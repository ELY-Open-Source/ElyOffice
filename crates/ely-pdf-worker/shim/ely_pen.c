/* The font new text is written in (PDF-03). */

#include <string.h>

#include "ely_shim.h"

/* The script face for typed signatures, from the worker. */
static const unsigned char *script_data;
static size_t script_len;

void ely_script_font(const unsigned char *data, size_t len)
{
	script_data = data;
	script_len = len;
}

/* `cid` as text extraction reads it (pdf-op-run.c), or -1. */
static int unicode_of(pdf_font_desc *desc, int cid)
{
	int ucs[PDF_MRANGE_CAP], n = 0;
	if (desc->to_unicode)
		n = pdf_lookup_cmap_full(desc->to_unicode, cid, ucs);
	if (n == 1 && ucs[0] >= 8 && ucs[0] <= 13)
		ucs[0] = ' ';
	if (n == 1 && (ucs[0] < 32 || (ucs[0] >= 127 && ucs[0] < 160)))
		n = 0;
	if (n == 0 && cid < (int)desc->cid_to_ucs_len)
		ucs[0] = desc->cid_to_ucs[cid], n = 1;
	return n == 1 && ucs[0] > 0 && ucs[0] != FZ_REPLACEMENT_CHARACTER ? ucs[0] : -1;
}

/* `desc` Unicode-to-code pairs; -1 when codes are not CIDs. */
static int own_codes(fz_context *ctx, pdf_font_desc *desc, ely_pen *p)
{
	pdf_cmap *enc = desc->encoding;
	int cid, end;
	if (!enc || strcmp(enc->cmap_name, "Identity-H") || enc->codespace_len != 1 || enc->codespace[0].n > 2)
		return -1;
	p->bytes = enc->codespace[0].n;
	end = 1 << (8 * p->bytes);
	p->map = fz_malloc_array(ctx, 2 * end, int);
	for (cid = 0; cid < end; cid++)
	{
		int u = unicode_of(desc, cid);
		if (u > 0 && pdf_font_cid_to_gid(ctx, desc, cid) > 0)
			p->map[2 * p->mapped] = u, p->map[2 * p->mapped + 1] = cid, p->mapped++;
	}
	return 0;
}

/* The page font drawn as `own`; -1: cannot write. */
static int own_font(fz_context *ctx, pdf_page *page, fz_font *own, ely_pen *p)
{
	pdf_obj *res = pdf_page_resources(ctx, page);
	pdf_resource_stack stack = { NULL, res };
	pdf_obj *fonts = pdf_dict_get(ctx, res, PDF_NAME(Font));
	int i, n = pdf_dict_len(ctx, fonts);
	for (i = 0; i < n && !p->desc; i++)
	{
		pdf_font_desc *desc = pdf_load_font(ctx, page->doc, &stack, pdf_dict_get_val(ctx, fonts, i));
		if (desc->font != own)
		{
			pdf_drop_font(ctx, desc);
			continue;
		}
		p->desc = desc;
		/* A direct font cannot be named again after the edit. */
		p->num = pdf_to_num(ctx, pdf_dict_get_val(ctx, fonts, i));
		return p->num ? own_codes(ctx, desc, p) : -1;
	}
	/* Drawn from elsewhere, as in a form: not writable here. */
	return -1;
}

int ely_choose_pen(fz_context *ctx, const char *choice, fz_font *own, pdf_page *page, ely_pen *p)
{
	static const struct { const char *choice, *name; } latin[] = {
		{ "helvetica", "Helvetica" }, { "times", "Times-Roman" }, { "courier", "Courier" } };
	static const struct { const char *choice; int ordering; } cjk[] = {
		{ "cjk-sc", FZ_ADOBE_GB }, { "cjk-tc", FZ_ADOBE_CNS }, { "cjk-jp", FZ_ADOBE_JAPAN }, { "cjk-kr", FZ_ADOBE_KOREA } };
	size_t i;
	if (!strcmp(choice, "script"))
	{
		if (!script_data)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "no script font was given");
		p->kind = PEN_CID;
		p->font = fz_new_font_from_memory(ctx, "GreatVibes", script_data, (int)script_len, 0, 0);
		return 0;
	}
	if (!strcmp(choice, "original"))
	{
		if (!own)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "the block has no font of its own");
		p->kind = PEN_OWN;
		return own_font(ctx, page, own, p);
	}
	for (i = 0; i < nelem(latin); i++)
		if (!strcmp(choice, latin[i].choice))
		{
			p->kind = PEN_SIMPLE;
			p->font = fz_new_base14_font(ctx, latin[i].name);
			return 0;
		}
	for (i = 0; i < nelem(cjk); i++)
		if (!strcmp(choice, cjk[i].choice))
		{
			p->kind = PEN_CJK;
			p->ordering = cjk[i].ordering;
			p->font = fz_new_cjk_font(ctx, cjk[i].ordering);
			return 0;
		}
	fz_throw(ctx, FZ_ERROR_ARGUMENT, "no font %s", choice);
}

/* The script face cut to the glyphs `u` needs. */
static fz_font *script_subset(fz_context *ctx, fz_font *font, const int *u, int n)
{
	fz_buffer *whole = NULL, *cut = NULL;
	fz_font *subset = NULL;
	int *gids = fz_malloc_array(ctx, n + 1, int);
	unsigned char digest[16];
	char name[32];
	int i, k = 0;
	fz_var(whole);
	fz_var(cut);
	fz_try(ctx)
	{
		gids[k++] = 0;
		for (i = 0; i < n; i++)
			if (u[i] != '\n')
				gids[k++] = fz_encode_character(ctx, font, u[i]);
		whole = fz_new_buffer_from_shared_data(ctx, script_data, script_len);
		cut = fz_subset_ttf_for_gids(ctx, whole, gids, k, 0, 1);
		/* ISO 32000-1 9.6.4: a tag per subset. */
		fz_md5_buffer(ctx, cut, digest);
		for (i = 0; i < 6; i++)
			name[i] = 'A' + digest[i] % 26;
		fz_strlcpy(name + 6, "+GreatVibes", sizeof name - 6);
		subset = fz_new_font_from_buffer(ctx, name, cut, 0, 0);
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, whole);
		fz_drop_buffer(ctx, cut);
		fz_free(ctx, gids);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
	return subset;
}

/* `r` as UTF-16BE hex for a CMap. */
static void append_utf16(fz_context *ctx, fz_buffer *buf, int r)
{
	if (r > 0xFFFF)
		fz_append_printf(ctx, buf, "%04x%04x", 0xD800 + ((r - 0x10000) >> 10), 0xDC00 + ((r - 0x10000) & 0x3FF));
	else
		fz_append_printf(ctx, buf, "%04x", r);
}

/* Maps written runes back; the subset lost its cmap. */
static void script_to_unicode(fz_context *ctx, pdf_document *pdf, pdf_obj *fobj, fz_font *font, const int *u, int n)
{
	fz_buffer *buf = NULL;
	int *runes = fz_malloc_array(ctx, n + 1, int);
	int i, k, m = 0;
	fz_var(buf);
	fz_try(ctx)
	{
		for (i = 0; i < n; i++)
		{
			for (k = 0; k < m && runes[k] != u[i]; k++)
				;
			if (u[i] != '\n' && k == m)
				runes[m++] = u[i];
		}
		buf = fz_new_buffer(ctx, 256);
		fz_append_string(ctx, buf, "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n"
			"/CIDSystemInfo <</Registry(Adobe)/Ordering(UCS)/Supplement 0>> def\n"
			"/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n"
			"1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n");
		for (i = 0; i < m; i++)
		{
			if (i % 100 == 0)
				fz_append_printf(ctx, buf, "%d beginbfchar\n", fz_mini(100, m - i));
			fz_append_printf(ctx, buf, "<%04x> <", fz_encode_character(ctx, font, runes[i]));
			append_utf16(ctx, buf, runes[i]);
			fz_append_string(ctx, buf, ">\n");
			if (i % 100 == 99 || i == m - 1)
				fz_append_string(ctx, buf, "endbfchar\n");
		}
		fz_append_string(ctx, buf, "endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
		pdf_dict_put_drop(ctx, fobj, PDF_NAME(ToUnicode), pdf_add_stream(ctx, pdf, buf, NULL, 0));
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, buf);
		fz_free(ctx, runes);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
}

const char *ely_pen_resource(fz_context *ctx, pdf_document *pdf, pdf_page *page, ely_pen *p, const int *u, int n,
	char *name, size_t room)
{
	fz_font *subset = NULL;
	pdf_drop_obj(ctx, p->ref);
	p->ref = NULL;
	switch (p->kind)
	{
	case PEN_OWN:
		/* Removing the block may drop the font from the page. */
		p->ref = pdf_new_indirect(ctx, pdf, p->num, 0);
		break;
	case PEN_CID:
		/* Only the glyphs written; the whole face is 447 KB. */
		subset = script_subset(ctx, p->font, u, n);
		fz_try(ctx)
		{
			p->ref = pdf_add_cid_font(ctx, pdf, subset);
			/* The child took the face's own PostScript name. */
			pdf_dict_put_name(ctx, pdf_array_get(ctx, pdf_dict_get(ctx, p->ref, PDF_NAME(DescendantFonts)), 0),
				PDF_NAME(BaseFont), fz_font_name(ctx, subset));
			script_to_unicode(ctx, pdf, p->ref, p->font, u, n);
		}
		fz_always(ctx)
			fz_drop_font(ctx, subset);
		fz_catch(ctx)
			fz_rethrow(ctx);
		break;
	case PEN_SIMPLE:
		p->ref = pdf_add_simple_font(ctx, pdf, p->font, PDF_SIMPLE_ENCODING_LATIN);
		break;
	case PEN_CJK:
		p->ref = pdf_add_cjk_font(ctx, pdf, p->font, p->ordering, 0, 0);
		break;
	}
	return ely_add_resource(ctx, page->obj, PDF_NAME(Font), "ElyF", p->ref, name, room);
}

/* The own font's code for `u`, or -1. */
static int own_code(ely_pen *p, int u)
{
	int i;
	for (i = 0; i < p->mapped; i++)
		if (p->map[2 * i] == u)
			return p->map[2 * i + 1];
	return -1;
}

/* The new font's glyph for `u`, or 0. */
static int glyph(fz_context *ctx, ely_pen *p, int u)
{
	if (p->kind == PEN_SIMPLE && fz_windows_1252_from_unicode(u) < 0)
		return 0;
	return fz_encode_character(ctx, p->font, u);
}

int ely_pen_has(fz_context *ctx, ely_pen *p, int u)
{
	return p->kind == PEN_OWN ? own_code(p, u) >= 0 : glyph(ctx, p, u) > 0;
}

float ely_pen_advance(fz_context *ctx, ely_pen *p, int u, float size)
{
	if (p->kind == PEN_OWN)
		return pdf_lookup_hmtx(ctx, p->desc, own_code(p, u)).w * size / 1000;
	return fz_advance_glyph(ctx, p->font, glyph(ctx, p, u), 0) * size;
}

void ely_pen_code(fz_context *ctx, fz_buffer *buf, ely_pen *p, int u)
{
	switch (p->kind)
	{
	case PEN_OWN:
		fz_append_printf(ctx, buf, p->bytes == 1 ? "%02x" : "%04x", own_code(p, u));
		break;
	case PEN_CID:
		fz_append_printf(ctx, buf, "%04x", glyph(ctx, p, u));
		break;
	case PEN_SIMPLE:
		fz_append_printf(ctx, buf, "%02x", fz_windows_1252_from_unicode(u));
		break;
	case PEN_CJK:
		if (u > 0xffff)
			fz_append_printf(ctx, buf, "%04x%04x", 0xd800 + ((u - 0x10000) >> 10), 0xdc00 + ((u - 0x10000) & 0x3ff));
		else
			fz_append_printf(ctx, buf, "%04x", u);
		break;
	}
}

void ely_drop_pen(fz_context *ctx, ely_pen *p)
{
	fz_drop_font(ctx, p->font);
	pdf_drop_obj(ctx, p->ref);
	pdf_drop_font(ctx, p->desc);
	fz_free(ctx, p->map);
}
