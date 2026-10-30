//! Leaving with messages open: each goes into Drafts (C12.12, C12.13).

use ely_gpui_component::primitives::Severity;
use ely_mail_engine::{Shelved, Store};
use gpui::{TestAppContext, VisualTestContext, WindowHandle};

use super::{
    Keys, account, settle, window,
    writing::{NOTE, composing, draft, opened, seeded},
};
use crate::{
    compose::{Compose, file_name},
    mail::Mail,
    writing::{How, guard_leaving, prepare},
};

/// A message with a PDF here and one not downloaded.
fn with_files() -> String {
    format!(
        "{NOTE}Content-Type: multipart/mixed; boundary=b\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nSee\r\n\
         --b\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=a.pdf\r\n\r\n%PDF\r\n--b--\r\n"
    )
}

fn closes(compose: WindowHandle<Compose>, cx: &mut VisualTestContext) -> bool {
    cx.update(|_, app| {
        compose
            .update(app, |compose, window, cx| compose.may_close(window, cx))
            .unwrap()
    })
}

/// The drafts the store holds now.
pub(super) fn kept(mail: &gpui::Entity<Mail>, cx: &mut VisualTestContext) -> Vec<Shelved> {
    cx.update(|_, app| {
        let store = mail.read(app).store.clone().unwrap();
        let store = store.lock().unwrap();
        store.drafts().unwrap()
    })
}

fn drafts_in(mail: &gpui::Entity<Mail>, app: &gpui::App) -> usize {
    let store = mail.read(app).store.clone().unwrap();
    store.lock().unwrap().drafts().unwrap().len()
}

fn gone(compose: WindowHandle<Compose>, cx: &mut VisualTestContext) {
    settle(cx, |app| compose.read(app).is_err());
}

fn with<R>(
    compose: WindowHandle<Compose>,
    cx: &mut VisualTestContext,
    act: impl FnOnce(&mut Compose, &mut gpui::Window, &mut gpui::Context<Compose>) -> R,
) -> R {
    cx.update(|_, app| compose.update(app, act).unwrap())
}

fn write(compose: WindowHandle<Compose>, words: &str, cx: &mut VisualTestContext) {
    with(compose, cx, |compose, _, cx| {
        compose
            .document
            .update(cx, |document, cx| document.set_text(words, cx))
    });
}

#[test]
fn quit_keeps_each_message_then_goes() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|window, app| {
        ely_ui::commands::bind_quit(app);
        guard_leaving(mail.downgrade(), window, app);
    });
    for count in 1..=2 {
        cx.update(|_, app| mail.update(app, |mail, cx| mail.write_new(cx)));
        settle(cx, |app| {
            let open = app.windows().into_iter();
            open.filter(|each| each.downcast::<Compose>().is_some())
                .count()
                == count
        });
    }
    let [first, second] = composing(cx).try_into().unwrap();
    write(first, "one", cx);
    write(second, "two", cx);
    cx.update(|window, app| window.dispatch_action(Box::new(ely_ui::commands::Quit), app));
    gone(first, cx);
    gone(second, cx);
    let mut words: Vec<_> = kept(&mail, cx)
        .into_iter()
        .map(|draft| draft.unsent.words)
        .collect();
    words.sort();
    assert_eq!(words, ["one", "two"], "each went into Drafts");
}

#[test]
fn the_main_window_stays_while_a_message_is_sending() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|window, app| guard_leaving(mail.downgrade(), window, app));
    cx.update(|_, app| mail.update(app, |mail, cx| mail.write_new(cx)));
    let compose = opened(cx);
    with(compose, cx, |compose, _, _| compose.sending = true);
    assert!(!cx.simulate_close(), "the main window stays");
    let problem = cx.update(|_, app| compose.read(app).unwrap().problem.clone());
    assert_eq!(
        problem.map(|(severity, _)| severity),
        Some(Severity::Warning)
    );
}

#[test]
fn closing_waits_for_files_then_keeps_them() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let file = dir.path().join("note.txt");
    std::fs::write(&file, "hello").unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.write_new(cx)));
    let compose = opened(cx);
    with(compose, cx, |compose, window, cx| {
        compose.attach(vec![file], cx);
        compose.send(&draft(&["ann@example.com"], "Hi"), window, cx);
        assert!(!compose.sending, "the file is not read yet");
        assert_eq!(compose.reading, 1);
    });
    assert!(!closes(compose, cx), "a file on its way holds the close");
    gone(compose, cx);
    let [draft] = kept(&mail, cx).try_into().unwrap();
    let store = cx.update(|_, app| mail.read(app).store.clone().unwrap());
    let files = store.lock().unwrap().draft_files(draft.id).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].bytes, b"hello");
}

#[test]
fn an_untouched_forward_leaves_no_draft() {
    let dir = tempfile::tempdir().unwrap();
    let id = seeded(dir.path(), with_files().as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    for touched in [false, true] {
        cx.update(|_, app| mail.update(app, |mail, cx| mail.answer(id, How::Forward, cx)));
        let compose = opened(cx);
        settle(cx, |_| true);
        if touched {
            with(compose, cx, |compose, _, _| compose.files.clear());
        }
        let shut = closes(compose, cx);
        assert_eq!(shut, !touched, "a file taken off is kept first");
        if !shut {
            gone(compose, cx);
        } else {
            with(compose, cx, |_, window, _| window.remove_window());
            gone(compose, cx);
        }
        assert_eq!(kept(&mail, cx).len(), usize::from(touched));
    }
    let [draft] = kept(&mail, cx).try_into().unwrap();
    assert_eq!(draft.unsent.subject, "Fwd: plan");
}

#[test]
fn a_file_without_a_name_is_forwarded() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = format!(
        "{NOTE}Content-Type: multipart/mixed; boundary=b\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nSee\r\n\
         --b\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment\r\n\r\n%PDF\r\n--b--\r\n"
    );
    let id = seeded(dir.path(), bytes.as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.answer(id, How::Forward, cx)));
    let compose = opened(cx);
    let out = cx.update(|_, app| {
        let compose = compose.read(app).unwrap();
        assert_eq!(compose.files.len(), 1);
        assert_eq!(
            file_name(&compose.files[0].part, app).as_ref(),
            "Unnamed file"
        );
        compose.outgoing(&draft(&["eve@example.com"], "Fwd: plan"), app)
    });
    let bytes = prepare(&out.unwrap()).unwrap().bytes;
    let sent = ely_mail_engine::read(&bytes).unwrap();
    assert_eq!(sent.attachments.len(), 1);
    assert_eq!(sent.attachments[0].name, None);
}

