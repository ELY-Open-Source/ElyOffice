//! New decks: the kernel's blank, or a bundled template (SHOW-02).

use std::{path::PathBuf, rc::Rc};

use ely_gpui_component::{
    buttons::Button,
    documents::{Template as Pick, TemplatePicker},
    overlays::Dialog as Card,
};
use ely_office::{in_window, prelude::*};
use gpui::{
    AnyElement, App, AppContext as _, Context, IntoElement, ParentElement as _, TaskExt as _,
    Window,
};

use crate::deck::Deck;
use crate::dialog::Dialog;

/// The New dialog's templates: blank first.
pub struct Choosing {
    templates: Vec<PathBuf>,
}

fn stem(path: &std::path::Path) -> String {
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    name.replace('_', " ")
}

impl Deck {
    pub fn ask_new(&mut self, cx: &mut Context<Self>) {
        match ely_lok::templates("presnt") {
            Ok(templates) => {
                log::info!("show: {} bundled templates", templates.len());
                // Refused while another dialog is open; it says so.
                let _ = self.open_dialog(Dialog::New(Choosing { templates }), cx);
            }
            Err(error) => self.engine_failed(&error.to_string(), cx),
        }
    }

    /// A new deck as the kernel makes one.
    pub fn new_blank(&mut self, cx: &mut Context<Self>) {
        if let Some(blank) = self.blank.clone() {
            return self.new_from_file(&blank, cx);
        }
        let (worker, path) = (self.core.worker.clone(), self.core.work().join("blank.fodp"));
        let writing = path.clone();
        cx.spawn(async move |this, cx| {
            let written = cx
                .background_spawn(async move {
                    let mut worker = worker.lock().expect("the worker lock");
                    let doc = worker.load("private:factory/simpress")?;
                    let saved = doc.save_as(&mut worker, &ely_lok::file_url(&writing), "fodp");
                    worker.close_doc(doc)?;
                    saved
                })
                .await;
            in_window(&this, cx, |deck, _, cx| match written {
                Ok(()) => {
                    deck.blank = Some(path.clone());
                    deck.new_from_file(&path, cx)
                }
                Err(error) => deck.engine_failed(&error.to_string(), cx),
            })
        })
        .detach_and_log_err(cx);
    }

    pub fn new_dialog(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let Some(Dialog::New(Choosing { templates })) = &self.dialog else {
            return None;
        };
        let this = cx.entity();
        let blank = Pick {
            name: Deck::text(cx, "show.new.blank", &[]),
            icon: "▭".into(),
            category: Deck::text(cx, "show.new.category", &[]),
            description: Deck::text(cx, "show.new.blank_about", &[]),
            markdown: "".into(),
        };
        let picks: Vec<Pick> = std::iter::once(blank)
            .chain(templates.iter().map(|path| Pick {
                name: stem(path).into(),
                icon: "▭".into(),
                category: Deck::text(cx, "show.new.category", &[]),
                description: "".into(),
                markdown: "".into(),
            }))
            .collect();
        let paths = templates.clone();
        let used = this.clone();
        let picker =
            TemplatePicker::new("show-templates", Rc::new(picks)).on_use(move |ix, _, cx| {
                let chosen = ix.checked_sub(1).map(|at| paths[at].clone());
                used.update(cx, |deck, cx| {
                    deck.dialog = None;
                    match chosen {
                        Some(template) => deck.new_from_file(&template, cx),
                        None => deck.new_blank(cx),
                    }
                })
            });
        let dismiss = {
            let this = this.clone();
            move |_: &mut Window, cx: &mut App| {
                this.update(cx, |deck, cx| {
                    deck.dialog = None;
                    cx.notify();
                })
            }
        };
        let label = Deck::text(cx, "show.new.from_file", &[]);
        let card = Card::new("show-new", Deck::text(cx, "show.new.title", &[]), dismiss)
            .child(picker)
            .action(move |close| {
                Button::new("show-new-file", label).on_click(move |_, window, cx| {
                    close(window, cx);
                    this.update(cx, |deck, cx| {
                        deck.pick(false, cx, |deck, path, cx| deck.new_from_file(&path, cx))
                    });
                })
            });
        Some(card.into_any_element())
    }
}
