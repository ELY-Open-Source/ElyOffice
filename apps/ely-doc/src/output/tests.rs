//! Export to each format (DOC-15).

use gpui::TestAppContext;

use ely_office::prelude::*;

use crate::tests::{blank, docs, settle, settled, type_into, with_window};
use crate::actions::Dialog;
use super::{Format, suggested};

#[test]
fn each_format_writes_a_copy_and_the_tab_stays() {
    // Markdown asks ElyMark, under the test data root.
    ely_platform::test_root();
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Exported año ñandú 中文");
    settled(&cx, &docs);
    let named = |cx: &mut TestAppContext| cx.update(|app| {
        let tab = docs.read(app).tab(id).expect("the tab");
        (tab.name.clone(), tab.session.path().map(std::path::Path::to_path_buf), tab.dirty(app))
    });
    let before = named(&mut cx);
    // Each format and how its bytes start.
    let cases: [(Format, &[u8]); 7] = [
        (Format::Pdf, b"%PDF"),
        (Format::Docx, b"PK"),
        (Format::Odt, b"PK"),
        (Format::Rtf, b"{\\rtf"),
        (Format::Txt, "\u{feff}Exported año ñandú 中文".as_bytes()),
        (Format::Epub, b"PK"),
        (Format::Markdown, b"Exported"),
    ];
    for (chosen, start) in cases {
        let out = dir.path().join(format!("out.{}", chosen.ext()));
        with_window(&mut cx, &docs, |docs, _, cx| {
            docs.ask_output(false, cx);
            let Some(Dialog::Output { format, .. }) = &mut docs.dialog else { panic!("the export dialog") };
            *format = chosen;
            docs.output(cx);
        });
        assert!(cx.did_prompt_for_new_path(), "{chosen:?} asks where");
        cx.simulate_new_path_selection(|_| Some(out.clone()));
        settle(&cx, |_| std::fs::read(&out).is_ok_and(|bytes| bytes.starts_with(start)));
        settle(&cx, |app| std::rc::Rc::strong_count(&docs.read(app).tab(id).expect("the tab").busy) == 1);
    }
    assert_eq!(named(&mut cx), before, "an export is a copy");
}

#[test]
fn the_open_file_is_never_the_offered_name() {
    let offered = |source: &str, format| suggested(source, format, |stem| format!("{stem} (copy)"));
    assert_eq!(offered("letter.docx", Format::Pdf), "letter.pdf");
    assert_eq!(offered("letter.docx", Format::Docx), "letter (copy).docx");
    assert_eq!(offered("Letter.DOCX", Format::Docx), "Letter (copy).docx");
    assert_eq!(offered("Untitled 1", Format::Odt), "Untitled 1.odt");
}

#[test]
fn every_format_is_found_by_its_extension() {
    for ext in ["pdf", "docx", "odt", "rtf", "txt", "epub", "md", "PDF"] {
        assert!(Format::of(ext).is_some(), "{ext}");
    }
    assert_eq!(Format::of("html"), None);
}
