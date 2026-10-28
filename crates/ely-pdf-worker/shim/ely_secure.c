/* Permissions and protected copies (PDF-11). */

#include <string.h>

#include "ely_shim.h"

enum { MAY_PRINT = 1, MAY_COPY = 2, MAY_EDIT = 4, MAY_ANNOTATE = 8, MAY_FILL = 16, MAY_ASSEMBLE = 32 };

/* What the document allows, as Ely's bits. */
int ely_permissions(fz_context *ctx, fz_document *doc)
{
	int may = 0;
	fz_var(may);
	fz_try(ctx)
	{
		pdf_document *pdf = ely_pdf_of(ctx, doc);
		int edit = pdf_has_permission(ctx, pdf, FZ_PERMISSION_EDIT) != 0;
		int annotate = pdf_has_permission(ctx, pdf, FZ_PERMISSION_ANNOTATE) != 0;
		may |= pdf_has_permission(ctx, pdf, FZ_PERMISSION_PRINT) ? MAY_PRINT : 0;
		may |= pdf_has_permission(ctx, pdf, FZ_PERMISSION_COPY) ? MAY_COPY : 0;
		may |= edit ? MAY_EDIT : 0;
		may |= annotate ? MAY_ANNOTATE : 0;
		may |= annotate || pdf_has_permission(ctx, pdf, FZ_PERMISSION_FORM) ? MAY_FILL : 0;
		may |= edit || pdf_has_permission(ctx, pdf, FZ_PERMISSION_ASSEMBLE) ? MAY_ASSEMBLE : 0;
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return may;
}

/* PDF's permission bits for Ely's `may`. */
static int pdf_bits(int may)
{
	int p = ~(PDF_PERM_PRINT | PDF_PERM_MODIFY | PDF_PERM_COPY | PDF_PERM_ANNOTATE | PDF_PERM_FORM |
		PDF_PERM_ASSEMBLE | PDF_PERM_PRINT_HQ | 3);
	p |= may & MAY_PRINT ? PDF_PERM_PRINT | PDF_PERM_PRINT_HQ : 0;
	p |= may & MAY_COPY ? PDF_PERM_COPY : 0;
	p |= may & MAY_EDIT ? PDF_PERM_MODIFY : 0;
	p |= may & MAY_ANNOTATE ? PDF_PERM_ANNOTATE : 0;
	p |= may & MAY_FILL ? PDF_PERM_FORM : 0;
	p |= may & MAY_ASSEMBLE ? PDF_PERM_ASSEMBLE : 0;
	return p;
}

static void put_password(fz_context *ctx, char *into, const char *password)
{
	if (strlen(password) >= 128)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a password longer than 127 bytes");
	strcpy(into, password);
}

/* Writes `out` with AES-256 and `may`, or unprotected. */
int ely_protect(fz_context *ctx, fz_document *doc, const char *raw, const char *out, const char *password,
	int protect, const char *user, const char *owner, int may)
{
	fz_try(ctx)
	{
		pdf_write_options opts = pdf_default_write_options;
		opts.do_encrypt = protect ? PDF_ENCRYPT_AES_256 : PDF_ENCRYPT_NONE;
		if (protect)
		{
			if (!*owner)
				fz_throw(ctx, FZ_ERROR_ARGUMENT, "an owner password is needed");
			if (!strcmp(user, owner))
				fz_throw(ctx, FZ_ERROR_ARGUMENT, "the owner password equals the open password");
			put_password(ctx, opts.upwd_utf8, user);
			put_password(ctx, opts.opwd_utf8, owner);
			opts.permissions = pdf_bits(may);
		}
		ely_save_copy(ctx, ely_pdf_of(ctx, doc), raw, out, password, &opts, NULL, NULL);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}
