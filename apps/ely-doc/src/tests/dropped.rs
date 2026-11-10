//! Files dropped on the page (#81).

use ely_office::prelude::*;
use gpui::TestAppContext;

#[test]
fn a_dropped_file_that_is_no_picture_opens_as_files_do() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = crate::tests::docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    cx.update(ely_ui::install_no_launch_files);
    crate::tests::blank(&mut cx, &docs);
    let view = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.clone());
    // The page passes on what it does not take (#81).
    let other = dir.path().join("letter.docx");
    cx.update(|app| view.update(app, |_, cx| cx.emit(ely_lok::Dropped(vec![other.clone()]))));
    cx.run_until_parked();
    assert_eq!(cx.update(ely_ui::launch_files), [other], "the file is queued to open here");
}
