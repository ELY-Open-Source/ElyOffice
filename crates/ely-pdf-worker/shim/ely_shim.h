/* What the shim files share. */
#ifndef ELY_SHIM_H
#define ELY_SHIM_H

#include <mupdf/fitz.h>
#include <mupdf/pdf.h>

int ely_fail(fz_context *ctx);
const char *ely_error(void);
int ely_render(fz_context *ctx, fz_document *doc, int number, float scale, int rotate, const int tile[4],
	unsigned char *bgra);
void ely_fail_also(fz_context *ctx, const char *what);
pdf_document *ely_pdf_of(fz_context *ctx, fz_document *doc);
int ely_edit_failed(fz_context *ctx, pdf_document *pdf, int begun);
void ely_forget_resources(fz_context *ctx, pdf_document *pdf);
pdf_obj *ely_target(fz_context *ctx, pdf_document *pdf, pdf_obj *obj);
typedef void (ely_change)(fz_context *ctx, fz_document *copy, void *arg);
void ely_save_copy(fz_context *ctx, pdf_document *pdf, const char *raw, const char *out, const char *password,
	const pdf_write_options *opts,
	ely_change *change, void *arg);

/* A text block as read, page space, and its glyphs. */
typedef struct
{
	void *glyphs;
	fz_font *font;
	fz_point first;
	float size;
	int rgb;
	fz_rect bbox;
	int lines;
	float gap;
	/* Each line's first origin, as read. */
	fz_point *at;
	int count;
} ely_shape;

fz_matrix ely_to_user(fz_context *ctx, pdf_page *page);
void ely_read_block(fz_context *ctx, fz_document *doc, pdf_page *page, int number, int index, const char *expected,
	ely_shape *shape);
void ely_remove_block(fz_context *ctx, pdf_document *pdf, pdf_page *page, ely_shape *shape);
void ely_drop_shape(fz_context *ctx, ely_shape *shape);
pdf_obj *ely_append_content(fz_context *ctx, pdf_document *pdf, pdf_obj *page, fz_buffer *buf);
const char *ely_add_resource(fz_context *ctx, pdf_obj *page, pdf_obj *type, const char *stem,
	pdf_obj *ref, char *name, size_t room);
void ely_json_string(fz_context *ctx, fz_output *out, const char *s);
fz_json *ely_member(fz_context *ctx, fz_json *obj, const char *key, int type);
float ely_number(fz_context *ctx, fz_json *array, int at);
fz_rect ely_rect(fz_context *ctx, fz_json *array, float least);
pdf_annot *ely_find_annot(fz_context *ctx, pdf_page *page, int id, int widget);
void ely_write_fields(fz_context *ctx, fz_output *out, pdf_page *page);
void ely_unlink_field(fz_context *ctx, pdf_document *pdf, pdf_obj *field);
void ely_unsign(fz_context *ctx, pdf_document *pdf);
void ely_watch_begin(fz_context *ctx);
int ely_watch_end(fz_context *ctx);
const char *ely_open_warnings(void);
void ely_drop_alt(fz_context *ctx, pdf_document *pdf, pdf_page *page);

/* A font to write with: new, or the page's own. */
typedef enum
{
	PEN_CID,
	PEN_SIMPLE,
	PEN_CJK,
	PEN_OWN
} ely_pen_kind;

typedef struct
{
	ely_pen_kind kind;
	fz_font *font;
	int ordering;
	pdf_obj *ref;
	/* The page's own font: object number, description, code width. */
	int num;
	pdf_font_desc *desc;
	int bytes;
	/* Unicode and code pairs of the own font. */
	int *map, mapped;
} ely_pen;

int ely_choose_pen(fz_context *ctx, const char *choice, fz_font *own, pdf_page *page, ely_pen *p);
const char *ely_pen_resource(fz_context *ctx, pdf_document *pdf, pdf_page *page, ely_pen *p, char *name, size_t room);
int ely_pen_has(fz_context *ctx, ely_pen *p, int u);
float ely_pen_advance(fz_context *ctx, ely_pen *p, int u, float size);
void ely_pen_code(fz_context *ctx, fz_buffer *buf, ely_pen *p, int u);
void ely_drop_pen(fz_context *ctx, ely_pen *p);

#endif
