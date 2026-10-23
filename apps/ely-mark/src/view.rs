//! One Markdown file, its text kept for the shell.

use std::path::PathBuf;

use ely_gpui_component::documents::{
    DocumentEvent, EditRefusal, LARGEST_READ, MarkdownDocument, MarkdownMode,
};
use gpui::{
    AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable as _,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Render, Styled as _,
    Window, div,
};

use crate::engine::{Doc, mark};

/// The text changed, or a read is in.
pub struct Reported;

/// What went wrong, for the shell to say.
pub struct Failed(pub String);

/// An edit asked did not happen, and why.
pub struct Refused(pub EditRefusal);

/// How a document shows (MARK-02).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Visual,
    Source,
    Preview,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Visual, Mode::Source, Mode::Preview];

    pub fn name(self) -> &'static str {
        match self {
            Mode::Visual => "visual",
            Mode::Source => "source",
            Mode::Preview => "preview",
        }
    }

    pub fn parse(name: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|mode| mode.name() == name)
    }

    fn shown(self) -> MarkdownMode {
        match self {
            Mode::Visual => MarkdownMode::Visual,
            Mode::Source => MarkdownMode::Source,
            Mode::Preview => MarkdownMode::Preview,
        }
    }
}

/// One heading of the outline, and its line from zero.
#[derive(Clone, Debug, PartialEq)]
pub struct Heading {
    pub level: usize,
    pub title: String,
    pub line: usize,
}

/// The caret and the first line in view.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Place {
    pub line: u64,
    #[serde(default)]
    pub column: u64,
    #[serde(default)]
    pub top: u64,
}

pub struct MarkView {
    document: Entity<MarkdownDocument>,
    doc: Doc,
    /// Text changes so far; autosave counts them.
    changed: u64,
    /// The mark at the last load or save.
    saved: String,
    pub mode: Mode,
    /// A saved file's folder: pictures resolve and go here.
    pub(crate) folder: Option<PathBuf>,
    /// The mode first took input (PERF-07).
    pub(crate) shown: bool,
    /// Trackpad pixels toward the next size step.
    wheel: f32,
    /// The last right-click menu asked.
    offer: Option<ely_ui::menu::Offer<menu::Asked>>,
    pub focus: FocusHandle,
}

impl EventEmitter<Reported> for MarkView {}
impl EventEmitter<Failed> for MarkView {}
impl EventEmitter<Refused> for MarkView {}

impl MarkView {
    pub fn new(doc: Doc, saved: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let text = doc.lock().expect("the text lock").current.clone();
        let mode = match text.len() > LARGEST_READ {
            true => Mode::Source,
            false => Mode::Visual,
        };
        let view = cx.weak_entity();
        let document = cx.new(|cx| {
            let mut document = MarkdownDocument::new(text, mode.shown(), window, cx);
            document.on_link(move |url, _, cx| {
                view.update(cx, |view, cx| view.follow(url, cx)).ok();
            });
            document
        });
        cx.subscribe_in(&document, window, Self::heard).detach();
        cx.subscribe(&document, |view, _, attached, cx| view.attach(attached, cx))
            .detach();
        // The shell hears the load.
        cx.defer_in(window, |view, _, cx| {
            view.show_once(cx);
            cx.emit(Reported);
        });
        let focus = cx.focus_handle();
        // The view's focus is the document's.
        cx.on_focus(&focus, window, |view, window, cx| {
            let inner = view.document.focus_handle(cx);
            window.focus(&inner, cx);
        })
        .detach();
        MarkView {
            document,
            doc,
            changed: 0,
            saved,
            mode,
            folder: None,
            shown: false,
            wheel: 0.0,
            offer: None,
            focus,
        }
    }

    pub(crate) fn document(&self) -> &Entity<MarkdownDocument> {
        &self.document
    }

    /// Every change is in the text at once.
    pub fn settled(&self) -> bool {
        true
    }

    pub fn changed(&self) -> u64 {
        self.changed
    }

    /// The text's size now, in bytes.
    pub fn bytes(&self) -> usize {
        self.doc.lock().expect("the text lock").current.len()
    }

    /// Too long to read: source alone (MARK-14).
    pub fn large(&self) -> bool {
        self.bytes() > LARGEST_READ
    }

    pub fn modified(&self) -> bool {
        mark(&self.doc.lock().expect("the text lock").current) != self.saved
    }

    pub fn saved(&mut self, mark: &str, cx: &mut Context<Self>) {
        self.saved = mark.to_owned();
        cx.emit(Reported);
    }

    /// The same text in `mode` (MARK-02).
    pub fn switch(&mut self, mode: Mode, window: &mut Window, cx: &mut Context<Self>) {
        if self.large() && mode != Mode::Source {
            log::info!("mark: a large file stays in source");
            let said = ely_ui::i18n::text(cx, "mark.large.modes", &[]).to_string();
            return cx.emit(Failed(said));
        }
        log::info!("mark: switches to {}", mode.name());
        self.mode = mode;
        self.document.update(cx, |document, cx| {
            document.set_mode(mode.shown(), window, cx)
        });
        cx.notify();
    }

