//! Annotation and form menus, and the panel's changes (PDF-05, PDF-06).

use ely_gpui_component::primitives::Severity;
use ely_office::Tabs as _;
use ely_pdf_engine::{FieldKind, LinkChange, NewAnnot, NewField, Shape};
use gpui::{App, Context, Div, InteractiveElement as _, Menu, MenuItem, actions};

use crate::annotate::{Pick, around};
use crate::content::Tool;
use crate::viewer::Viewer;

actions!(
    pdf,
    [
        Highlight,
        Underline,
        StrikeOut,
        AddNote,
        DrawRectangle,
        DrawEllipse,
        DrawLine,
        DrawFreehand,
        AddStamp,
        AddTextField,
        AddCheckbox,
        AddRadioButton,
        AddChoice
    ]
);

/// A stamp's size, in points.
const STAMP: (f32, f32) = (190.0, 50.0);

/// What the note panel asks for.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Note(String),
    Stamp(String),
    Contents(String),
    Color(u32),
    Delete,
    /// A field's name and options.
    Design(String, Vec<String>),
    Fill(String),
    /// A link target as typed.
    Target(String),
}

/// The Annotate and Form menus.
pub fn menus(cx: &App) -> [Menu; 2] {
    let label = |key: &str| Viewer::text(cx, key, &[]);
    [
        Menu::new(label("menu.annotate")).items([
            MenuItem::action(label("pdf.tool.highlight"), Highlight),
            MenuItem::action(label("pdf.tool.underline"), Underline),
            MenuItem::action(label("pdf.tool.strikeout"), StrikeOut),
            MenuItem::separator(),
            MenuItem::action(label("pdf.tool.note"), AddNote),
            MenuItem::action(label("pdf.tool.stamp"), AddStamp),
            MenuItem::separator(),
            MenuItem::action(label("pdf.tool.square"), DrawRectangle),
            MenuItem::action(label("pdf.tool.circle"), DrawEllipse),
            MenuItem::action(label("pdf.tool.line"), DrawLine),
            MenuItem::action(label("pdf.tool.ink"), DrawFreehand),
        ]),
        Menu::new(label("menu.form")).items([
            MenuItem::action(label("pdf.tool.field_text"), AddTextField),
            MenuItem::action(label("pdf.tool.field_check"), AddCheckbox),
            MenuItem::action(label("pdf.tool.field_radio"), AddRadioButton),
            MenuItem::action(label("pdf.tool.field_choice"), AddChoice),
        ]),
    ]
}

