/* Signatures: read and checked (PDF-12, QA-15). */

#include <limits.h>
#include <string.h>

#include <openssl/pkcs7.h>
#include <openssl/x509.h>

#include <mupdf/helpers/pkcs7-openssl.h>

#include "ely_shim.h"

static const char *digest_name(pdf_signature_error error)
{
	switch (error)
	{
	case PDF_SIGNATURE_ERROR_OKAY: return "ok";
	case PDF_SIGNATURE_ERROR_DIGEST_FAILURE: return "changed";
	case PDF_SIGNATURE_ERROR_NO_CERTIFICATE: return "no_certificate";
	default: return "unreadable";
	}
}

static void put_hex(fz_context *ctx, fz_output *out, const unsigned char *data, int len)
{
	int i;
	fz_write_byte(ctx, out, '"');
	for (i = 0; i < len; i++)
		fz_write_printf(ctx, out, "%02x", data[i]);
	fz_write_byte(ctx, out, '"');
}

static void put_cert(fz_context *ctx, fz_output *out, X509 *cert, int *first)
{
	unsigned char *der = NULL;
	int len = i2d_X509(cert, &der);
	if (len <= 0)
		fz_throw(ctx, FZ_ERROR_FORMAT, "a certificate that will not encode");
	fz_try(ctx)
	{
		if (!*first)
			fz_write_byte(ctx, out, ',');
		*first = 0;
		put_hex(ctx, out, der, len);
	}
	fz_always(ctx)
		OPENSSL_free(der);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* The signer's certificate first, then the rest it carries. */
static void put_certs(fz_context *ctx, fz_output *out, const char *contents, size_t len)
{
	const unsigned char *at = (const unsigned char *)contents;
	PKCS7 *p7 = len < INT_MAX ? d2i_PKCS7(NULL, &at, (long)len) : NULL;
	STACK_OF(X509) *signers = NULL, *carried;
	int i, first = 1;
	if (!p7 || !PKCS7_type_is_signed(p7))
	{
		PKCS7_free(p7);
		fz_throw(ctx, FZ_ERROR_FORMAT, "a signature that is not PKCS #7 signed data");
	}
	fz_try(ctx)
	{
		carried = p7->d.sign->cert;
		signers = PKCS7_get0_signers(p7, NULL, 0);
		fz_write_string(ctx, out, "[");
		for (i = 0; i < sk_X509_num(signers); i++)
			put_cert(ctx, out, sk_X509_value(signers, i), &first);
		for (i = 0; i < sk_X509_num(carried); i++)
			if (sk_X509_find(signers, sk_X509_value(carried, i)) < 0)
				put_cert(ctx, out, sk_X509_value(carried, i), &first);
		fz_write_string(ctx, out, "]");
	}
	fz_always(ctx)
	{
		sk_X509_free(signers);
		PKCS7_free(p7);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
}

static void put_text(fz_context *ctx, fz_output *out, const char *key, pdf_obj *value)
{
	fz_write_printf(ctx, out, ",\"%s\":", key);
	ely_json_string(ctx, out, pdf_is_string(ctx, value) ? pdf_to_text_string(ctx, value) : "");
}

static void put_signed(fz_context *ctx, fz_output *out, pdf_document *pdf, pdf_obj *field, pdf_pkcs7_verifier *verifier)
{
	pdf_obj *v = pdf_dict_get_inheritable(ctx, field, PDF_NAME(V));
	pdf_pkcs7_distinguished_name *dn = pdf_signature_get_signatory(ctx, verifier, pdf, field);
	char *signer = NULL, *contents = NULL;
	size_t len;
	fz_var(signer);
	fz_var(contents);
	fz_try(ctx)
	{
		signer = dn ? pdf_signature_format_distinguished_name(ctx, dn) : NULL;
		fz_write_printf(ctx, out, ",\"digest\":\"%s\"", digest_name(pdf_check_digest(ctx, verifier, pdf, field)));
		fz_write_printf(ctx, out, ",\"later\":%d", pdf_signature_incremental_change_since_signing(ctx, pdf, field));
		fz_write_printf(ctx, out, ",\"broken\":%d", pdf_validate_signature(ctx, pdf, field));
		fz_write_string(ctx, out, ",\"signer\":");
		ely_json_string(ctx, out, signer ? signer : "");
		fz_write_string(ctx, out, ",\"common\":");
		ely_json_string(ctx, out, dn && dn->cn ? dn->cn : "");
		put_text(ctx, out, "date", pdf_dict_get(ctx, v, PDF_NAME(M)));
		put_text(ctx, out, "reason", pdf_dict_get(ctx, v, PDF_NAME(Reason)));
		put_text(ctx, out, "location", pdf_dict_get(ctx, v, PDF_NAME(Location)));
		len = pdf_signature_contents(ctx, pdf, field, &contents);
		fz_write_string(ctx, out, ",\"certs\":");
		put_certs(ctx, out, contents, len);
	}
	fz_always(ctx)
	{
		fz_free(ctx, contents);
		fz_free(ctx, signer);
		pdf_signature_drop_distinguished_name(ctx, dn);
	}
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* A broken signature reports why; the others still list. */
static void put_checked(fz_context *ctx, fz_output *out, pdf_document *pdf, pdf_obj *field, pdf_pkcs7_verifier *verifier)
{
	fz_buffer *buf = fz_new_buffer(ctx, 256);
	fz_output *part = NULL;
	char why[256] = "";
	fz_var(part);
	fz_try(ctx)
	{
		part = fz_new_output_with_buffer(ctx, buf);
		put_signed(ctx, part, pdf, field, verifier);
		fz_close_output(ctx, part);
	}
	fz_always(ctx)
		fz_drop_output(ctx, part);
	fz_catch(ctx)
	{
		fz_strlcpy(why, fz_caught_message(ctx), sizeof why);
		fz_ignore_error(ctx);
	}
	fz_try(ctx)
	{
		if (*why)
		{
			fz_write_string(ctx, out, ",\"digest\":\"unreadable\",\"error\":");
			ely_json_string(ctx, out, why);
		}
		else
			fz_write_buffer(ctx, out, buf);
	}
	fz_always(ctx)
		fz_drop_buffer(ctx, buf);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* A widget's page and page-space box; -1 for none. */
static int placed(fz_context *ctx, pdf_document *pdf, pdf_obj *field, fz_rect *box)
{
	pdf_obj *widget = pdf_name_eq(ctx, pdf_dict_get(ctx, field, PDF_NAME(Subtype)), PDF_NAME(Widget)) ? field :
		pdf_array_get(ctx, pdf_dict_get(ctx, field, PDF_NAME(Kids)), 0);
	pdf_obj *page = pdf_dict_get(ctx, widget, PDF_NAME(P));
	fz_matrix ctm;
	int i, k, number = page ? pdf_lookup_page_number(ctx, pdf, page) : -1;
	/* /P is optional; the page's /Annots tell. */
	for (i = 0; number < 0 && i < pdf_count_pages(ctx, pdf); i++)
	{
		pdf_obj *annots = pdf_dict_get(ctx, pdf_lookup_page_obj(ctx, pdf, i), PDF_NAME(Annots));
		for (k = 0; k < pdf_array_len(ctx, annots); k++)
			if (pdf_to_num(ctx, pdf_array_get(ctx, annots, k)) == pdf_to_num(ctx, widget))
				number = i, page = pdf_lookup_page_obj(ctx, pdf, i);
	}
	*box = fz_empty_rect;
	if (number >= 0)
	{
		pdf_page_obj_transform(ctx, page, NULL, &ctm);
		*box = fz_transform_rect(pdf_dict_get_rect(ctx, widget, PDF_NAME(Rect)), ctm);
	}
	return number;
}

static void put_field(fz_context *ctx, fz_output *out, pdf_document *pdf, pdf_obj *field, pdf_pkcs7_verifier *verifier, int *first)
{
	fz_rect r;
	int number = placed(ctx, pdf, field, &r), signd = pdf_signature_is_signed(ctx, pdf, field);
	char *name = pdf_load_field_name(ctx, field);
	fz_var(number);
	fz_var(r);
	fz_try(ctx)
	{
		fz_write_printf(ctx, out, "%s{\"page\":%d,\"object\":%d,\"rect\":[%g,%g,%g,%g],\"signed\":%s,\"name\":",
			*first ? "" : ",", number, pdf_to_num(ctx, field), r.x0, r.y0, r.x1, r.y1, signd ? "true" : "false");
		*first = 0;
		ely_json_string(ctx, out, name);
		if (signd)
			put_checked(ctx, out, pdf, field, verifier);
		fz_write_byte(ctx, out, '}');
	}
	fz_always(ctx)
		fz_free(ctx, name);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* Signature fields in the form tree, placed or not. */
static void put_tree(fz_context *ctx, fz_output *out, pdf_document *pdf, pdf_obj *node, pdf_pkcs7_verifier *verifier, int *first)
{
	pdf_obj *kids = pdf_dict_get(ctx, node, PDF_NAME(Kids));
	int i, leaf = 1;
	if (pdf_mark_obj(ctx, node))
		return;
	fz_try(ctx)
	{
		for (i = 0; i < pdf_array_len(ctx, kids); i++)
			leaf &= pdf_dict_get(ctx, pdf_array_get(ctx, kids, i), PDF_NAME(T)) == NULL;
		if (!leaf)
			for (i = 0; i < pdf_array_len(ctx, kids); i++)
				put_tree(ctx, out, pdf, pdf_array_get(ctx, kids, i), verifier, first);
		else if (pdf_name_eq(ctx, pdf_dict_get_inheritable(ctx, node, PDF_NAME(FT)), PDF_NAME(Sig)))
			put_field(ctx, out, pdf, node, verifier, first);
	}
	fz_always(ctx)
		pdf_unmark_obj(ctx, node);
	fz_catch(ctx)
		fz_rethrow(ctx);
}

/* Every signature field, as JSON, with its check. */
int ely_signatures(fz_context *ctx, fz_document *doc, unsigned char **data, size_t *len)
{
	pdf_pkcs7_verifier *verifier = NULL;
	fz_buffer *buf = NULL;
	fz_output *out = NULL;
	fz_var(verifier);
	fz_var(buf);
	fz_var(out);
	fz_try(ctx)
	{
		pdf_document *pdf = ely_pdf_of(ctx, doc);
		pdf_obj *fields = pdf_dict_getp(ctx, pdf_trailer(ctx, pdf), "Root/AcroForm/Fields");
		int i, first = 1;
		verifier = pkcs7_openssl_new_verifier(ctx);
		buf = fz_new_buffer(ctx, 256);
		out = fz_new_output_with_buffer(ctx, buf);
		fz_write_byte(ctx, out, '[');
		for (i = 0; i < pdf_array_len(ctx, fields); i++)
			put_tree(ctx, out, pdf, pdf_array_get(ctx, fields, i), verifier, &first);
		fz_write_byte(ctx, out, ']');
		fz_close_output(ctx, out);
		*len = fz_buffer_extract(ctx, buf, data);
	}
	fz_always(ctx)
	{
		fz_drop_output(ctx, out);
		fz_drop_buffer(ctx, buf);
		pdf_drop_verifier(ctx, verifier);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* An empty field by number, or a new one. */
static pdf_annot *sign_field(fz_context *ctx, pdf_document *pdf, pdf_page *page, int object, const float *rect, const char *name)
{
	pdf_obj *form = pdf_dict_getp(ctx, pdf_trailer(ctx, pdf), "Root/AcroForm/Fields");
	pdf_annot *widget;
	if (object >= 0)
	{
		widget = ely_find_annot(ctx, page, object, 1);
		if (pdf_widget_type(ctx, widget) != PDF_WIDGET_TYPE_SIGNATURE)
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "field %d is not a signature field", object);
		if (pdf_signature_is_signed(ctx, pdf, pdf_annot_obj(ctx, widget)))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "field %d is signed already", object);
		return widget;
	}
	if (!*name || strchr(name, '.') || (form && pdf_lookup_field(ctx, form, name)))
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a signature field needs a new name without dots");
	widget = pdf_create_signature_widget(ctx, page, (char *)name);
	pdf_set_annot_rect(ctx, widget, fz_make_rect(rect[0], rect[1], rect[2], rect[3]));
	return widget;
}

/* Signs into `raw` by an update, then checks it. */
int ely_sign(fz_context *ctx, fz_document *doc, const char *raw, const char *password, const char *pfx, const char *pfx_password,
	int number, int object, const float *rect, const char *name, const char *reason, const char *location)
{
	pdf_pkcs7_signer *signer = NULL;
	pdf_pkcs7_verifier *verifier = NULL;
	fz_document *copy = NULL, *signd = NULL;
	pdf_page *page = NULL;
	pdf_obj *made = NULL;
	fz_var(made);
	fz_var(signer);
	fz_var(verifier);
	fz_var(copy);
	fz_var(signd);
	fz_var(page);
	fz_try(ctx)
	{
		pdf_write_options update = pdf_default_write_options;
		pdf_document *pdf;
		pdf_annot *widget;
		int field;
		signer = pkcs7_openssl_read_pfx(ctx, pfx, pfx_password);
		pdf_save_snapshot(ctx, ely_pdf_of(ctx, doc), raw);
		copy = fz_open_document(ctx, raw);
		if (fz_needs_password(ctx, copy) && !fz_authenticate_password(ctx, copy, password))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "the copy refused its password");
		pdf = ely_pdf_of(ctx, copy);
		page = pdf_load_page(ctx, pdf, number);
		widget = sign_field(ctx, pdf, page, object, rect, name);
		field = pdf_to_num(ctx, pdf_annot_obj(ctx, widget));
		pdf_sign_signature(ctx, widget, signer, PDF_SIGNATURE_SHOW_LABELS | PDF_SIGNATURE_SHOW_DATE | PDF_SIGNATURE_SHOW_TEXT_NAME, NULL, reason, location);
		/* Earlier signatures cover earlier bytes; only append. */
		update.do_incremental = 1;
		pdf_save_document(ctx, pdf, raw, &update);
		fz_drop_page(ctx, (fz_page *)page);
		page = NULL;
		signd = fz_open_document(ctx, raw);
		if (fz_needs_password(ctx, signd) && !fz_authenticate_password(ctx, signd, password))
			fz_throw(ctx, FZ_ERROR_ARGUMENT, "the signed copy refused its password");
		verifier = pkcs7_openssl_new_verifier(ctx);
		made = pdf_new_indirect(ctx, ely_pdf_of(ctx, signd), field, 0);
		if (pdf_check_digest(ctx, verifier, ely_pdf_of(ctx, signd), made) != PDF_SIGNATURE_ERROR_OKAY)
			fz_throw(ctx, FZ_ERROR_FORMAT, "the new signature does not check");
	}
	fz_always(ctx)
	{
		pdf_drop_obj(ctx, made);
		fz_drop_page(ctx, (fz_page *)page);
		fz_drop_document(ctx, signd);
		fz_drop_document(ctx, copy);
		pdf_drop_verifier(ctx, verifier);
		pdf_drop_signer(ctx, signer);
	}
	fz_catch(ctx)
		return ely_fail(ctx);
	return 0;
}

/* Signed fields under `field`; with `drop`, their values go. */
static int signed_under(fz_context *ctx, pdf_obj *field, int drop)
{
	pdf_obj *kids = pdf_dict_get(ctx, field, PDF_NAME(Kids));
	int i, n = 0;
	if (pdf_mark_obj(ctx, field))
		return 0;
	fz_try(ctx)
	{
		if (pdf_name_eq(ctx, pdf_dict_get_inheritable(ctx, field, PDF_NAME(FT)), PDF_NAME(Sig)) &&
			pdf_is_dict(ctx, pdf_dict_get(ctx, field, PDF_NAME(V))))
		{
			n++;
			if (drop)
			{
				/* The drawn signature would still claim it. */
				pdf_dict_del(ctx, field, PDF_NAME(V));
				pdf_dict_del(ctx, field, PDF_NAME(AP));
				/* Its own lock went with it; it can sign again. */
				pdf_dict_put_int(ctx, field, PDF_NAME(Ff), pdf_dict_get_int(ctx, field, PDF_NAME(Ff)) & ~PDF_FIELD_IS_READ_ONLY);
				for (i = 0; i < pdf_array_len(ctx, kids); i++)
					pdf_dict_del(ctx, pdf_array_get(ctx, kids, i), PDF_NAME(AP));
			}
		}
		for (i = 0; i < pdf_array_len(ctx, kids); i++)
			n += signed_under(ctx, pdf_array_get(ctx, kids, i), drop);
	}
	fz_always(ctx)
		pdf_unmark_obj(ctx, field);
	fz_catch(ctx)
		fz_rethrow(ctx);
	return n;
}

/* SigFlags bit 2; MuPDF keeps its name private. */
enum { APPEND_ONLY = 2 };

static int signed_fields(fz_context *ctx, pdf_document *pdf, int drop)
{
	pdf_obj *form = pdf_dict_getp(ctx, pdf_trailer(ctx, pdf), "Root/AcroForm");
	pdf_obj *fields = pdf_dict_get(ctx, form, PDF_NAME(Fields));
	int i, n = 0;
	for (i = 0; i < pdf_array_len(ctx, fields); i++)
		n += signed_under(ctx, pdf_array_get(ctx, fields, i), drop);
	if (drop && n)
		pdf_dict_put_int(ctx, form, PDF_NAME(SigFlags), pdf_dict_get_int(ctx, form, PDF_NAME(SigFlags)) & ~APPEND_ONLY);
	return n;
}

/* Ely's whole-file writes break signatures, so carry none. */
void ely_unsign(fz_context *ctx, pdf_document *pdf)
{
	signed_fields(ctx, pdf, 1);
}

/* How many signed fields the form holds. */
int ely_signed(fz_context *ctx, fz_document *doc)
{
	int n = 0;
	fz_var(n);
	fz_try(ctx)
		n = signed_fields(ctx, ely_pdf_of(ctx, doc), 0);
	fz_catch(ctx)
		return ely_fail(ctx);
	return n;
}
