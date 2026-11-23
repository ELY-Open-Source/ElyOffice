//! Insert text box and basic shapes (DOC-08).

use gpui::{Context, Div, InteractiveElement as _, MenuItem, actions};

use crate::docs::Docs;

actions!(doc, [InsertTextBox, InsertRectangle, InsertEllipse, InsertTriangle]);

/// The Insert menu's drawing rows.
pub fn items(cx: &gpui::App) -> Vec<MenuItem> {
    let label = |key: &str| Docs::text(cx, key, &[]);
    vec![
        MenuItem::action(label("doc.insert.text_box"), InsertTextBox),
        MenuItem::action(label("doc.insert.rectangle"), InsertRectangle),
        MenuItem::action(label("doc.insert.ellipse"), InsertEllipse),
        MenuItem::action(label("doc.insert.triangle"), InsertTriangle),
    ]
}

/// Binds each row on the window root.
pub fn bind(root: Div, cx: &mut Context<Docs>) -> Div {
    root.on_action(cx.listener(|docs: &mut Docs, _: &InsertTextBox, _, cx| {
        docs.run_active("draw.text_box", &["true"], cx)
    }))
    .on_action(cx.listener(|docs: &mut Docs, _: &InsertRectangle, _, cx| {
        docs.run_active("draw.shape", &["rectangle", "true"], cx)
    }))
    .on_action(cx.listener(|docs: &mut Docs, _: &InsertEllipse, _, cx| {
        docs.run_active("draw.shape", &["ellipse", "true"], cx)
    }))
    .on_action(cx.listener(|docs: &mut Docs, _: &InsertTriangle, _, cx| {
        docs.run_active("draw.shape", &["isosceles-triangle", "true"], cx)
    }))
}
