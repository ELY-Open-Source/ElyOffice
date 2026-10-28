/* MuPDF behind plain return codes: fz_try never crosses into Rust. */

#include <mupdf/fitz.h>
#include <mupdf/pdf.h>
#include <string.h>

static char last_error[512];

/* Takes the caught error to Rust; MuPDF counts it handled. */
int ely_fail(fz_context *ctx)
{
	int code;
	fz_strlcpy(last_error, fz_convert_error(ctx, &code), sizeof last_error);
	return -1;
}

/* A second failure while handling the first. */
void ely_fail_also(fz_context *ctx, const char *what)
{
	char also[256];
	int code;
	fz_snprintf(also, sizeof also, "; %s: %s", what, fz_convert_error(ctx, &code));
	fz_strlcat(last_error, also, sizeof last_error);
}

const char *ely_error(void)
{
	return last_error;
}

fz_context *ely_init(void)
{
	fz_context *ctx = fz_new_context(NULL, NULL, FZ_STORE_DEFAULT);
	if (!ctx)
	{
		fz_strlcpy(last_error, "no MuPDF context", sizeof last_error);
		return NULL;
	}
	fz_try(ctx)
		fz_register_document_handlers(ctx);
	fz_catch(ctx)
	{
		ely_fail(ctx);
		fz_drop_context(ctx);
		return NULL;
	}
	return ctx;
}

/* What MuPDF warned while opening, for repair reports. */
static char warned[4096];

typedef struct
{
	fz_warning_cb *next;
	void *user;
} passing;

/* MuPDF draws an unembedded font with another. */
static const char notice[] = "non-embedded font using identity encoding:";

/* 1 while the last warning was that notice. */
static int substituted;

static void keep_warning(void *user, const char *message)
{
	passing *p = user;
	/* An unembedded font drawn by another is no damage. */
	if (!strncmp(message, notice, sizeof notice - 1))
		substituted = 1;
	else if (strncmp(message, "... repeated ", 13))
		substituted = 0;
	if (!substituted && strlen(warned) + strlen(message) + 2 < sizeof warned)
	{
		fz_strlcat(warned, message, sizeof warned);
		fz_strlcat(warned, "\n", sizeof warned);
	}
	if (p->next)
		p->next(p->user, message);
}

static passing watching;
static int watched;

/* Collects MuPDF's warnings until `ely_watch_end`. */
void ely_watch_begin(fz_context *ctx)
{
	/* A pending repeat is of MuPDF's last warning. */
	substituted = !strncmp(ctx->warn.message, notice, sizeof notice - 1);
	watching.next = fz_warning_callback(ctx, &watching.user);
	warned[0] = 0;
	watched = 1;
	fz_set_warning_callback(ctx, keep_warning, &watching);
}

/* 1 if MuPDF warned since `ely_watch_begin`. */
int ely_watch_end(fz_context *ctx)
{
	if (!watched)
		return 0;
	fz_flush_warnings(ctx);
	fz_set_warning_callback(ctx, watching.next, watching.user);
	watched = 0;
	return warned[0] != 0;
}

