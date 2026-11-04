//! LOK-11: linked sources update only when the user asks.

use std::path::Path;

use ely_office::Linked;

use super::data::{cells, run};
use super::*;

/// A one-sheet book whose S1.A1 holds `value`.
fn source(path: &Path, value: u32) {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S1"><table:table-row>
 <table:table-cell office:value-type="float" office:value="{value}"/></table:table-row></table:table>
 </office:spreadsheet></office:body>
</office:document>"#
    );
    std::fs::write(path, xml).expect("writes the source");
}

/// `reader.xlsx`; A1, A2… read S1.A1 of each source, made here.
fn reader(cx: &mut TestAppContext, book: &Entity<Book>, dir: &Path, sources: &[&Path]) -> std::path::PathBuf {
    blank(cx, book);
    for (row, path) in sources.iter().enumerate() {
        source(path, 1);
        let (folder, name) = (path.parent().expect("a folder"), path.file_name().expect("a name"));
        let formula = format!("='{}/[{}]S1'!A1", folder.display(), name.to_string_lossy());
        run(cx, book, "cell.goto", &[&format!("A{}", row + 1)]);
        run(cx, book, "cell.enter", &[&formula]);
    }
    let saved = dir.join("reader.xlsx");
    cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        let mut worker = book.core.worker.lock().expect("the worker lock");
        tab.doc.save_as(&mut worker, &ely_lok::file_url(&saved), "xlsx").expect("saves");
    });
    saved
}

fn opened(cx: &mut TestAppContext, book: &Entity<Book>, path: &Path) -> u64 {
    update(cx, book, |book, cx| book.open_file(path.to_path_buf(), cx));
    let name = path.file_name().expect("a name").to_string_lossy().to_string();
    settle(cx, |app| book.read(app).active_tab().is_some_and(|tab| tab.name == name));
    cx.update(|app| book.read(app).active_tab().expect("the tab").id)
}

#[track_caller]
fn linked(cx: &TestAppContext, book: &Entity<Book>, want: Linked) {
    settle(cx, |app| book.read(app).active_tab().is_some_and(|tab| *tab.links.state() == want));
}

#[test]
fn the_user_updates_links_and_hears_of_a_later_save() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("source.fods");
    let saved = reader(&mut cx, &book, dir.path(), &[&path]);
    source(&path, 2);
    let id = opened(&mut cx, &book, &saved);
    linked(&cx, &book, Linked::Saved(Vec::new()));
    assert_eq!(cells(&mut cx, &book, "A1"), "1", "opening shows the saved value");
    update(&mut cx, &book, |book, cx| book.update_links(id, cx));
    linked(&cx, &book, Linked::Updated(Vec::new()));
    assert_eq!(cells(&mut cx, &book, "A1"), "2", "the update reads the source");
    source(&path, 3);
    linked(&cx, &book, Linked::Changed(path.display().to_string()));
    assert_eq!(cells(&mut cx, &book, "A1"), "2", "a later save waits for the user");
    update(&mut cx, &book, |book, cx| book.update_links(id, cx));
    linked(&cx, &book, Linked::Updated(Vec::new()));
    assert_eq!(cells(&mut cx, &book, "A1"), "3");
    std::fs::remove_file(&path).expect("the source goes");
    linked(&cx, &book, Linked::Changed(path.display().to_string()));
    update(&mut cx, &book, |book, cx| book.update_links(id, cx));
    linked(&cx, &book, Linked::Updated(vec![path.display().to_string()]));
    assert_eq!(cells(&mut cx, &book, "A1"), "3", "a missing source keeps the value");
}

#[test]
fn a_missing_source_is_named_at_open() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("source.fods");
    let saved = reader(&mut cx, &book, dir.path(), &[&path]);
    std::fs::remove_file(&path).expect("the source goes");
    opened(&mut cx, &book, &saved);
    linked(&cx, &book, Linked::Saved(vec![path.display().to_string()]));
    assert_eq!(cells(&mut cx, &book, "A1"), "1", "the saved value stays");
}

#[test]
fn a_source_in_a_missing_folder_leaves_the_others_watched() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let (path, sub) = (dir.path().join("source.fods"), dir.path().join("sub"));
    let gone = sub.join("gone.fods");
    std::fs::create_dir(&sub).expect("the folder");
    let saved = reader(&mut cx, &book, dir.path(), &[&path, &gone]);
    std::fs::remove_dir_all(&sub).expect("the folder goes");
    let id = opened(&mut cx, &book, &saved);
    linked(&cx, &book, Linked::Saved(vec![gone.display().to_string()]));
    update(&mut cx, &book, |book, cx| book.update_links(id, cx));
    linked(&cx, &book, Linked::Updated(vec![gone.display().to_string()]));
    source(&path, 2);
    linked(&cx, &book, Linked::Changed(path.display().to_string()));
}