/// Presses the drawn Close of `on`, Windows and Linux style.
#[cfg(not(target_os = "macos"))]
fn press_close(on: gpui::AnyWindowHandle, cx: &mut VisualTestContext) {
    let mut there = VisualTestContext::from_window(on, cx);
    there.run_until_parked();
    let controls = there
        .debug_bounds("titlebar-controls")
        .expect("drawn controls");
    let close = gpui::point(controls.right() - gpui::px(8.0), controls.center().y);
    there.simulate_click(close, gpui::Modifiers::none());
    there.run_until_parked();
}

// macOS draws no Close: AppKit's traffic lights.
#[cfg(not(target_os = "macos"))]
#[test]
fn the_drawn_close_keeps_as_the_system_close_does() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    let main = cx.update(|window, _| window.window_handle());
    cx.update(|window, app| guard_leaving(mail.downgrade(), window, app));
    cx.update(|_, app| mail.update(app, |mail, cx| mail.write_new(cx)));
    let compose = opened(cx);
    write(compose, "words", cx);
    with(compose, cx, |compose, _, _| compose.sending = true);
    press_close(compose.into(), cx);
    assert!(
        cx.update(|_, app| compose.read(app).is_ok()),
        "sending holds it"
    );
    press_close(main, cx);
    let open = cx.update(|_, app| app.windows().len());
    assert_eq!(open, 2, "the main window waits for the message");
    with(compose, cx, |compose, _, _| compose.sending = false);
    press_close(compose.into(), cx);
    gone(compose, cx);
    let [draft] = kept(&mail, cx).try_into().unwrap();
    assert_eq!(draft.unsent.words, "words");
}

#[test]
fn a_success_keeps_edits_made_while_sending_as_a_draft() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    for late in [false, true] {
        cx.update(|_, app| mail.update(app, |mail, cx| mail.write_new(cx)));
        let compose = opened(cx);
        write(compose, "before", cx);
        with(compose, cx, |compose, _, cx| {
            compose.edited(draft(&["ann@example.com"], "Hi"), cx);
            compose.begin_sending(cx);
        });
        if late {
            write(compose, "before and late edit", cx);
        }
        with(compose, cx, |compose, _, cx| compose.queued(Ok(()), cx));
        if !late {
            gone(compose, cx);
            assert!(kept(&mail, cx).is_empty(), "as sent, nothing stays");
            continue;
        }
        settle(cx, |app| drafts_in(&mail, app) == 1);
        let problem = cx.update(|_, app| compose.read(app).unwrap().problem.clone());
        assert_eq!(problem.map(|(severity, _)| severity), Some(Severity::Info));
        let [draft] = kept(&mail, cx).try_into().unwrap();
        assert_eq!(draft.unsent.words, "before and late edit", "kept at once");
        assert!(closes(compose, cx), "kept, it closes");
    }
}

#[test]
fn a_success_keeps_the_window_when_the_sender_or_format_changed() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), format!("{NOTE}\r\nx").as_bytes(), &[]);
    let cy = Store::open(dir.path())
        .unwrap()
        .add_account(&account("cy@example.com"))
        .unwrap();
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    for sender in [false, true] {
        cx.update(|_, app| mail.update(app, |mail, cx| mail.write_new(cx)));
        let compose = opened(cx);
        write(compose, "words", cx);
        with(compose, cx, |compose, _, cx| {
            compose.edited(draft(&["ann@example.com"], "Hi"), cx);
            compose.begin_sending(cx);
        });
        if sender {
            with(compose, cx, |compose, _, cx| compose.choose_sender(cy, cx));
        } else {
            let mut there = VisualTestContext::from_window(compose.into(), cx);
            there.run_until_parked();
            let plain = there
                .debug_bounds("segment plain")
                .expect("the format switch");
            there.simulate_click(plain.center(), gpui::Modifiers::none());
            there.run_until_parked();
            assert!(!cx.update(|_, app| compose.read(app).unwrap().markdown));
        }
        with(compose, cx, |compose, _, cx| compose.queued(Ok(()), cx));
        settle(cx, |app| drafts_in(&mail, app) == 1);
        let problem = cx.update(|_, app| compose.read(app).map(|it| it.problem.clone()));
        let severity = problem.ok().flatten().map(|(severity, _)| severity);
        assert_eq!(severity, Some(Severity::Info), "sender changed: {sender}");
        assert!(closes(compose, cx), "the unsent change was kept");
        with(compose, cx, |_, window, _| window.remove_window());
        gone(compose, cx);
        let drafts = kept(&mail, cx);
        let draft = drafts.first().expect("kept");
        assert_eq!(draft.account == cy, sender);
        assert_eq!(draft.unsent.markdown, sender);
        let store = cx.update(|_, app| mail.read(app).store.clone().unwrap());
        store.lock().unwrap().unshelve(draft.id).unwrap();
    }
}
