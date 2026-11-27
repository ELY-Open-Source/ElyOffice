//! #148: delete all comments after asking; one undo.

use gpui::TestAppContext;

use super::*;
use crate::review::DeleteComments;

/// The note count and the change count the panel lists.
fn listed(cx: &mut TestAppContext, review: &Entity<Review>) -> (usize, usize) {
    cx.update(|app| {
        let review = review.read(app);
        (review.notes.len(), review.changes.len())
    })
}

/// The saved part `name`, or empty if absent.
fn part(path: &std::path::Path, name: &str) -> String {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).expect("the save opens")).expect("the save unzips");
    let mut text = String::new();
    if let Ok(mut file) = zip.by_name(name) {
        std::io::Read::read_to_string(&mut file, &mut text).expect("reads");
    }
    text
}

#[test]
fn every_comment_goes_after_asking_and_one_undo_brings_them_back() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    let id = blank(&mut cx, &docs);
    let review = panel(&mut cx, &docs);
    for (words, note) in [("Hello", "Note A"), (" there", "Note B")] {
        type_into(&mut cx, &docs, words);
        with_window(&mut cx, &docs, |docs, window, cx| {
            docs.on_comment(&InsertComment, window, cx);
            let Some(Dialog::Ask { ask, .. }) = &docs.dialog else {
                panic!("the comment prompt is open");
            };
            let ask = ask.clone();
            docs.answer(&ask, note, cx);
        });
    }
    settle(&cx, |app| review.read(app).threads.len() == 2);
    let key = cx.update(|app| review.read(app).threads[0].key.clone());
    cx.update(|app| review.update(app, |review, cx| review.select(&key, cx)));
    cx.update(|app| review.update(app, |review, cx| review.reply("An answer", cx)));
    settle(&cx, |app| review.read(app).notes.len() == 3);
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_track(&TrackChanges, window, cx));
    type_into(&mut cx, &docs, " kept");
    settle(&cx, |app| review.read(app).changes.len() == 1);
    // It asks first, counting the reply.
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_delete_comments(&DeleteComments, window, cx));
    let asked = cx.update(|app| matches!(docs.read(app).dialog, Some(Dialog::DeleteComments { count: 3 })));
    assert!(asked, "the warning counts three comments");
    assert_eq!(listed(&mut cx, &review), (3, 1), "asking deletes nothing");
    cx.update(|app| docs.update(app, |docs, cx| docs.delete_comments(cx)));
    settle(&cx, |app| review.read(app).notes.is_empty());
    assert_eq!(listed(&mut cx, &review), (0, 1), "the change stays");
    assert!(cx.update(|app| docs.read(app).dialog.is_none()), "the warning closes");
    // One undo brings all three back.
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.undo", &[], cx)));
    settle(&cx, |app| review.read(app).notes.len() == 3);
    assert_eq!(listed(&mut cx, &review), (3, 1), "one undo restores them");
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.redo", &[], cx)));
    settle(&cx, |app| review.read(app).notes.is_empty());
    // With none left it does not ask.
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_delete_comments(&DeleteComments, window, cx));
    assert!(cx.update(|app| docs.read(app).dialog.is_none()), "nothing to delete, no warning");
    // The .docx keeps text and change, no comments.
    settled(&cx, &docs);
    let path = dir.path().join("cleared.docx");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    written(&cx, &path, None);
    let body = part(&path, "word/document.xml");
    assert!(body.contains("Hello") && body.contains("there"), "the words stay");
    assert!(body.contains("<w:ins "), "the tracked change stays");
    assert!(!body.contains("commentReference"), "{body}");
    assert!(!part(&path, "word/comments.xml").contains("<w:comment "), "no comment is saved");
    // Closed and reopened: the change, no comments.
    cx.update(|app| docs.update(app, |docs, cx| docs.close_tab(id, cx)));
    settle(&cx, |app| docs.read(app).core.tabs.is_empty());
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(path.clone(), cx));
    settle(&cx, |app| docs.read(app).active_tab().is_some_and(|tab| tab.id != id));
    settled(&cx, &docs);
    let review = panel(&mut cx, &docs);
    settle(&cx, |app| review.read(app).changes.len() == 1);
    assert_eq!(listed(&mut cx, &review), (0, 1), "reopened: no comments, the change");
}
