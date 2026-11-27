//! New, and ElyDoc's own dialogs (DOC-01).

use std::rc::Rc;

use ely_gpui_component::{
    buttons::Button,
    documents::{Template as Pick, TemplatePicker},
    forms::TextInput,
    overlays::Dialog as Card,
};
use ely_office::prelude::*;
use ely_ui::commands::New;
use gpui::{AnyElement, App, Context, Entity, IntoElement, ParentElement as _, Window};

use crate::docs::Docs;
use crate::files::TEMPLATES;
use crate::refs::Ask;

pub enum Dialog {
    Templates,
    Table {
        columns: f64,
        rows: f64,
    },
    Section,
    /// Asks before deleting every comment (#148).
    DeleteComments {
        count: usize,
    },
    Symbols,
    Ask {
        ask: Ask,
        field: Entity<TextInput>,
    },
    Link {
        text: Entity<TextInput>,
        url: Entity<TextInput>,
    },
    Reference {
        marks: Vec<String>,
        mark: String,
        show: &'static str,
    },
    Output {
        tab: u64,
        print: bool,
        format: crate::output::Format,
        all: bool,
        pdf: ely_lok::Pdf,
    },
}

impl Docs {
    pub fn on_new(&mut self, _: &New, _: &mut Window, cx: &mut Context<Self>) {
        self.dialog = Some(Dialog::Templates);
        cx.notify();
    }

    /// The open dialog, if any.
    pub fn dialog_element(&mut self, _: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let this = cx.entity();
        let dismiss = {
            let this = this.clone();
            move |_: &mut Window, cx: &mut App| {
                this.update(cx, |docs, cx| {
                    docs.dialog = None;
                    cx.notify();
                })
            }
        };
        let card = match self.dialog.as_ref()? {
            Dialog::Templates => templates(this, dismiss, cx),
            Dialog::Section => {
                return Some(crate::insert::section_dialog(this, dismiss, cx).into_any_element());
            }
            Dialog::DeleteComments { count } => {
                return Some(crate::review::delete_comments_dialog(this, *count, dismiss, cx).into_any_element());
            }
            Dialog::Symbols => crate::symbols::symbol_dialog(this, dismiss, cx),
            Dialog::Ask { ask, field } => {
                let prompt = crate::refs::ask_dialog(this, ask, field, dismiss, cx);
                return Some(prompt.into_any_element());
            }
            Dialog::Link { text, url } => crate::refs::link_dialog(this, (text, url), dismiss, cx),
            Dialog::Reference { marks, mark, show } => {
                crate::refs::reference_dialog(this, (marks, mark, show), dismiss, cx)
            }
            Dialog::Output {
                tab,
                print,
                format,
                all,
                pdf,
            } => {
                let last = self
                    .tab(*tab)
                    .expect("a modal keeps its tab")
                    .view
                    .read(cx)
                    .pages()
                    .len();
                let last = u32::try_from(last).expect("a page count");
                crate::output::output_dialog(this, (*print, *format, *all, *pdf, last), dismiss, cx)
            }
            Dialog::Table { columns, rows } => {
                crate::insert::table_dialog(this, (*columns, *rows), dismiss, cx)
            }
        };
        Some(card.into_any_element())
    }
}

/// The New dialog: bundled templates and a local file (DOC-01).
fn templates(
    this: Entity<Docs>,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &mut Context<Docs>,
) -> Card {
    let picks: Vec<Pick> = TEMPLATES
        .iter()
        .map(|template| Pick {
            name: Docs::text(cx, &format!("doc.template.{}", template.key()), &[]),
            icon: "▤".into(),
            category: Docs::text(cx, "doc.template.category", &[]),
            description: Docs::text(cx, &format!("doc.template.{}.about", template.key()), &[]),
            markdown: template.preview(cx).into(),
        })
        .collect();
    let used = this.clone();
    let picker = TemplatePicker::new("doc-templates", Rc::new(picks)).on_use(move |ix, _, cx| {
        used.update(cx, |docs, cx| {
            docs.dialog = None;
            docs.new_from(TEMPLATES[ix], cx);
        })
    });
    let label = Docs::text(cx, "doc.new.from_file", &[]);
    Card::new("doc-new", Docs::text(cx, "doc.new.title", &[]), dismiss)
        .child(picker)
        .action(move |close| {
            Button::new("doc-new-file", label).on_click(move |_, window, cx| {
                close(window, cx);
                this.update(cx, |docs, cx| {
                    docs.pick(false, cx, |docs, path, cx| docs.new_from_file(&path, cx))
                });
            })
        })
}
