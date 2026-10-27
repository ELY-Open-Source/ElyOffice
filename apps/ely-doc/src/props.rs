//! The properties panel (DOC-02 to 08).

mod builtin;
#[cfg(test)]
mod chinese;
mod groups;
#[cfg(test)]
mod layout_tests;
mod object;
mod page;
mod pairs;
mod paragraph;
mod sheet;
mod styles;
mod table;
#[cfg(test)]
mod tests;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use ely_gpui_component::{
    data_display::PropertyGrid, forms::TextInput, layout::ScrollArea, overlays::PromptDialog,
};
use ely_lok::{Doc, LokError, LokView, Reported, Worker};
use ely_office::panel::Panel as _;
use gpui::{
    App, Context, Entity, EventEmitter, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, Window, div,
};

use crate::docs::Docs;

/// The active document the panel edits.
struct Shown {
    view: Entity<LokView>,
    doc: Doc,
    paragraph: Vec<SharedString>,
    character: Vec<SharedString>,
    pages: Vec<SharedString>,
    _observe: Subscription,
    _errors: Subscription,
}

pub struct Props {
    worker: Arc<Mutex<Worker>>,
    shown: Option<Shown>,
    fonts: Vec<SharedString>,
    /// Re-read the styles once input lands.
    reload: bool,
    /// The family a new style goes to: "1" or "2".
    naming: Option<&'static str>,
    tab_at: f64,
    /// Two-value edits awaiting the kernel's echo.
    pending: HashMap<&'static str, pairs::Sent>,
    /// Page edits awaiting the kernel's echo.
    sheet_sent: Option<sheet::Sent>,
}

impl EventEmitter<LokError> for Props {}

impl Props {
    pub fn new(worker: Arc<Mutex<Worker>>) -> Self {
        Props {
            worker,
            shown: None,
            fonts: Vec::new(),
            reload: false,
            naming: None,
            tab_at: 1.0,
            pending: HashMap::new(),
            sheet_sent: None,
        }
    }

    /// Follows the active tab; reads the kernel lists on change.
    pub fn show(&mut self, tab: Option<(&Entity<LokView>, &Doc)>, cx: &mut Context<Self>) {
        let id = tab.map(|(view, _)| view.entity_id());
        if self.shown.as_ref().map(|shown| shown.view.entity_id()) == id {
            return;
        }
        self.shown = None;
        self.naming = None;
        self.pending.clear();
        self.sheet_sent = None;
        let Some((view, doc)) = tab else {
            return;
        };
        let observe = cx.subscribe(view, |props, view, _: &Reported, cx| {
            if props.reload && view.read(cx).settled() {
                props.reload = false;
                props.read_styles(cx);
            }
            props.track_pairs(cx);
            props.track_sheet(cx);
            cx.notify();
        });
        let errors = cx.subscribe(view, |props, _, _: &LokError, _| {
            props.pending.clear();
            props.sheet_sent = None;
        });
        self.shown = Some(Shown {
            view: view.clone(),
            doc: doc.clone(),
            paragraph: Vec::new(),
            character: Vec::new(),
            pages: Vec::new(),
            _observe: observe,
            _errors: errors,
        });
        if self.fonts.is_empty() {
            self.read_fonts(doc, cx);
        }
        self.read_styles(cx);
    }

    /// The installed fonts, read off the window thread.
    fn read_fonts(&mut self, doc: &Doc, cx: &mut Context<Self>) {
        ely_lok::read_off(
            &self.worker,
            doc,
            |worker, doc| ely_lok::commands::fonts(&worker.values(doc, ".uno:CharFontName")?),
            |props: &mut Props, read, cx| match read {
                Ok(fonts) => {
                    props.fonts = fonts.into_iter().map(Into::into).collect();
                    cx.notify();
                }
                Err(error) => cx.emit(LokError(format!("the font list is unreadable: {error}"))),
            },
            cx,
        );
    }

