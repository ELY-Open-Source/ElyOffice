/* A folder's HTML page as a PDF (MARK-11). */

#include "ely_shim.h"

/* A4, two centimetres in from each edge. */
static const fz_rect sheet = {0, 0, 595.28f, 841.89f};
static const fz_rect area = {56.69f, 56.69f, 538.59f, 785.2f};

int ely_html_pdf(fz_context *ctx, const char *dir, const char *page, const char *out)
{
	fz_archive *folder = NULL;
	fz_buffer *html = NULL;
	fz_story *story = NULL;
	fz_document_writer *writer = NULL;
	fz_var(folder);
	fz_var(html);
	fz_var(story);
	fz_var(writer);
	fz_try(ctx)
	{
		ely_watch_begin(ctx);
		folder = fz_open_directory(ctx, dir);
		html = fz_read_archive_entry(ctx, folder, page);
		story = fz_new_story(ctx, html, NULL, 11, folder);
		writer = fz_new_pdf_writer(ctx, out, "");
		int more;
		do
		{
			fz_device *dev = fz_begin_page(ctx, writer, sheet);
			more = fz_place_story(ctx, story, area, NULL);
			fz_draw_story(ctx, story, dev, fz_identity);
			fz_end_page(ctx, writer);
		} while (more);
		fz_close_document_writer(ctx, writer);
		/* The story keeps its own warnings. */
		const char *said = fz_story_warnings(ctx, story);
		if (said && said[0])
			fz_throw(ctx, FZ_ERROR_FORMAT, "the page did not set whole: %s", said);
		if (ely_watch_end(ctx))
			fz_throw(ctx, FZ_ERROR_FORMAT, "the page did not set whole: %s", ely_open_warnings());
	}
	fz_always(ctx)
	{
		ely_watch_end(ctx);
		fz_drop_document_writer(ctx, writer);
		fz_drop_story(ctx, story);
		fz_drop_buffer(ctx, html);
		fz_drop_archive(ctx, folder);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}
