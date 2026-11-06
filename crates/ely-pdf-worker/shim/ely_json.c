/* JSON in and out of the worker. */

#include <math.h>

#include "ely_shim.h"

/* JSON's number form, not PDF's; non-finite fails. */
void ely_json_number(fz_context *ctx, fz_output *out, double v)
{
	fz_json number;
	if (!isfinite(v))
		fz_throw(ctx, FZ_ERROR_FORMAT, "a non-finite number in JSON");
	number.type = FZ_JSON_NUMBER;
	number.u.number = v;
	fz_write_json(ctx, out, &number);
}

void ely_json_rect(fz_context *ctx, fz_output *out, fz_rect r)
{
	fz_write_byte(ctx, out, '[');
	ely_json_number(ctx, out, r.x0);
	fz_write_byte(ctx, out, ',');
	ely_json_number(ctx, out, r.y0);
	fz_write_byte(ctx, out, ',');
	ely_json_number(ctx, out, r.x1);
	fz_write_byte(ctx, out, ',');
	ely_json_number(ctx, out, r.y1);
	fz_write_byte(ctx, out, ']');
}

/* Valid UTF-8 always: a stray byte becomes U+FFFD. */
void ely_json_string(fz_context *ctx, fz_output *out, const char *s)
{
	char utf8[FZ_UTFMAX];
	int rune;
	fz_write_byte(ctx, out, '"');
	while (*s)
	{
		s += fz_chartorune(&rune, s);
		/* A surrogate pair joins; a lone one is no character. */
		if (rune >= 0xD800 && rune < 0xDC00)
		{
			int low;
			int n = fz_chartorune(&low, s);
			if (low >= 0xDC00 && low < 0xE000)
			{
				rune = 0x10000 + ((rune - 0xD800) << 10) + (low - 0xDC00);
				s += n;
			}
		}
		if ((rune >= 0xD800 && rune < 0xE000) || rune > 0x10FFFF)
			rune = FZ_REPLACEMENT_CHARACTER;
		if (rune == '"' || rune == '\\')
			fz_write_printf(ctx, out, "\\%c", rune);
		else if (rune < 32)
			fz_write_printf(ctx, out, "\\u%04x", rune);
		else
			fz_write_data(ctx, out, utf8, fz_runetochar(utf8, rune));
	}
	fz_write_byte(ctx, out, '"');
}

fz_json *ely_member(fz_context *ctx, fz_json *obj, const char *key, int type)
{
	fz_json *value = fz_json_object_get(ctx, obj, key);
	if (value && value->type != type)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a bad %s", key);
	return value;
}

float ely_number(fz_context *ctx, fz_json *array, int at)
{
	fz_json *value = fz_json_array_get(ctx, array, at);
	if (!value || value->type != FZ_JSON_NUMBER)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a bad number list");
	return (float)value->u.number;
}

/* Four numbers, at least `least` wide and high. */
fz_rect ely_rect(fz_context *ctx, fz_json *array, float least)
{
	fz_rect r;
	if (!array || array->type != FZ_JSON_ARRAY || fz_json_array_length(ctx, array) != 4)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a bad rectangle");
	r = fz_make_rect(ely_number(ctx, array, 0), ely_number(ctx, array, 1), ely_number(ctx, array, 2),
		ely_number(ctx, array, 3));
	if (r.x1 - r.x0 < least || r.y1 - r.y0 < least)
		fz_throw(ctx, FZ_ERROR_ARGUMENT, "a rectangle too small");
	return r;
}
