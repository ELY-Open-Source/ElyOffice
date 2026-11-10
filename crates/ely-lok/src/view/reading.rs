//! The body for a reader, the caret inside it (UX-12).

use std::io;

use gpui::{Context, Window};

use super::LokView;
use crate::A11y;
use crate::review::Body;

/// The body a reader gets and the read behind it.
#[derive(Default)]
pub(super) struct Reading {
    /// Assistive tech listened at the last paint.
    heard: bool,
    /// A read is out.
    busy: bool,
    /// The document moved on while that read was out.
    stale: bool,
    /// The caret's new paragraph, held until its body lands.
    next: Option<A11y>,
    /// The body as last read; None outside it.
    body: Option<Body>,
}

impl LokView {
    /// At paint: a reader that just arrived gets the body.
    pub(super) fn hear(&mut self, window: &Window, cx: &mut Context<Self>) {
        let heard = window.is_a11y_active();
        let arrived = heard && !self.reading.heard;
        self.reading.heard = heard;
        if arrived {
            self.read_on(cx);
        } else if !heard {
            self.reading.body = None;
        }
    }

    /// Takes the kernel's paragraph; `changed` is a changed page.
    pub(super) fn take_a11y(&mut self, a11y: A11y, changed: bool, cx: &mut Context<Self>) {
        // Only a text document's body flows on.
        if !self.reading.heard || self.doc.is_sheet() || self.doc.is_deck() {
            self.a11y = a11y;
            return;
        }
        let shown = self.reading.next.as_ref().unwrap_or(&self.a11y);
        let moved = a11y.focused != shown.focused;
        // The caret waits for the body that holds its paragraph.
        match moved || self.reading.next.is_some() {
            true => self.reading.next = Some(a11y),
            false => self.a11y = a11y,
        }
        if moved || changed {
            self.read_on(cx);
        }
    }

    /// The body's paragraphs and the caret's index.
    pub(super) fn body(&self) -> Option<(&[String], usize)> {
        let body = self.reading.body.as_ref()?;
        Some((&body.paragraphs, body.caret))
    }

    /// Reads the body, one read at a time.
    fn read_on(&mut self, cx: &mut Context<Self>) {
        if self.reading.busy {
            self.reading.stale = true;
            return;
        }
        self.reading.busy = true;
        let read = |worker: &mut crate::Worker, doc: &crate::Doc| crate::review::reading(&worker.values(doc, crate::review::READING)?);
        super::read_off(&self.worker, &self.doc, read, Self::landed, cx);
    }

    /// A read came back; a stale one is read again.
    fn landed(&mut self, read: io::Result<Option<Body>>, cx: &mut Context<Self>) {
        self.reading.busy = false;
        if std::mem::take(&mut self.reading.stale) {
            return self.read_on(cx);
        }
        self.reading.body = read.unwrap_or_else(|error| {
            log::error!("lok: the body was not read for the reader: {error}");
            None
        });
        if let Some(next) = self.reading.next.take() {
            self.a11y = next;
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::super::tests::live_view_of;
    use super::LokView;
    use crate::Events;
    use crate::client::tests::{HANG, install, poll_until, reading::THREE};

    /// Presses `key`; the batch once the caret's paragraph is `text`.
    fn moved(view: &LokView, key: i32, text: &str) -> Events {
        let mut worker = view.worker.lock().expect("the worker lock");
        worker.post_key(&view.doc, 0, key).expect("the key posts");
        poll_until(&mut worker, &view.doc, &Events::default(), HANG, |seen| seen.a11y.text.starts_with(text))
    }

    /// Runs the app until `done` or the hang limit.
    fn until(cx: &TestAppContext, mut done: impl FnMut() -> bool) -> bool {
        let deadline = std::time::Instant::now() + HANG;
        while std::time::Instant::now() < deadline && !done() {
            for _ in 0..20 {
                cx.executor().tick();
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        done()
    }

    #[test]
    fn a_listening_reader_gets_the_body_and_the_caret_s_place() {
        let Some(install) = install() else {
            return eprintln!("SKIP: no pinned LOK build");
        };
        let cx = TestAppContext::single();
        cx.executor().allow_parking();
        let (view, _) = cx.update(|app| live_view_of(app, &install, "three.fodt", THREE));
        let shown = || cx.update(|app| view.read(app).body().map(|(body, caret)| (body.to_vec(), caret, view.read(app).a11y.text.clone())));
        // No reader listens: a move asks the kernel nothing.
        cx.update(|app| view.update(app, |view, cx| view.apply_batch(moved(view, 1024, "two"), None, cx)));
        assert!(!cx.update(|app| view.read(app).reading.busy), "nothing is read for no one");
        assert_eq!(cx.update(|app| view.read(app).a11y.text.clone()), "two 二1", "the paragraph alone follows at once");
        cx.update(|app| {
            view.update(app, |view, cx| {
                view.reading.heard = true;
                view.apply_batch(moved(view, 1025, "one"), None, cx);
                // The caret stays put until the body names its place.
                assert_eq!(view.a11y.text, "two 二1");
            })
        });
        let body = ["one".to_string(), "two 二1".to_string(), "three".to_string()];
        assert!(until(&cx, || shown() == Some((body.to_vec(), 0, "one".into()))), "at the first paragraph: {:?}", shown());
        cx.update(|app| view.update(app, |view, cx| view.apply_batch(moved(view, 1024, "two"), None, cx)));
        assert!(until(&cx, || shown() == Some((body.to_vec(), 1, "two 二1".into()))), "at the second: {:?}", shown());
    }
}
