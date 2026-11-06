//! Body text copies; its pictures copy and save (#68).

use std::sync::atomic::Ordering;

use ely_mail_engine::{Fetched, Key, Role, Store};
use gpui::{ClipboardItem, Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};

use ely_gpui_component::primitives::Severity;

use super::{
    Keys, account,
    drafts::{rooted, told},
    reading::{opened, pictures},
    settle, window,
};
use crate::mail::Mail;

/// A 1×1 PNG.
const DOT: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

/// Two paragraphs, an inline picture of `picture`, then `web`'s.
fn seeded(dir: &std::path::Path, picture: &str, web: Option<&str>) {
    let web = web.map_or_else(String::new, |web| format!("<img src=\"{web}\">"));
    let bytes = format!(
        "From: Ann <ann@example.com>\r\nTo: Bo <bo@example.com>\r\nSubject: order\r\n\
         Date: 3 Jan 2024 10:00:00 +0000\r\n\
         Content-Type: multipart/related; boundary=r\r\n\r\n\
         --r\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
         <p>Order 42 ships</p><p>订单明天到</p><img src=\"cid:dot@ex\" alt=\"Dot\">{web}\r\n\
         --r\r\nContent-Type: image/png\r\nContent-ID: <dot@ex>\r\n\
         Content-Transfer-Encoding: base64\r\n\r\n{picture}\r\n--r--\r\n"
    )
    .into_bytes();
    let mut store = Store::open(dir).unwrap();
    let ann = store.add_account(&account("bo@example.com")).unwrap();
    let [inbox] = store.set_folders(ann, &[("INBOX", Some(Role::Inbox))]).unwrap().try_into().unwrap();
    let text = Fetched::Text { bytes: &bytes, missing: &[] };
    store.put(inbox, &Key::Imap { uidvalidity: 1, uid: 1 }, &[], text).unwrap();
}

/// The one letter, opened; its body's selector.
fn shown(mail: &Entity<Mail>, cx: &mut VisualTestContext) -> &'static str {
    settle(cx, |app| matches!(&mail.read(app).listing.mails, Some(Ok(rows)) if rows.len() == 1));
    let key = cx.update(|_, app| mail.read(app).listing.mails.clone().unwrap().unwrap()[0].key.clone());
    cx.update(|_, app| mail.update(app, |mail, cx| mail.select(&[key], cx)));
    settle(cx, |app| opened(mail, app).is_some());
    let id = cx.update(|_, app| mail.read(app).reading.shown.as_ref().unwrap().id);
    cx.run_until_parked();
    Box::leak(format!("mail-body {}", id.0).into_boxed_str())
}

fn right_at(at: gpui::Point<gpui::Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
}

/// Arrow `keys`, then Enter; menus choose on its release.
fn choose(keys: &str, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes(keys);
    cx.simulate_keystrokes("enter");
    let enter = gpui::Keystroke::parse("enter").expect("a key");
    cx.simulate_event(gpui::KeyUpEvent { keystroke: enter });
    cx.run_until_parked();
}

fn clipboard(cx: &mut VisualTestContext) -> Option<String> {
    cx.read_from_clipboard().and_then(|item| item.text())
}

const WHOLE: &str = "Order 42 ships\n订单明天到";

#[test]
fn the_body_copies_by_keys_and_from_its_menu() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), DOT, Some("https://example.invalid/a.png"));
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    let named = shown(&mail, cx);
    let body = cx.debug_bounds(named).expect("the body is drawn");
    let first = body.origin + point(px(2.0), px(4.0));
    cx.simulate_click(first, Modifiers::none());
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_keystrokes("secondary-c");
    assert_eq!(clipboard(cx).as_deref(), Some(WHOLE), "every line, in order");
    cx.write_to_clipboard(ClipboardItem::new_string("before".into()));
    right_at(first, cx);
    choose("down", cx);
    assert_eq!(clipboard(cx).as_deref(), Some(WHOLE), "Copy in the menu");
}

#[test]
fn an_inline_picture_saves_and_a_blocked_one_fetches_nothing() {
    let (web, served) = pictures();
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), DOT, Some(&web));
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    let named = shown(&mail, cx);
    let picture = cx.debug_bounds("markdown-picture").expect("the inline picture is drawn");
    let chosen = dir.path().join("dot.png");
    let target = chosen.clone();
    right_at(picture.center(), cx);
    choose("down down", cx);
    assert!(cx.did_prompt_for_new_path(), "Save asks where");
    cx.simulate_new_path_selection(move |_| Some(target));
    settle(cx, |_| chosen.exists());
    let want = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, DOT).unwrap();
    assert_eq!(std::fs::read(&chosen).unwrap(), want, "the picture's own bytes");
    let body = cx.debug_bounds(named).expect("the body is drawn");
    right_at(point(body.left() + px(4.0), body.bottom() - px(4.0)), cx);
    choose("down down", cx);
    assert_eq!(served.load(Ordering::SeqCst), 0, "a blocked picture stays unfetched");
    let asked = cx.update(|_, app| mail.read(app).body_asked.as_ref().map(|offer| offer.what.press.image.clone()));
    assert_eq!(asked, Some(Some(web)), "the press found the blocked picture");
}

#[test]
fn a_picture_that_will_not_decode_says_so() {
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), "iVBORw==", None);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    let root = rooted(&mail, cx);
    let named = shown(&mail, cx);
    let body = cx.debug_bounds(named).expect("the body is drawn");
    right_at(point(body.left() + px(4.0), body.bottom() - px(4.0)), cx);
    choose("down", cx);
    assert!(told(&root, Severity::Danger, cx), "the failed copy is told");
}

#[test]
fn an_inline_picture_copies_as_an_image() {
    use ely_platform::clipboard::{self, Clip};
    if std::env::var_os("ELY_CLIPBOARD_TEST").is_none() {
        return eprintln!("SKIP: set ELY_CLIPBOARD_TEST=1; this replaces the clipboard");
    }
    let dir = tempfile::tempdir().unwrap();
    seeded(dir.path(), DOT, None);
    let mut cx = TestAppContext::single();
    let (mail, cx) = window(&mut cx, dir.path(), Keys::default());
    shown(&mail, cx);
    clipboard::write(&Clip::Text("before".into())).expect("writes");
    let picture = cx.debug_bounds("markdown-picture").expect("the inline picture is drawn");
    right_at(picture.center(), cx);
    choose("down", cx);
    let Some(Clip::Png(png)) = clipboard::read().expect("reads") else {
        panic!("no image on the clipboard")
    };
    assert_eq!(png.get(1..4), Some(&b"PNG"[..]), "an image, not its address");
    assert_eq!(png[0], 0x89, "the PNG signature");
}
