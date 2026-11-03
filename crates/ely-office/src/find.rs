//! Find and replace over the active document (DOC-12, TABLE-08).

use std::collections::VecDeque;

use ely_gpui_component::{
    editor::{FindOptions, FindWidget},
    forms::{InputEvent, TextInput},
    primitives::{Severity, raise},
};
use ely_lok::{LokView, Rect};
use gpui::{
    AnyElement, AppContext as _, Context, Entity, Focusable as _, InteractiveElement as _,
    IntoElement, ParentElement as _, Styled as _, Subscription, Window, actions, div,
};

use crate::{Commands as _, Host, Tabs as _, text};

actions!(office, [FindReplace]);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Asked {
    All,
    Step,
}

/// The open find box and what the kernel last found.
pub struct Finder {
    /// The tab it searches.
    tab: u64,
    pub query: Entity<TextInput>,
    pub replace: Option<Entity<TextInput>>,
    pub options: FindOptions,
    /// Every match from the last count, in order.
    pub all: Vec<Rect>,
    pub current: Option<usize>,
    /// The view's search count already taken.
    seen: u64,
    /// Requests still to answer; a newer query voids older ones.
    pub pending: VecDeque<(Asked, bool)>,
    /// Steps waiting for a count to land.
    pub queued: VecDeque<bool>,
    /// The first step marked the place for Back.
    left: bool,
    /// The view's edit count and undo history last seen.
    edited: u64,
    history: Option<String>,
    /// The edit count of the newest history check applied.
    checked: u64,
    /// The document changed since the count.
    pub stale: bool,
    _query: Subscription,
}

