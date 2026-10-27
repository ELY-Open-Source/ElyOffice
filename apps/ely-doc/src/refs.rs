//! References, formulas and comments (DOC-09, 10, 11).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Choice, Input, Select, TextInput},
    overlays::{Dialog as Card, PromptDialog},
    primitives::Severity,
};
use gpui::{
    App, AppContext as _, Context, Entity, MenuItem, ParentElement as _, SharedString, Styled as _,
    Window, actions, div,
};

use crate::actions::Dialog;
use crate::docs::Docs;
use ely_office::prelude::*;

actions!(
    doc,
    [
        InsertContents,
        UpdateContents,
        InsertFootnote,
        InsertEndnote,
        InsertBookmark,
        InsertLink,
        InsertReference,
        InsertCaption,
        InsertFormula,
        InsertComment
    ]
);

/// What a one-line prompt asks for.
#[derive(Clone)]
pub enum Ask {
    Bookmark(Vec<String>),
    /// The caption's category, by program name.
    Caption(&'static str),
    /// True when it replaces the selected formula.
    Formula(bool),
    Comment,
}

/// The reference formats offered, by kernel number.
const SHOWN: [(&str, &str); 4] = [
    ("2", "doc.reference.text"),
    ("0", "doc.reference.page"),
    ("1", "doc.reference.chapter"),
    ("3", "doc.reference.direction"),
];

/// The Insert menu's reference entries.
pub fn items(cx: &App) -> Vec<MenuItem> {
    let label = |key: &str| Docs::text(cx, key, &[]);
    vec![
        MenuItem::separator(),
        MenuItem::action(label("doc.insert.contents"), InsertContents),
        MenuItem::action(label("doc.insert.update_contents"), UpdateContents),
        MenuItem::action(label("doc.insert.footnote"), InsertFootnote),
        MenuItem::action(label("doc.insert.endnote"), InsertEndnote),
        MenuItem::action(label("doc.insert.bookmark"), InsertBookmark),
        MenuItem::action(label("doc.insert.link"), InsertLink),
        MenuItem::action(label("doc.insert.reference"), InsertReference),
        MenuItem::action(label("doc.insert.caption"), InsertCaption),
        MenuItem::separator(),
        MenuItem::action(label("doc.insert.formula"), InsertFormula),
        MenuItem::action(label("doc.insert.comment"), InsertComment),
    ]
}

impl Docs {
    /// Reads a kernel list off the window thread, then `then`.
    fn read_then(
        &mut self,
        command: &'static str,
        then: impl FnOnce(&mut Self, String, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.active_tab() else {
            let text = Docs::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        let id = tab.id;
        self.dialog_asked += 1;
        let asked = self.dialog_asked;
        self.ask_in(
            id,
            move |worker, doc| worker.values(doc, command),
            move |docs, read, window, cx| {
                // A dialog opened since, or a later ask, wins.
                if docs.dialog_asked != asked || docs.dialog.is_some() || docs.core.dialog.is_some() {
                    return log::info!("doc: a dialog read for tab {id} lapsed");
                }
                match read {
                    Ok(json) => then(docs, json, window, cx),
                    Err(error) => docs.engine_failed(&error.to_string(), cx),
                }
            },
            cx,
        );
    }

    /// The bookmarks' names, then `then`.
    fn with_bookmarks(
        &mut self,
        then: impl FnOnce(&mut Self, Vec<String>, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        self.read_then(
            ".uno:Bookmarks",
            move |docs, json, window, cx| match ely_lok::review::bookmarks(&json) {
                Ok(marks) => then(docs, marks.into_iter().map(|mark| mark.name).collect(), window, cx),
                Err(error) => docs.engine_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    fn ask(&mut self, ask: Ask, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let field = cx.new(|cx| TextInput::new(window, cx));
        field.update(cx, |field, cx| field.set_text(text, cx));
        self.dialog = Some(Dialog::Ask { ask, field });
        cx.notify();
    }

    pub fn on_contents(&mut self, _: &InsertContents, _: &mut Window, cx: &mut Context<Self>) {
        let title = Docs::text(cx, "doc.contents.title", &[]);
        self.run_active("index.insert", &[&title], cx);
    }

    pub fn on_update(&mut self, _: &UpdateContents, _: &mut Window, cx: &mut Context<Self>) {
        self.run_active("index.update", &[], cx);
    }

    pub fn on_footnote(&mut self, _: &InsertFootnote, _: &mut Window, cx: &mut Context<Self>) {
        self.run_active("note.footnote", &[], cx);
    }

    pub fn on_endnote(&mut self, _: &InsertEndnote, _: &mut Window, cx: &mut Context<Self>) {
        self.run_active("note.endnote", &[], cx);
    }

    pub fn on_bookmark(&mut self, _: &InsertBookmark, _: &mut Window, cx: &mut Context<Self>) {
        self.with_bookmarks(|docs, taken, window, cx| docs.ask(Ask::Bookmark(taken), "", window, cx), cx);
    }

    pub fn on_link(&mut self, _: &InsertLink, window: &mut Window, cx: &mut Context<Self>) {
        let text = cx.new(|cx| TextInput::new(window, cx));
        let url = cx.new(|cx| TextInput::new(window, cx));
        cx.observe(&url, |_, _, cx| cx.notify()).detach();
        self.dialog = Some(Dialog::Link { text, url });
        cx.notify();
    }

    pub fn on_reference(&mut self, _: &InsertReference, _: &mut Window, cx: &mut Context<Self>) {
        self.with_bookmarks(
            |docs, marks, _, cx| {
                let Some(mark) = marks.first().cloned() else {
                    let text = Docs::text(cx, "doc.reference.none", &[]);
                    return docs.toast(text, Severity::Warning, cx);
                };
                docs.dialog = Some(Dialog::Reference { marks, mark, show: "2" });
                cx.notify();
            },
            cx,
        );
    }

    pub fn on_caption(&mut self, _: &InsertCaption, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            let text = Docs::text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        let view = tab.view.read(cx);
        let category = if view.graphic().is_some() {
            "Figure"
        } else if view.state(".uno:DeleteTable") == Some("enabled") {
            "Table"
        } else {
            let text = Docs::text(cx, "doc.caption.none", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        self.ask(Ask::Caption(category), "", window, cx);
    }

    pub fn on_formula(&mut self, _: &InsertFormula, _: &mut Window, cx: &mut Context<Self>) {
        self.read_then(
            ".uno:Formula",
            |docs, json, window, cx| match ely_lok::review::formula(&json) {
                Ok(Some(formula)) => docs.ask(Ask::Formula(true), &formula, window, cx),
                Ok(None) => docs.ask(Ask::Formula(false), "", window, cx),
                Err(error) => docs.engine_failed(&error.to_string(), cx),
            },
            cx,
        );
    }

    pub fn on_comment(&mut self, _: &InsertComment, window: &mut Window, cx: &mut Context<Self>) {
        self.ask(Ask::Comment, "", window, cx);
    }

    /// Sends the hyperlink dialog.
    pub(crate) fn insert_link(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Link { text, url }) = self.dialog.take() else {
            return;
        };
        let (words, address) = (
            text.read(cx).text().trim().to_string(),
            url.read(cx).text().trim().to_string(),
        );
        // No text shows the address.
        let shown = if words.is_empty() { &address } else { &words };
        self.run_active("link.insert", &[shown, &address], cx);
        cx.notify();
    }

    /// Sends the cross-reference dialog.
    pub(crate) fn insert_reference(&mut self, cx: &mut Context<Self>) {
        let Some(Dialog::Reference { mark, show, .. }) = self.dialog.take() else {
            return;
        };
        self.run_active("reference.insert", &["13", "2", &mark, show], cx);
        cx.notify();
    }

    /// Sends a prompt's answer and closes it.
    pub(crate) fn answer(&mut self, ask: &Ask, text: &str, cx: &mut Context<Self>) {
        self.dialog = None;
        match ask {
            Ask::Bookmark(_) => self.run_active("bookmark.insert", &[text], cx),
            Ask::Caption(category) => self.run_active("caption.insert", &[category, text], cx),
            Ask::Formula(_) => self.run_active("formula.insert", &[text], cx),
            Ask::Comment => {
                self.show_side(crate::docs::Side::Review, cx);
                self.run_active("comment.insert", &[text], cx);
            }
        }
        cx.notify();
    }
}

/// The one-line prompt for `ask`.
pub fn ask_dialog(
    this: Entity<Docs>,
    ask: &Ask,
    field: &Entity<TextInput>,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> PromptDialog {
    let text = |key: &str| Docs::text(cx, key, &[]);
    let (title, label) = match ask {
        Ask::Bookmark(_) => ("doc.bookmark.title", "doc.bookmark.name"),
        Ask::Caption(_) => ("doc.caption.title", "doc.caption.text"),
        Ask::Formula(true) => ("doc.formula.edit", "doc.formula.text"),
        Ask::Formula(false) => ("doc.formula.title", "doc.formula.text"),
        Ask::Comment => ("doc.comment.title", "doc.comment.text"),
    };
    let submit = match ask {
        Ask::Formula(true) => text("doc.formula.apply"),
        _ => text("doc.insert.insert"),
    };
    let reasons: Vec<(&str, SharedString)> = [
        "doc.bookmark.empty",
        "doc.bookmark.taken",
        "doc.caption.empty",
        "doc.formula.empty",
        "doc.comment.empty",
    ]
    .into_iter()
    .map(|key| (key, text(key)))
    .collect();
    let (asked, ask) = (ask.clone(), ask.clone());
    PromptDialog::new("doc-ask", text(title), field, dismiss)
        .label(text(label))
        .submit(submit)
        .check(move |value| {
            refuse(&asked, value.trim()).map_err(|key| {
                let (_, why) = reasons
                    .iter()
                    .find(|(known, _)| *known == key)
                    .expect("a reason");
                why.clone()
            })
        })
        .on_submit(move |value, _, cx| {
            let value = value.trim().to_string();
            this.update(cx, |docs, cx| docs.answer(&ask, &value, cx))
        })
}

/// Why a prompt's answer cannot go, by message key.
fn refuse(ask: &Ask, value: &str) -> Result<(), &'static str> {
    if let Ask::Bookmark(taken) = ask
        && taken.iter().any(|name| name == value)
    {
        return Err("doc.bookmark.taken");
    }
    if !value.is_empty() {
        return Ok(());
    }
    Err(match ask {
        Ask::Bookmark(_) => "doc.bookmark.empty",
        Ask::Caption(_) => "doc.caption.empty",
        Ask::Formula(_) => "doc.formula.empty",
        Ask::Comment => "doc.comment.empty",
    })
}

/// The hyperlink dialog: text and address.
pub fn link_dialog(
    this: Entity<Docs>,
    (text, url): (&Entity<TextInput>, &Entity<TextInput>),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let fields = DescriptionList::new()
        .item(
            Docs::text(cx, "doc.link.text", &[]),
            div().w_full().max_w_64().child(Input::new(text)),
        )
        .item(
            Docs::text(cx, "doc.link.url", &[]),
            div().w_full().max_w_64().child(Input::new(url)),
        );
    let label = Docs::text(cx, "doc.insert.insert", &[]);
    let empty = url.read(cx).text().trim().is_empty();
    Card::new(
        "doc-insert-link",
        Docs::text(cx, "doc.link.title", &[]),
        dismiss,
    )
    .child(fields)
    .action(ely_office::cancel(cx))
    .action(move |_| {
        Button::new("doc-insert-link-go", label)
            .variant(ButtonVariant::Primary)
            .disabled(empty)
            .on_click(move |_, _, cx| this.update(cx, |docs, cx| docs.insert_link(cx)))
    })
}

/// The cross-reference dialog: which bookmark, shown how.
pub fn reference_dialog(
    this: Entity<Docs>,
    (marks, mark, show): (&[String], &str, &'static str),
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Card {
    let pick = {
        let this = this.clone();
        let choices = marks
            .iter()
            .map(|name| Choice::new(name.clone(), name.clone()));
        Select::new("doc-reference-mark", choices)
            .label(Docs::text(cx, "doc.reference.target", &[]))
            .selected(mark.to_string())
            .on_change(move |name, _, cx| {
                this.update(cx, |docs, cx| {
                    if let Some(Dialog::Reference { mark, .. }) = &mut docs.dialog {
                        *mark = name.to_string();
                    }
                    cx.notify();
                })
            })
    };
    let how = {
        let this = this.clone();
        let choices = SHOWN
            .iter()
            .map(|(value, key)| Choice::new(*value, Docs::text(cx, key, &[])));
        Select::new("doc-reference-show", choices)
            .label(Docs::text(cx, "doc.reference.show", &[]))
            .selected(show)
            .on_change(move |value, _, cx| {
                let Some((value, _)) = SHOWN.iter().find(|(shown, _)| *shown == value.as_ref())
                else {
                    return;
                };
                this.update(cx, |docs, cx| {
                    if let Some(Dialog::Reference { show, .. }) = &mut docs.dialog {
                        *show = value;
                    }
                    cx.notify();
                })
            })
    };
    let fields = DescriptionList::new()
        .item(
            Docs::text(cx, "doc.reference.target", &[]),
            div().w_full().max_w_64().child(pick),
        )
        .item(
            Docs::text(cx, "doc.reference.show", &[]),
            div().w_full().max_w_64().child(how),
        );
    let label = Docs::text(cx, "doc.insert.insert", &[]);
    Card::new(
        "doc-insert-reference",
        Docs::text(cx, "doc.reference.title", &[]),
        dismiss,
    )
    .child(fields)
    .action(ely_office::cancel(cx))
    .action(move |_| {
        Button::new("doc-insert-reference-go", label)
            .variant(ButtonVariant::Primary)
            .on_click(move |_, _, cx| this.update(cx, |docs, cx| docs.insert_reference(cx)))
    })
}

#[cfg(test)]
mod tests;
