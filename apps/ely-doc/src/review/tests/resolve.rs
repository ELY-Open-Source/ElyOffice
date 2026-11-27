//! #145: one thread resolves and reopens; the rest stays.

use gpui::TestAppContext;

use super::*;

/// Whether the thread opened by `body` is resolved.
fn resolved(cx: &mut TestAppContext, review: &Entity<Review>, body: &str) -> Option<bool> {
    cx.update(|app| {
        let threads = &review.read(app).threads;
        let thread = threads.iter().find(|it| it.comments.first().is_some_and(|it| it.body == body));
        thread.map(|it| it.resolved)
    })
}

#[test]
fn one_thread_resolves_in_one_undo_step_and_the_rest_stays() {
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
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_track(&TrackChanges, window, cx));
    type_into(&mut cx, &docs, " kept");
    settle(&cx, |app| review.read(app).changes.len() == 1);
    let key = |cx: &mut TestAppContext| {
        cx.update(|app| {
            let threads = &review.read(app).threads;
            threads.iter().find(|it| it.comments[0].body == "Note A").expect("thread A").key.clone()
        })
    };
    let a = key(&mut cx);
    cx.update(|app| review.update(app, |review, cx| review.resolve(&a, true, cx)));
    let ask = |cx: &mut TestAppContext| (resolved(cx, &review, "Note A"), resolved(cx, &review, "Note B"));
    settle(&cx, |app| review.read(app).threads.iter().any(|it| it.resolved));
    assert_eq!(ask(&mut cx), (Some(true), Some(false)), "only A resolves");
    assert_eq!(cx.update(|app| review.read(app).changes.len()), 1, "the change stays");
    // One undo reopens it.
    cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.undo", &[], cx)));
    settle(&cx, |app| review.read(app).threads.iter().all(|it| !it.resolved));
    let a = key(&mut cx);
    cx.update(|app| review.update(app, |review, cx| review.resolve(&a, true, cx)));
    settle(&cx, |app| review.read(app).threads.iter().any(|it| it.resolved));
    // The .docx keeps it resolved.
    settled(&cx, &docs);
    let path = dir.path().join("resolved.docx");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    written(&cx, &path, None);
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&path).expect("the save opens")).expect("the save unzips");
    let mut extended = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("word/commentsExtended.xml").expect("resolved state is saved"), &mut extended)
        .expect("reads");
    assert_eq!(extended.matches("w15:done=\"1\"").count(), 1, "{extended}");
    // Reopen.
    let a = key(&mut cx);
    cx.update(|app| review.update(app, |review, cx| review.resolve(&a, false, cx)));
    settle(&cx, |app| review.read(app).threads.iter().all(|it| !it.resolved));
    assert_eq!(ask(&mut cx), (Some(false), Some(false)));
    assert_eq!(cx.update(|app| review.read(app).changes.len()), 1, "the change still stays");
}
