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
        self.listened(window.is_a11y_active(), cx);
    }

    /// Whether assistive tech listens now.
    fn listened(&mut self, heard: bool, cx: &mut Context<Self>) {
        let arrived = heard && !self.reading.heard;
        self.reading.heard = heard;
        if arrived {
            return self.read_on(cx);
        }
        if !heard {
            self.reading.body = None;
            // No body is coming: the held paragraph shows now.
            if let Some(next) = self.reading.next.take() {
                self.a11y = next;
            }
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
        let stale = std::mem::take(&mut self.reading.stale);
        // The reader left: its body is no one's.
        if !self.reading.heard {
            return;
        }
        if stale {
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

    use super::super::tests::{live_view_of, until};
    use super::LokView;
    use crate::Events;
    use crate::client::tests::{HANG, install, poll_until, reading::THREE};

    /// Presses `key`; the batch once the caret's paragraph is `text`.
    fn moved(view: &LokView, key: i32, text: &str) -> Events {
        let mut worker = view.worker.lock().expect("the worker lock");
        worker.post_key(&view.doc, 0, key).expect("the key posts");
        poll_until(&mut worker, &view.doc, &Events::default(), HANG, |seen| seen.a11y.text.starts_with(text))
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

    #[test]
    fn a_reader_that_leaves_takes_no_late_body() {
        let Some(install) = install() else {
            return eprintln!("SKIP: no pinned LOK build");
        };
        let cx = TestAppContext::single();
        cx.executor().allow_parking();
        let (view, _) = cx.update(|app| live_view_of(app, &install, "three.fodt", THREE));
        let text = || cx.update(|app| view.read(app).a11y.text.clone());
        cx.update(|app| {
            view.update(app, |view, cx| {
                view.apply_batch(moved(view, 1024, "two"), None, cx);
                view.listened(true, cx);
                view.apply_batch(moved(view, 1025, "one"), None, cx);
                assert_eq!(view.a11y.text, "two 二1", "the first paragraph waits for its body");
                view.listened(false, cx);
                assert_eq!(view.a11y.text, "one", "no body is coming: it shows now");
                view.apply_batch(moved(view, 1024, "two"), None, cx);
            })
        });
        assert_eq!(text(), "two 二1", "no one listens: the paragraph follows at once");
        assert!(until(&cx, || !cx.update(|app| view.read(app).reading.busy)), "the read never landed");
        assert_eq!(text(), "two 二1", "the late body brought the old paragraph back");
        assert!(cx.update(|app| view.read(app).body().is_none()), "a body was kept for no one");
        // The reader returns: the body is read anew.
        cx.update(|app| view.update(app, |view, cx| view.listened(true, cx)));
        let body = || cx.update(|app| view.read(app).body().map(|(_, caret)| caret));
        assert!(until(&cx, || body() == Some(1)), "the returning reader's body: {:?}", body());
    }
}
