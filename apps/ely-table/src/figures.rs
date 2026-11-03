//! The kernel's status texts in the interface language (TABLE-03, 05).

use gpui::{App, SharedString};

use crate::book::Book;

/// The kernel's English names of the selection figures.
const FIGURES: [(&str, &str); 7] = [
    ("Sum", "table.figure.sum"),
    ("Selection count", "table.figure.selected"),
    ("Count", "table.figure.count"),
    ("CountA", "table.figure.counta"),
    ("Average", "table.figure.average"),
    ("Max", "table.figure.max"),
    ("Min", "table.figure.min"),
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
    let parts: Vec<String> = fields(status)
        .into_iter()
        .map(|part| {
            let known = part.split_once(": ").and_then(|(name, value)| {
                let (_, key) = FIGURES.iter().find(|(known, _)| *known == name)?;
                Some(Book::text(cx, key, &[("value", value.trim())]).to_string())
            });
            known.unwrap_or_else(|| {
                log::warn!("table: the kernel status {part:?} has no label");
                part.to_string()
            })
        })
        .collect();
    parts.join("  ").into()
}

/// The status split at each "Name: "; values keep semicolons.
fn fields(status: &str) -> Vec<&str> {
    let starts = |rest: &str| {
        rest.split_once(": ")
            .is_some_and(|(name, _)| !name.is_empty() && name.chars().all(|c| c.is_ascii_alphabetic() || c == ' '))
    };
    let mut out = Vec::new();
    let mut from = 0;
    for (at, _) in status.match_indices("; ") {
        if starts(&status[at + 2..]) {
            out.push(&status[from..at]);
            from = at + 2;
        }
    }
    out.push(&status[from..]);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn kernel_status_takes_the_interface_language() {
        let cx = gpui::TestAppContext::single();
        cx.update(|app| {
            ely_ui::settings::install_defaults(app);
            ely_ui::i18n::install(app);
            let label = super::label("Average: 21; Sum: 42; Odd: 1", app);
            let expect = format!(
                "{}  {}  Odd: 1",
                super::Book::text(app, "table.figure.average", &[("value", "21")]),
                super::Book::text(app, "table.figure.sum", &[("value", "42")])
            );
            assert_eq!(label.as_ref(), expect, "known names relabel, others stay");
            // The kernel's formatted values pass through untouched.
            ely_ui::i18n::install_as("en-US", app);
            let custom = super::label("Average: 8128.00 kg; net; Sum: 16256.00 kg; net", app);
            assert_eq!(custom.as_ref(), "Average 8128.00 kg; net  Sum 16256.00 kg; net", "a format's own semicolons stay");
            for value in ["8128.00", "8128.33333333333", "0.0000000001", "1234567890.25", "$1,234.50"] {
                assert_eq!(super::label(&format!("Sum: {value}"), app).as_ref(), format!("Sum {value}"));
            }
            for (english, key) in super::ERRORS {
                assert_eq!(super::label(english, app), super::Book::text(app, key, &[]));
            }
        });
    }
}
