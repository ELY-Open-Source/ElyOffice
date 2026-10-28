//! Protecting a sheet: matching passwords lock; a wrong one fails.

use ely_gpui_component::forms::TextInput;
use ely_lok::sheet;

use super::data::asked;
use super::output::{read, type_in};
use super::*;

fn password(dialog: &Dialog) -> &Entity<TextInput> {
    match dialog {
        Dialog::Protect { protecting, .. } => &protecting.password,
        _ => unreachable!("a protect dialog"),
    }
}

fn confirm(dialog: &Dialog) -> &Entity<TextInput> {
    match dialog {
        Dialog::Protect { protecting, .. } => &protecting.confirm,
        _ => unreachable!("a protect dialog"),
    }
}

fn protected(cx: &mut TestAppContext, book: &Entity<Book>) -> bool {
    read(cx, book, sheet::protection, sheet::PROTECTION).protected
}

#[test]
fn a_wrong_password_leaves_the_sheet_protected() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    let ask = |cx: &mut TestAppContext| {
        update(cx, &book, |book, cx| book.ask_protect(cx));
        asked(cx, &book, |dialog| matches!(dialog, Dialog::Protect { .. }));
    };
    ask(&mut cx);
    type_in(&mut cx, &book, password, "pw");
    type_in(&mut cx, &book, confirm, "px");
    update(&mut cx, &book, |book, cx| book.protect_asked(cx));
    assert!(cx.update(|app| book.read(app).dialog.is_some()), "a mismatch stays");
    type_in(&mut cx, &book, confirm, "pw");
    update(&mut cx, &book, |book, cx| book.protect_asked(cx));
    assert!(protected(&mut cx, &book));
    ask(&mut cx);
    type_in(&mut cx, &book, password, "no");
    update(&mut cx, &book, |book, cx| book.protect_asked(cx));
    cx.run_until_parked();
    assert!(protected(&mut cx, &book), "still protected");
    ask(&mut cx);
    type_in(&mut cx, &book, password, "pw");
    update(&mut cx, &book, |book, cx| book.protect_asked(cx));
    cx.run_until_parked();
    assert!(!protected(&mut cx, &book));
}
