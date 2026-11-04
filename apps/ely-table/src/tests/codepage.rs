//! CSV imports pick the language's code page.

use super::data::{asked, cells};
use super::*;

#[test]
fn a_french_excel_csv_opens_in_its_code_page() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("menu.csv");
    // French Excel: semicolons, Windows-1252 bytes.
    std::fs::write(&path, b"Plat;Prix\nCr\xe8me br\xfbl\xe9e;4,5\n").expect("writes");
    cx.update(|app| ely_gpui_component::i18n::I18n::set_locale("fr-FR", app));
    update(&mut cx, &book, |book, cx| book.ask_import(path.clone(), cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(pending) if !pending.reading));
    cx.update(|app| match book.read(app).dialog.as_ref() {
        Some(Dialog::Import(pending)) => {
            assert_eq!(pending.how.encoding, ely_lok::sheet::csv::Encoding::Windows1252);
            assert_eq!(pending.how.separator, ';');
        }
        _ => unreachable!("an import dialog"),
    });
    update(&mut cx, &book, |book, cx| book.import_asked(cx));
    settle(&cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.name == "menu.csv"));
    assert_eq!(cells(&mut cx, &book, "A2"), "Crème brûlée");
}

#[test]
fn a_german_excel_csv_opens_in_its_code_page() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("orte.csv");
    // German Excel: semicolons, Windows-1252 bytes.
    std::fs::write(&path, b"Ort;Preis\nGr\xfc\xdfe aus K\xf6ln;4,5\n").expect("writes");
    cx.update(|app| ely_gpui_component::i18n::I18n::set_locale("de-DE", app));
    update(&mut cx, &book, |book, cx| book.ask_import(path.clone(), cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(pending) if !pending.reading));
    cx.update(|app| match book.read(app).dialog.as_ref() {
        Some(Dialog::Import(pending)) => {
            assert_eq!(pending.how.encoding, ely_lok::sheet::csv::Encoding::Windows1252);
            assert_eq!(pending.how.separator, ';');
        }
        _ => unreachable!("an import dialog"),
    });
    update(&mut cx, &book, |book, cx| book.import_asked(cx));
    settle(&cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.name == "orte.csv"));
    assert_eq!(cells(&mut cx, &book, "A2"), "Grüße aus Köln");
}

#[test]
fn the_default_encoding_follows_the_apps_language() {
    use ely_gpui_component::i18n::I18n;
    use ely_lok::sheet::csv::Encoding;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("gb.csv");
    // 你好 in GB18030: not UTF-8.
    std::fs::write(&path, [0xC4, 0xE3, 0xBA, 0xC3, b'\n']).expect("writes");
    let pages = [
        ("zh-CN", Encoding::Gb18030),
        ("en-US", Encoding::Windows1252),
        ("es-ES", Encoding::Windows1252),
        ("fr-FR", Encoding::Windows1252),
        ("de-DE", Encoding::Windows1252),
        ("ja-JP", Encoding::ShiftJis),
        ("ko-KR", Encoding::Windows949),
        ("vi-VN", Encoding::Windows1258),
    ];
    for (tag, expected) in pages {
        cx.update(|app| I18n::set_locale(tag, app));
        update(&mut cx, &book, |book, cx| book.ask_import(path.clone(), cx));
        asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(_)));
        let found = cx.update(|app| match book.read(app).dialog.as_ref() {
            Some(Dialog::Import(pending)) => pending.how.encoding,
            _ => unreachable!("an import dialog"),
        });
        assert_eq!(found, expected, "{tag}");
        update(&mut cx, &book, |book, cx| {
            book.dialog = None;
            cx.notify();
        });
    }
}

#[test]
fn a_korean_windows_csv_imports_beyond_euc_kr() {
    use ely_gpui_component::i18n::I18n;
    use ely_lok::sheet::csv::Encoding;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("ko.csv");
    // 똠방각하 in Windows-949; 똠 is outside EUC-KR.
    std::fs::write(&path, [0x8C, 0x63, 0xB9, 0xE6, 0xB0, 0xA2, 0xC7, 0xCF, b',', b'1', b'\n']).expect("writes");
    cx.update(|app| I18n::set_locale("ko-KR", app));
    update(&mut cx, &book, |book, cx| book.ask_import(path.clone(), cx));
    asked(&cx, &book, |dialog| matches!(dialog, Dialog::Import(pending) if !pending.reading));
    cx.update(|app| match book.read(app).dialog.as_ref() {
        Some(Dialog::Import(pending)) => {
            assert_eq!(pending.how.encoding, Encoding::Windows949);
            assert_eq!(pending.rows[0], ["똠방각하", "1"], "the preview");
        }
        _ => unreachable!("an import dialog"),
    });
    update(&mut cx, &book, |book, cx| book.import_asked(cx));
    settle(&cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.name == "ko.csv"));
    assert_eq!(cells(&mut cx, &book, "A1"), "똠방각하", "the kernel reads Windows-949");
}