pub trait Finding: Host<Engine = crate::Lok> {
    fn on_find(&mut self, _: &FindReplace, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            let text = text(cx, "command.blocked.no_document", &[]);
            return self.toast(text, Severity::Warning, cx);
        };
        let open = self.core().finder.as_ref();
        if let Some(finder) = open.filter(|finder| finder.tab == tab.id) {
            return window.focus(&finder.query.focus_handle(cx), cx);
        }
        let (id, seen) = (tab.id, tab.view.read(cx).search().0);
        let edited = tab.view.read(cx).edited();
        let query = cx.new(|cx| TextInput::new(window, cx));
        let subscription = cx.subscribe(&query, |host, _, event, cx| match event {
            InputEvent::Changed => host.count_matches(cx),
            InputEvent::Submit => host.step(true, cx),
            _ => {}
        });
        window.focus(&query.focus_handle(cx), cx);
        self.core_mut().finder = Some(Finder {
            tab: id,
            query,
            replace: None,
            options: FindOptions::default(),
            all: Vec::new(),
            current: None,
            seen,
            pending: VecDeque::new(),
            queued: VecDeque::new(),
            left: false,
            edited,
            history: None,
            checked: edited,
            stale: false,
            _query: subscription,
        });
        self.check_history(id, edited, false, cx);
        cx.notify();
    }

    /// Checks the undo history in the background; moves mark stale.
    fn check_history(&mut self, id: u64, edited: u64, counted: bool, cx: &mut Context<Self>) {
        self.read_in(
            id,
            |worker, doc| worker.values(doc, ".uno:Undo"),
            move |host, read, _, cx| {
                let history = match read {
                    Ok(history) => history,
                    Err(error) => return host.find_failed(&error, cx),
                };
                let slug = Self::KIND.app.slug();
                let open = host.core_mut().finder.as_mut();
                let Some(finder) = open.filter(|finder| finder.tab == id) else {
                    return;
                };
                // A check for older edits never overrides a newer one.
                if edited < finder.checked {
                    return;
                }
                let moved = finder.history.as_ref().is_some_and(|old| *old != history);
                if counted {
                    finder.stale = false;
                } else if moved && edited > finder.checked {
                    log::info!("{slug}: the text changed under the finder");
                    finder.stale = true;
                }
                (finder.checked, finder.history) = (edited, Some(history));
                cx.notify();
            },
            cx,
        );
    }

    /// The query and its options as `find.run` values.
    fn search_values(&self, cx: &Context<Self>, command: &str, back: bool) -> Option<Vec<String>> {
        let finder = self.here()?;
        let query = finder.query.read(cx).text().to_string();
        if query.is_empty() {
            return None;
        }
        let options = finder.options;
        let flag = |on: bool, yes: &str, no: &str| if on { yes } else { no }.to_string();
        Some(vec![
            query,
            back.to_string(),
            command.to_string(),
            flag(options.case, "0", "256"),
            flag(options.word, "16", "0"),
            flag(options.regex, "2", "1"),
        ])
    }

    fn run_search(
        &mut self,
        id: &str,
        values: Vec<String>,
        asked: Option<Asked>,
        cx: &mut Context<Self>,
    ) {
        if let Some((finder, asked)) = self.core_mut().finder.as_mut().zip(asked) {
            finder.pending.push_back((asked, true));
        }
        let values: Vec<&str> = values.iter().map(String::as_str).collect();
        self.run_active(id, &values, cx);
    }

    fn find_failed(&self, error: &std::io::Error, cx: &mut Context<Self>) {
        let text = text(cx, "office.error.engine", &[("error", &error.to_string())]);
        self.toast(text, Severity::Danger, cx);
    }

    /// Counts every match; the kernel highlights them.
    fn count_matches(&mut self, cx: &mut Context<Self>) {
        if let Some(finder) = self.core_mut().finder.as_mut() {
            finder
                .pending
                .iter_mut()
                .for_each(|(_, live)| *live = false);
            finder.queued.clear();
        }
        let Some(values) = self.search_values(cx, "1", false) else {
            if let Some(finder) = self.core_mut().finder.as_mut() {
                finder.all.clear();
                finder.current = None;
            }
            return cx.notify();
        };
        self.run_search("find.run", values, Some(Asked::All), cx);
    }

    fn step(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(finder) = self.here() else {
            return;
        };
        if finder.stale {
            return self.count_matches(cx);
        }
        if counting(finder) {
            let finder = self.core_mut().finder.as_mut().expect("the finder is open");
            finder.queued.push_back(forward);
            return;
        }
        if let Some(values) = self.search_values(cx, "0", !forward) {
            if !std::mem::replace(
                &mut self.core_mut().finder.as_mut().expect("open").left,
                true,
            ) {
                self.search_leaves(cx);
            }
            self.run_search("find.run", values, Some(Asked::Step), cx);
        }
    }

    /// Replaces the chosen match, or every one; then counts again.
    fn replace(&mut self, every: bool, cx: &mut Context<Self>) {
        let Some(finder) = self.here() else {
            return;
        };
        let Some(field) = &finder.replace else {
            return;
        };
        if finder.stale {
            return self.count_matches(cx);
        }
        if !every && finder.current.is_none() {
            return self.step(true, cx);
        }
        let with = field.read(cx).text().to_string();
        let Some(mut values) = self.search_values(cx, if every { "3" } else { "2" }, false) else {
            return;
        };
        values.push(with);
        // A replace reports only when nothing is left.
        self.run_search("find.replace", values, None, cx);
        self.count_matches(cx);
    }

    /// Takes a search outcome or an edit.
    fn found(&mut self, id: u64, view: &Entity<LokView>, cx: &mut Context<Self>) {
        let Some(finder) = self
            .core()
            .finder
            .as_ref()
            .filter(|finder| finder.tab == id)
        else {
            return;
        };
        let edited = view.read(cx).edited();
        let (count, found) = {
            let (count, found) = view.read(cx).search();
            (count, found.to_vec())
        };
        let outcomes = usize::try_from(count - finder.seen).expect("a count of outcomes");
        let asked = (0..outcomes)
            .fold(None, |last, _| {
                let finder = self.core_mut().finder.as_mut().expect("the finder is open");
                let next = finder.pending.pop_front();
                next.map(|(asked, live)| live.then_some(asked)).or(last)
            })
            .flatten();
        let finder = self.core().finder.as_ref().expect("the finder is open");
        let counted = asked == Some(Asked::All);
        if edited != finder.edited || counted {
            let finder = self.core_mut().finder.as_mut().expect("the finder is open");
            finder.edited = edited;
            // A fresh count is current now; older checks drop.
            if counted {
                (finder.stale, finder.checked) = (false, edited);
            }
            self.check_history(id, edited, counted, cx);
        }
        let finder = self.core_mut().finder.as_mut().expect("the finder is open");
        if count != finder.seen {
            finder.seen = count;
            match (asked, found.first()) {
                (Some(Asked::All), _) | (Some(Asked::Step), None) => {
                    finder.all = found.clone();
                    finder.current = None;
                }
                (Some(Asked::Step), Some(first)) => {
                    finder.current = finder.all.iter().position(|rect| rect == first);
                    let first = *first;
                    view.update(cx, |view, cx| view.show(first, cx));
                }
                (None, _) => log::info!(
                    "{}: a search outcome nobody asked for",
                    Self::KIND.app.slug()
                ),
            }
            log::info!(
                "{}: {} matches, at {:?}",
                Self::KIND.app.slug(),
                finder.all.len(),
                finder.current
            );
        }
        let finder = self.core_mut().finder.as_mut().expect("the finder is open");
        let chosen = finder.current.map(|at| finder.all[at]);
        if chosen.is_some() && view.read(cx).selection().first() != chosen.as_ref() {
            finder.current = None;
        }
        if !counting(finder) {
            for forward in std::mem::take(&mut finder.queued) {
                self.step(forward, cx);
            }
        }
    }

    fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.core_mut().finder = None;
        if let Some(tab) = self.active_tab() {
            window.focus(&tab.view.read(cx).focus_handle().clone(), cx);
        }
        cx.notify();
    }

    /// The finder, when it searches the active tab.
    fn here(&self) -> Option<&Finder> {
        self.core()
            .finder
            .as_ref()
            .filter(|finder| Some(finder.tab) == self.core().active())
    }

    /// The find box over the page, while open.
    fn find_box(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let finder = self.here()?;
        let this = cx.entity();
        let mut widget = FindWidget::new(
            "office-find",
            &finder.query,
            finder.current,
            finder.all.len(),
        )
        .options(finder.options)
        .on_options({
            let this = this.clone();
            move |options, _, cx| {
                this.update(cx, |host, cx| {
                    if let Some(finder) = host.core_mut().finder.as_mut() {
                        finder.options = options;
                    }
                    host.count_matches(cx);
                })
            }
        })
        .on_step({
            let this = this.clone();
            move |forward, _, cx| this.update(cx, |host, cx| host.step(forward, cx))
        })
        .on_replace({
            let this = this.clone();
            move |every, _, cx| this.update(cx, |host, cx| host.replace(every, cx))
        })
        .on_toggle_replace({
            let this = this.clone();
            move |show, window, cx| {
                this.update(cx, |host, cx| {
                    if let Some(finder) = host.core_mut().finder.as_mut() {
                        finder.replace = show.then(|| cx.new(|cx| TextInput::new(window, cx)));
                    }
                    cx.notify();
                })
            }
        })
        .on_close(move |window, cx| this.update(cx, |host, cx| host.close_find(window, cx)));
        if let Some(field) = &finder.replace {
            widget = widget.replace(field);
        }
        if finder.stale {
            widget = widget.error(text(cx, "office.find.stale", &[]));
        }
        let place = div()
            .absolute()
            .top_2()
            .right_4()
            .block_mouse_except_scroll()
            .child(widget);
        Some(raise("office-find-layer", place).into_any_element())
    }
}

impl<H: Host<Engine = crate::Lok>> Finding for H {}

/// Whether a live count is still on its way.
fn counting(finder: &Finder) -> bool {
    finder
        .pending
        .iter()
        .any(|(asked, live)| *live && *asked == Asked::All)
}