    /// The shown document's style lists, read off the window thread.
    fn read_styles(&mut self, cx: &mut Context<Self>) {
        let Some(shown) = self.shown.as_ref() else {
            return;
        };
        let view = shown.view.entity_id();
        ely_lok::read_off(
            &self.worker,
            &shown.doc,
            |worker, doc| {
                let json = worker.values(doc, ".uno:StyleApply")?;
                let list = |family| -> std::io::Result<Vec<SharedString>> {
                    let names = ely_lok::commands::styles(&json, family)?;
                    Ok(names.into_iter().map(Into::into).collect())
                };
                Ok((list("2")?, list("1")?, list("8")?))
            },
            move |props: &mut Props, read, cx| {
                // Another tab shown since: its own read follows.
                let Some(shown) = props.shown.as_mut().filter(|shown| shown.view.entity_id() == view) else {
                    return;
                };
                match read {
                    Ok((paragraph, character, pages)) => {
                        (shown.paragraph, shown.character, shown.pages) = (paragraph, character, pages);
                        cx.notify();
                    }
                    Err(error) => cx.emit(LokError(format!("the style list is unreadable: {error}"))),
                }
            },
            cx,
        );
    }

    /// Re-reads the style lists once queued input lands.
    pub fn refresh_lists(&mut self) {
        self.reload = true;
    }

    /// Sends one table command to the shown document.
    fn run(&mut self, id: &str, values: &[&str], cx: &mut Context<Self>) {
        let Some(shown) = &self.shown else {
            return;
        };
        log::info!("doc: panel runs {id}");
        // Numbers here, like tab stops, are as the view shows.
        let sent = shown.view.update(cx, |view, _| view.run_at(&view.read_at(), id, values));
        if let Err(error) = sent {
            cx.emit(LokError(format!("{id}: {error}")));
        }
    }

    /// A style from the selection; lists reload after.
    fn create_style(&mut self, name: &str, family: &'static str, cx: &mut Context<Self>) {
        self.naming = None;
        self.reload = true;
        self.run("style.new", &[name, family], cx);
        cx.notify();
    }

    /// The new-style prompt, while open.
    fn prompt(&self, window: &mut Window, cx: &mut Context<Self>) -> Option<PromptDialog> {
        let (family, shown) = (self.naming?, self.shown.as_ref()?);
        let field = window.use_keyed_state("doc-style-name", cx, |window, cx| {
            TextInput::new(window, cx)
        });
        let taken = match family {
            "1" => shown.character.clone(),
            _ => shown.paragraph.clone(),
        };
        let title = match family {
            "1" => "doc.style.new_character",
            _ => "doc.style.new_paragraph",
        };
        let this = cx.entity();
        let close = {
            let this = this.clone();
            move |_: &mut Window, cx: &mut App| {
                this.update(cx, |props, cx| {
                    props.naming = None;
                    cx.notify();
                })
            }
        };
        let (empty, exists) = (
            Docs::text(cx, "doc.style.empty", &[]),
            Docs::text(cx, "doc.style.taken", &[]),
        );
        Some(
            PromptDialog::new("doc-style-new", Docs::text(cx, title, &[]), &field, close)
                .label(Docs::text(cx, "doc.style.name", &[]))
                .submit(Docs::text(cx, "doc.style.create", &[]))
                .check(move |text| {
                    let name = text.trim();
                    if name.is_empty() {
                        return Err(empty.clone());
                    }
                    if taken.iter().any(|style| style == name) {
                        return Err(exists.clone());
                    }
                    Ok(())
                })
                .on_submit(move |text, _, cx| {
                    let name = text.trim().to_string();
                    this.update(cx, |props, cx| props.create_style(&name, family, cx))
                }),
        )
    }
}

impl Render for Props {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let graphic = self
            .shown
            .as_ref()
            .is_some_and(|shown| shown.view.read(cx).graphic().is_some());
        let grid = PropertyGrid::new("doc-props");
        let grid = if graphic {
            grid.group(self.object_group(cx))
        } else {
            // In a table, its group leads, as in Word.
            let grid = match self.state(".uno:DeleteTable", cx).as_deref() {
                Some("enabled") => grid.group(self.table_group(cx)),
                _ => grid,
            };
            grid.group(self.text_group(window, cx))
                .group(self.paragraph_group(cx))
                .group(self.style_group(cx))
                .group(self.page_group(cx))
        };
        div()
            .size_full()
            .child(ScrollArea::new("doc-props-scroll").vertical().size_full().child(grid))
            .children(self.prompt(window, cx))
    }
}
