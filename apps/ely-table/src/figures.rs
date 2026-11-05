//! The kernel's status texts in the interface language (TABLE-03, 05).

use gpui::{App, SharedString};

use crate::book::Book;

/// The kernel's figure names, in the order it writes them.
const FIGURES: [(&str, &str); 7] = [
    ("Average", "table.figure.average"),
    ("Count", "table.figure.count"),
    ("CountA", "table.figure.counta"),
    ("Max", "table.figure.max"),
    ("Min", "table.figure.min"),
    ("Sum", "table.figure.sum"),
    ("Selection count", "table.figure.selected"),
];

/// The kernel's explanations of a cell's error.
const ERRORS: [(&str, &str); 25] = [
    ("Error: Division by zero", "table.error.div_zero"),
    ("Error: No value", "table.error.no_value"),
    ("Error: Not a valid reference", "table.error.no_ref"),
    ("Error: Invalid name", "table.error.no_name"),
    ("Error: Invalid numeric value", "table.error.number"),
    ("Error: Value not available", "table.error.not_available"),
    ("Error: Add-in not found", "table.error.no_addin"),
    ("Error: Macro not found", "table.error.no_macro"),
    ("Internal syntactical error", "table.error.syntax"),
    ("Error: No code or intersection", "table.error.no_code"),
    ("Error: Invalid argument", "table.error.argument"),
    ("Error in parameter list", "table.error.parameters"),
    ("Error: Invalid character", "table.error.character"),
    ("Error: in bracketing", "table.error.brackets"),
    ("Error: Operator missing", "table.error.operator"),
    ("Error: Variable missing", "table.error.variable"),
    ("Error: Formula overflow", "table.error.formula_overflow"),
    ("Error: String overflow", "table.error.string_overflow"),
    ("Error: Internal overflow", "table.error.internal_overflow"),
    ("Error: Array or matrix size", "table.error.array_size"),
    ("Error: Circular reference", "table.error.circular"),
    (
        "Error: Calculation does not converge",
        "table.error.converge",
    ),
    (
        "Nested arrays are not supported.",
        "table.error.nested_array",
    ),
    (
        "Unsupported inline array content.",
        "table.error.array_content",
    ),
    ("External content disabled.", "table.error.external"),
];

/// The kernel's cell status: its error, else the selection figures.
pub fn label(status: &str, cx: &App) -> SharedString {
    if let Some((_, key)) = ERRORS.iter().find(|(known, _)| *known == status) {
        return Book::text(cx, key, &[]);
    }
    let Some(parts) = fields(status) else {
        // Values may hold the document's text; none is logged.
        log::warn!("table: a kernel status of {} chars names no figure", status.chars().count());
        return status.to_string().into();
    };
    let parts: Vec<String> = parts.into_iter().map(|(key, value)| Book::text(cx, key, &[("value", value.trim())]).to_string()).collect();
    parts.join("  ").into()
}

/// The figure from `from` on that `rest` opens.
fn named(rest: &str, from: usize) -> Option<(usize, &'static str, &str)> {
    FIGURES.iter().enumerate().skip(from).find_map(|(at, (name, key))| Some((at, *key, rest.strip_prefix(name)?.strip_prefix(": ")?)))
}

/// The figures; a value runs to a later name.
fn fields(status: &str) -> Option<Vec<(&'static str, &str)>> {
    let (mut at, mut key, mut value) = named(status, 0)?;
    let mut out = Vec::new();
    loop {
        let next = value.match_indices("; ").find_map(|(cut, _)| Some((cut, named(&value[cut + 2..], at + 1)?)));
        let Some((cut, found)) = next else {
            out.push((key, value));
            return Some(out);
        };
        out.push((key, &value[..cut]));
        (at, key, value) = found;
    }
}

#[cfg(test)]
mod tests {
    #[gpui::test]
    fn spanish_status_translates_labels_without_changing_values(cx: &mut gpui::TestAppContext) {
        cx.update(|app| {
            ely_ui::i18n::install_as("es-ES", app);
            assert_eq!(super::label("Average: 21; Sum: 42", app), "Promedio 21  Suma 42");
            assert_eq!(super::label("Sum: $1,234.50", app), "Suma $1,234.50");
            for (english, key) in super::ERRORS {
                assert_eq!(super::label(english, app), super::Book::text(app, key, &[]));
            }
        });
    }

    #[gpui::test]
    fn vietnamese_status_takes_libreoffice_words(cx: &mut gpui::TestAppContext) {
        cx.update(|app| {
            ely_ui::i18n::install_as("vi-VN", app);
            assert_eq!(super::label("Average: 21; Max: 30; Sum: 42", app), "Trung bình 21  Đại 30  Tổng 42");
            assert_eq!(super::label("Error: Division by zero", app), "Lỗi: chia cho không");
        });
    }

    #[test]
    fn kernel_status_takes_the_interface_language() {
        let cx = gpui::TestAppContext::single();
        cx.update(|app| {
            ely_ui::settings::install_defaults(app);
            ely_ui::i18n::install(app);
            let label = super::label("Average: 21; Sum: 42", app);
            let expect = format!(
                "{}  {}",
                super::Book::text(app, "table.figure.average", &[("value", "21")]),
                super::Book::text(app, "table.figure.sum", &[("value", "42")])
            );
            assert_eq!(label.as_ref(), expect, "known names relabel");
            assert_eq!(super::label("Odd: 1", app).as_ref(), "Odd: 1", "an unknown status stays");
            // The kernel's formatted values pass through untouched.
            ely_ui::i18n::install_as("en-US", app);
            let custom = super::label("Average: 8128.00 kg; net; Sum: 16256.00 kg; net", app);
            assert_eq!(custom.as_ref(), "Average 8128.00 kg; net  Sum 16256.00 kg; net", "a format's own semicolons stay");
            // A literal shaped like a field is still the value.
            let named = super::label("Average: 8128.00 kg; Net: ok; Sum: 16256.00 kg; Net: ok", app);
            assert_eq!(named.as_ref(), "Average 8128.00 kg; Net: ok  Sum 16256.00 kg; Net: ok", "only figure names start a field");
            let earlier = super::label("Sum: 1 x; Average: 2; Max: 3", app);
            assert_eq!(earlier.as_ref(), "Sum 1 x; Average: 2; Max: 3", "an earlier figure's name is the value's text");
            for value in ["8128.00", "8128.33333333333", "0.0000000001", "1234567890.25", "$1,234.50"] {
                assert_eq!(super::label(&format!("Sum: {value}"), app).as_ref(), format!("Sum {value}"));
            }
            for (english, key) in super::ERRORS {
                assert_eq!(super::label(english, app), super::Book::text(app, key, &[]));
            }
        });
    }
}
