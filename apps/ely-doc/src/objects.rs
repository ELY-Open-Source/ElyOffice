//! Insert text box and basic shapes (DOC-08).

use gpui::{Context, Div, InteractiveElement as _, MenuItem, Window, actions};

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
    root.on_action(cx.listener(Docs::on_text_box))
        .on_action(cx.listener(Docs::on_rectangle))
        .on_action(cx.listener(Docs::on_ellipse))
        .on_action(cx.listener(Docs::on_triangle))
}

impl Docs {
    pub(crate) fn on_text_box(&mut self, _: &InsertTextBox, _: &mut Window, cx: &mut Context<Self>) {
        self.run_active("draw.text_box", &["true"], cx);
    }

    pub(crate) fn on_rectangle(&mut self, _: &InsertRectangle, _: &mut Window, cx: &mut Context<Self>) {
        self.run_active("draw.shape", &["rectangle", "true"], cx);
    }

    pub(crate) fn on_ellipse(&mut self, _: &InsertEllipse, _: &mut Window, cx: &mut Context<Self>) {
        self.run_active("draw.shape", &["ellipse", "true"], cx);
    }

    pub(crate) fn on_triangle(&mut self, _: &InsertTriangle, _: &mut Window, cx: &mut Context<Self>) {
        self.run_active("draw.shape", &["isosceles-triangle", "true"], cx);
    }
}