impl Viewer {
    /// Makes `change` if tab `tab` is still at `version`.
    pub fn change_note(&mut self, (tab, version, pick): (u64, u64, Pick), change: Change, cx: &mut Context<Self>) {
        let Some(view) = self.tab(tab).map(|tab| tab.view.clone()) else {
            let text = Self::text(cx, "pdf.object.changed", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        self.on_same(&view, version, cx, move |view, cx| {
            let note = |page, shape, contents| NewAnnot { page, shape, rgb: None, contents, author: String::new() };
            match (pick, change) {
                (Pick::NewNote { page, at }, Change::Note(text)) => view.annotate("pdf.doing.add_note", vec![note(page, Shape::Note(at), text)], cx),
                (Pick::NewStamp { page, at }, Change::Stamp(icon)) => {
                    let rect = around(at, STAMP, view.sizes[page]);
                    view.annotate("pdf.doing.add_stamp", vec![note(page, Shape::Stamp(rect, icon), String::new())], cx)
                }
                (Pick::Annot { page, id }, Change::Contents(text)) => {
                    view.edit("pdf.doing.change_annot", move |w, doc| w.change_annot(doc, page, id, None, Some(&text)), cx)
                }
                (Pick::Annot { page, id }, Change::Color(rgb)) => {
                    view.edit("pdf.doing.colour_annot", move |w, doc| w.change_annot(doc, page, id, Some(rgb), None), cx)
                }
                (Pick::Annot { page, id }, Change::Delete) => view.edit("pdf.doing.delete_annot", move |w, doc| w.delete_annot(doc, page, id), cx),
                (Pick::Field { page, id }, Change::Delete) => view.edit("pdf.doing.delete_field", move |w, doc| w.delete_field(doc, page, id), cx),
                (Pick::Field { page, id }, Change::Fill(value)) => view.edit("pdf.doing.fill", move |w, doc| w.fill(doc, page, id, &value), cx),
                (Pick::Field { page, id }, Change::Design(name, options)) => {
                    let Some(field) = view.notes.get(&page).and_then(|m| m.fields.iter().find(|f| f.id == id)) else {
                        return log::error!("pdf: a picked field is not read");
                    };
                    let renamed = (field.leaf() != name).then_some(name);
                    let choice = matches!(field.kind, FieldKind::Choice | FieldKind::List);
                    let listed = (choice && field.options != options).then_some(options);
                    if renamed.is_none() && listed.is_none() {
                        return log::info!("pdf: the field is as it was");
                    }
                    view.edit(
                        "pdf.doing.change_field",
                        move |w, doc| w.change_field(doc, page, id, renamed.as_deref(), listed.as_deref()),
                        cx,
                    )
                }
                (Pick::NewField { page, rect, kind }, Change::Design(name, options)) => {
                    let field = NewField { kind, name, rect, options };
                    view.edit("pdf.doing.add_field", move |w, doc| w.add_field(doc, page, &field), cx)
                }
                (Pick::NewLink { page, rect }, Change::Target(typed)) => {
                    let Some(to) = crate::links::parse(&typed, view.sizes.len()) else {
                        return bad_target(cx);
                    };
                    view.edit("pdf.doing.add_link", move |w, doc| w.change_link(doc, page, &LinkChange::Add { rect, to }), cx)
                }
                (Pick::Link { page, index }, change) => {
                    let Some(expect) = view.links.get(&page).and_then(|links| links.get(index)).map(|link| link.uri.clone()) else {
                        return log::error!("pdf: a picked link is not read");
                    };
                    let change = match change {
                        Change::Delete => LinkChange::Delete { index, expect },
                        Change::Target(typed) => match crate::links::parse(&typed, view.sizes.len()) {
                            Some(to) => LinkChange::Edit { index, expect, to },
                            None => return bad_target(cx),
                        },
                        other => return log::error!("pdf: {other:?} does not fit a link"),
                    };
                    view.edit("pdf.doing.change_link", move |w, doc| w.change_link(doc, page, &change), cx)
                }
                (pick, change) => log::error!("pdf: {change:?} does not fit {pick:?}"),
            }
        });
    }
}

/// Says a typed link target is neither page nor address.
fn bad_target(cx: &mut Context<crate::view::PdfView>) {
    let text = Viewer::text(cx, "pdf.link.bad", &[]);
    cx.emit(crate::view::Failed(text.to_string()));
}

/// Wires the annotation and form actions onto the root.
pub fn on(root: Div, cx: &mut Context<Viewer>) -> Div {
    let tool = |tool: Tool| move |viewer: &mut Viewer, cx: &mut Context<Viewer>| viewer.toggle_tool(tool, cx);
    let (highlight, underline, strike, note, stamp) =
        (tool(Tool::Highlight), tool(Tool::Underline), tool(Tool::StrikeOut), tool(Tool::Note), tool(Tool::Stamp));
    let (square, circle, line, ink) = (tool(Tool::Square), tool(Tool::Circle), tool(Tool::Line), tool(Tool::Ink));
    let (text, check, radio, choice) = (
        tool(Tool::Field(FieldKind::Text)),
        tool(Tool::Field(FieldKind::Checkbox)),
        tool(Tool::Field(FieldKind::Radio)),
        tool(Tool::Field(FieldKind::Choice)),
    );
    root.on_action(cx.listener(move |v, _: &Highlight, _, cx| highlight(v, cx)))
        .on_action(cx.listener(move |v, _: &Underline, _, cx| underline(v, cx)))
        .on_action(cx.listener(move |v, _: &StrikeOut, _, cx| strike(v, cx)))
        .on_action(cx.listener(move |v, _: &AddNote, _, cx| note(v, cx)))
        .on_action(cx.listener(move |v, _: &AddStamp, _, cx| stamp(v, cx)))
        .on_action(cx.listener(move |v, _: &DrawRectangle, _, cx| square(v, cx)))
        .on_action(cx.listener(move |v, _: &DrawEllipse, _, cx| circle(v, cx)))
        .on_action(cx.listener(move |v, _: &DrawLine, _, cx| line(v, cx)))
        .on_action(cx.listener(move |v, _: &DrawFreehand, _, cx| ink(v, cx)))
        .on_action(cx.listener(move |v, _: &AddTextField, _, cx| text(v, cx)))
        .on_action(cx.listener(move |v, _: &AddCheckbox, _, cx| check(v, cx)))
        .on_action(cx.listener(move |v, _: &AddRadioButton, _, cx| radio(v, cx)))
        .on_action(cx.listener(move |v, _: &AddChoice, _, cx| choice(v, cx)))
}