int ely_open(fz_context *ctx, const char *path, fz_document **out)
{
	ely_watch_begin(ctx);
	fz_try(ctx)
		*out = fz_open_document(ctx, path);
	fz_always(ctx)
		ely_watch_end(ctx);
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* Reads every object and non-image stream; 1 if any warned. */
int ely_scan(fz_context *ctx, fz_document *doc)
{
	pdf_document *pdf = pdf_document_from_fz_document(ctx, doc);
	int i;
	if (!pdf)
		return 0;
	ely_watch_begin(ctx);
	for (i = 1; i < pdf_xref_len(ctx, pdf); i++)
	{
		pdf_obj *obj = NULL;
		fz_buffer *bytes = NULL;
		fz_var(obj);
		fz_var(bytes);
		fz_try(ctx)
		{
			obj = pdf_new_indirect(ctx, pdf, i, 0);
			pdf_cache_object(ctx, pdf, i);
			/* Pictures are checked as they draw, or before an overwrite. */
			if (pdf_is_stream(ctx, obj) && !pdf_name_eq(ctx, pdf_dict_get(ctx, obj, PDF_NAME(Subtype)), PDF_NAME(Image)))
				bytes = pdf_load_stream_number(ctx, pdf, i);
		}
		fz_always(ctx)
		{
			fz_drop_buffer(ctx, bytes);
			pdf_drop_obj(ctx, obj);
		}
		fz_catch(ctx)
		{
			/* An unreadable object goes in the report. */
			fz_warn(ctx, "object %d: %s", i, fz_caught_message(ctx));
			fz_ignore_error(ctx);
		}
	}
	return ely_watch_end(ctx);
}

/* The warnings of the last watch. */
const char *ely_open_warnings(void)
{
	return warned;
}

void ely_close(fz_context *ctx, fz_document *doc)
{
	fz_drop_document(ctx, doc);
}

/* 1 locked, 0 open, -1 an error. */
int ely_needs_password(fz_context *ctx, fz_document *doc)
{
	int locked = 0;
	fz_var(locked);
	fz_try(ctx)
		locked = fz_needs_password(ctx, doc);
	fz_catch(ctx)
		return ely_fail(ctx);
	return locked;
}

/* Access bits: 4 owner; 0 refused, -1 an error. */
int ely_authenticate(fz_context *ctx, fz_document *doc, const char *password)
{
	int access = 0;
	fz_var(access);
	fz_try(ctx)
		access = fz_authenticate_password(ctx, doc, password);
	fz_catch(ctx)
		return ely_fail(ctx);
	return access;
}

int ely_count_pages(fz_context *ctx, fz_document *doc, int *out)
{
	fz_try(ctx)
		*out = fz_count_pages(ctx, doc);
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

int ely_page_bounds(fz_context *ctx, fz_document *doc, int number, float out[4])
{
	fz_page *page = NULL;
	fz_var(page);
	fz_try(ctx)
	{
		page = fz_load_page(ctx, doc, number);
		fz_rect box = fz_bound_page(ctx, page);
		out[0] = box.x0; out[1] = box.y0; out[2] = box.x1; out[3] = box.y1;
	}
	fz_always(ctx)
		fz_drop_page(ctx, page);
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* The page turned, scaled and moved to the origin. */
static fz_matrix placed(fz_rect box, float scale, int rotate)
{
	fz_matrix ctm = fz_concat(fz_rotate(rotate), fz_scale(scale, scale));
	fz_rect turned = fz_transform_rect(box, ctm);
	return fz_concat(ctm, fz_translate(-turned.x0, -turned.y0));
}

/* Paints device pixels x, y, w, h as BGRA. */
int ely_render(fz_context *ctx, fz_document *doc, int number, float scale, int rotate,
	const int tile[4], unsigned char *bgra)
{
	fz_page *page = NULL;
	fz_pixmap *pix = NULL;
	fz_device *dev = NULL;
	fz_var(page);
	fz_var(pix);
	fz_var(dev);
	fz_try(ctx)
	{
		page = fz_load_page(ctx, doc, number);
		fz_matrix ctm = placed(fz_bound_page(ctx, page), scale, rotate);
		fz_irect area = fz_make_irect(tile[0], tile[1], tile[0] + tile[2], tile[1] + tile[3]);
		pix = fz_new_pixmap_with_bbox(ctx, fz_device_bgr(ctx), area, NULL, 0);
		fz_clear_pixmap_with_value(ctx, pix, 0xff);
		dev = fz_new_draw_device(ctx, fz_identity, pix);
		fz_run_page(ctx, page, dev, ctm, NULL);
		fz_close_device(ctx, dev);
		int stride = (int)fz_pixmap_stride(ctx, pix);
		unsigned char *rows = fz_pixmap_samples(ctx, pix);
		for (int y = 0; y < tile[3]; y++)
			for (int x = 0; x < tile[2]; x++)
			{
				const unsigned char *from = rows + y * stride + x * 3;
				unsigned char *to = bgra + ((size_t)y * tile[2] + x) * 4;
				to[0] = from[0]; to[1] = from[1]; to[2] = from[2]; to[3] = 0xff;
			}
	}
	fz_always(ctx)
	{
		fz_drop_device(ctx, dev);
		fz_drop_pixmap(ctx, pix);
		fz_drop_page(ctx, page);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* One character: line, code point, box in page points. */
typedef struct
{
	int line;
	int c;
	float x0, y0, x1, y1;
} ely_char;

static void add_chars(fz_context *ctx, fz_buffer *buf, fz_stext_block *block, int *line)
{
	for (; block; block = block->next)
	{
		if (block->type == FZ_STEXT_BLOCK_STRUCT && block->u.s.down)
			add_chars(ctx, buf, block->u.s.down->first_block, line);
		if (block->type != FZ_STEXT_BLOCK_TEXT)
			continue;
		for (fz_stext_line *l = block->u.t.first_line; l; l = l->next, ++*line)
			for (fz_stext_char *ch = l->first_char; ch; ch = ch->next)
			{
				fz_rect box = fz_rect_from_quad(ch->quad);
				ely_char one = { *line, ch->c, box.x0, box.y0, box.x1, box.y1 };
				fz_append_data(ctx, buf, &one, sizeof one);
			}
	}
}

/* The page's characters in reading order; free with ely_free. */
int ely_text(fz_context *ctx, fz_document *doc, int number, unsigned char **data, size_t *len)
{
	fz_page *page = NULL;
	fz_stext_page *text = NULL;
	fz_buffer *buf = NULL;
	fz_var(page);
	fz_var(text);
	fz_var(buf);
	fz_try(ctx)
	{
		page = fz_load_page(ctx, doc, number);
		text = fz_new_stext_page_from_page(ctx, page, NULL);
		buf = fz_new_buffer(ctx, 4096);
		int line = 0;
		add_chars(ctx, buf, text->first_block, &line);
		*len = fz_buffer_extract(ctx, buf, data);
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, buf);
		fz_drop_stext_page(ctx, text);
		fz_drop_page(ctx, page);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* Each hit: its quad count, then those quads. */
static int add_hit(fz_context *ctx, void *opaque, int count, fz_quad *quads)
{
	fz_buffer *buf = opaque;
	fz_append_data(ctx, buf, &count, sizeof count);
	fz_append_data(ctx, buf, quads, sizeof *quads * count);
	return 0;
}

/* Every hit on a page; free with ely_free. */
int ely_search(fz_context *ctx, fz_document *doc, int number, const char *needle,
	unsigned char **data, size_t *len)
{
	fz_page *page = NULL;
	fz_stext_page *text = NULL;
	fz_buffer *buf = NULL;
	fz_var(page);
	fz_var(text);
	fz_var(buf);
	fz_try(ctx)
	{
		page = fz_load_page(ctx, doc, number);
		text = fz_new_stext_page_from_page(ctx, page, NULL);
		buf = fz_new_buffer(ctx, 256);
		fz_search_stext_page_cb(ctx, text, needle, add_hit, buf);
		*len = fz_buffer_extract(ctx, buf, data);
	}
	fz_always(ctx)
	{
		fz_drop_buffer(ctx, buf);
		fz_drop_stext_page(ctx, text);
		fz_drop_page(ctx, page);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

static void outline_json(fz_context *ctx, fz_output *out, fz_document *doc, fz_outline *item)
{
	fz_write_string(ctx, out, "[");
	for (; item; item = item->next)
	{
		int page = item->page.page >= 0 ? fz_page_number_from_location(ctx, doc, item->page) : -1;
		fz_write_printf(ctx, out, "{%q:%q,%q:%d,%q:%g,%q:", "title", item->title ? item->title : "",
			"page", page, "y", item->y, "down");
		outline_json(ctx, out, doc, item->down);
		fz_write_string(ctx, out, item->next ? "}," : "}");
	}
	fz_write_string(ctx, out, "]");
}

/* Bookmarks as JSON: title, page or -1, y, down. */
int ely_outline(fz_context *ctx, fz_document *doc, unsigned char **data, size_t *len)
{
	fz_outline *outline = NULL;
	fz_buffer *buf = NULL;
	fz_output *out = NULL;
	fz_var(outline);
	fz_var(buf);
	fz_var(out);
	fz_try(ctx)
	{
		outline = fz_load_outline(ctx, doc);
		buf = fz_new_buffer(ctx, 1024);
		out = fz_new_output_with_buffer(ctx, buf);
		outline_json(ctx, out, doc, outline);
		fz_close_output(ctx, out);
		*len = fz_buffer_extract(ctx, buf, data);
	}
	fz_always(ctx)
	{
		fz_drop_output(ctx, out);
		fz_drop_buffer(ctx, buf);
		fz_drop_outline(ctx, outline);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

void ely_free(fz_context *ctx, void *data)
{
	fz_free(ctx, data);
}