    /// The headings as last read.
    pub fn outline(&self, cx: &gpui::App) -> Vec<Heading> {
        let outline = self.document.read(cx).outline(cx);
        outline
            .into_iter()
            .map(|(heading, line)| Heading {
                level: usize::from(heading.depth),
                title: heading.title,
                line,
            })
            .collect()
    }

    /// Line and column `at` in view and focus, caret there.
    pub fn reveal(&mut self, at: (usize, usize), window: &mut Window, cx: &mut Context<Self>) {
        self.document.update(cx, |document, cx| {
            document.reveal(at.0, window, cx);
            document.set_caret(at.0, at.1, cx);
        });
    }

    /// The caret and the view, to keep across launches.
    pub fn place(&self, cx: &gpui::App) -> Place {
        let document = self.document.read(cx);
        let (line, column) = document.caret(cx);
        Place {
            line: line as u64,
            column: column as u64,
            top: document.top_line(cx) as u64,
        }
    }

    /// The disk's text replaces a clean one; undo starts there.
    pub fn refresh(&mut self, source: crate::text::Source, cx: &mut Context<Self>) {
        let text = source.text.clone();
        {
            let mut doc = self.doc.lock().expect("the text lock");
            doc.current = text.clone();
            doc.source = source;
        }
        self.saved = mark(&text);
        self.document
            .update(cx, |document, cx| document.reset(text, cx));
        cx.emit(Reported);
    }

    /// `text` replaces the document's, as one undoable edit.
    pub fn adopt(&mut self, text: String, cx: &mut Context<Self>) {
        self.document
            .update(cx, |document, cx| document.set_text(text, cx));
    }

    /// Shows as `mode` at `place`; recovered text unsaved.
    pub fn reopen(
        &mut self,
        mode: Mode,
        place: Option<Place>,
        text: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(text) = text {
            self.document
                .update(cx, |document, cx| document.reset(text, cx));
        }
        let mode = if self.large() { Mode::Source } else { mode };
        self.switch(mode, window, cx);
        if let Some(place) = place {
            self.document.update(cx, |document, cx| {
                document.set_caret(place.line as usize, place.column as usize, cx);
                document.set_top_line(place.top as usize, window, cx);
            });
        }
    }

    /// The saved file's folder; pictures follow it.
    pub fn moved(&mut self, folder: PathBuf, cx: &mut Context<Self>) {
        if self.folder.as_ref() == Some(&folder) {
            return;
        }
        self.document
            .update(cx, |document, cx| document.set_base(folder.as_path(), cx));
        self.folder = Some(folder);
    }

    fn heard(
        &mut self,
        document: &Entity<MarkdownDocument>,
        event: &DocumentEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let DocumentEvent::EditRefused(why) = event {
            return cx.emit(Refused(*why));
        }
        if let DocumentEvent::Pointed(target) = event {
            let at = window.mouse_position();
            return self.ask_menu(menu::Asked::Text(target.clone()), at, cx);
        }
        if *event == DocumentEvent::Changed {
            let text = document.read(cx).text(cx);
            self.doc.lock().expect("the text lock").current = text;
            self.changed += 1;
            if self.large() && self.mode != Mode::Source {
                log::info!("mark: the text grew past the read limit");
                self.switch(Mode::Source, window, cx);
            }
        }
        self.show_once(cx);
        cx.emit(Reported);
    }

    /// Logs the first moment the mode takes input.
    fn show_once(&mut self, cx: &gpui::App) {
        let ready = self.mode == Mode::Source || self.document.read(cx).read_now(cx);
        if ready && !self.shown {
            self.shown = true;
            log::info!("mark: shown as {:?}", self.mode.name());
        }
    }

    /// A pressed link: web and mail open outside (M-05).
    fn follow(&mut self, url: &str, cx: &mut Context<Self>) {
        if ely_office::outward(url) {
            log::info!("mark: opens a link outside");
            return cx.open_url(url);
        }
        log::info!("mark: a link of another kind was refused");
        let said = ely_ui::i18n::text(cx, "office.link.refused", &[]).to_string();
        cx.emit(Failed(said));
    }
}

impl Render for MarkView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        // Ctrl and the wheel size text, as Ctrl+= does.
        let zoom = ely_ui::wheel::zoom_layer(move |event, window, cx| {
            let steps = view.update(cx, |view, _| ely_ui::wheel::steps(event, &mut view.wheel));
            for _ in 0..steps.unsigned_abs() {
                match steps > 0 {
                    true => window.dispatch_action(Box::new(crate::look::Larger), cx),
                    false => window.dispatch_action(Box::new(crate::look::Smaller), cx),
                }
            }
        });
        div()
            .relative()
            .size_full()
            .track_focus(&self.focus)
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|view, event, _, cx| view.pressed(event, cx)),
            )
            .child(self.document.clone())
            .child(zoom)
            .children(self.context_menu(window, cx))
    }
}

mod menu;
#[cfg(test)]
mod tests;
