use gpui::{Entity, TestAppContext};

use super::*;
use crate::actions::Dialog;
use crate::refs::InsertComment;
use crate::tests::{blank, docs, settle, settled, text_of, type_into, with_window, written};
use ely_office::prelude::*;

mod clicks;
mod views;

/// The review panel on the active tab.
fn panel(cx: &mut TestAppContext, docs: &Entity<Docs>) -> Entity<Review> {
    cx.update(|app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        let (view, doc, work) = (tab.view.clone(), tab.doc.clone(), tab.work.clone());
        let review = docs.read(app).review.clone();
        review.update(app, |review, cx| review.show(Some((&view, &doc, &work)), cx));
        review
    })
}

#[test]
fn comments_thread_and_changes_accept_or_reject() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    // The page's washes take their colors from the theme.
    cx.update(ely_gpui_component::init_for_tests);
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Hello");
    let review = panel(&mut cx, &docs);
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_comment(&InsertComment, window, cx);
        let Some(Dialog::Ask { ask, .. }) = &docs.dialog else {
            panic!("the comment prompt is open");
        };
        let ask = ask.clone();
        docs.answer(&ask, "First note", cx);
    });
    assert_eq!(
        cx.update(|app| docs.read(app).side),
        Side::Review,
        "the panel shows"
    );
    settle(&cx, |app| review.read(app).threads.len() == 1);
    let key = cx.update(|app| review.read(app).threads[0].key.clone());
    cx.update(|app| review.update(app, |review, cx| review.select(&key, cx)));
    cx.update(|app| review.update(app, |review, cx| review.reply("An answer", cx)));
    settle(&cx, |app| {
        review
            .read(app)
            .threads
            .first()
            .is_some_and(|thread| thread.comments.len() == 2)
    });
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_track(&TrackChanges, window, cx)
    });
    settle(&cx, |app| {
        let tab = docs.read(app).active_tab().expect("a tab");
        tab.view.read(app).state(".uno:TrackChanges") == Some("true")
    });
    type_into(&mut cx, &docs, " kept");
    settle(&cx, |app| review.read(app).changes.len() == 1);
    let index = cx.update(|app| review.read(app).changes[0].index.to_string());
    cx.update(|app| review.update(app, |review, cx| review.run("track.accept", &[&index], cx)));
    settle(&cx, |app| review.read(app).changes.is_empty());
    type_into(&mut cx, &docs, " gone");
    settle(&cx, |app| review.read(app).changes.len() == 1);
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_reject_all(&RejectAll, window, cx)
    });
    settle(&cx, |app| review.read(app).changes.is_empty());
    settled(&cx, &docs);
    let path = dir.path().join("review.odt");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, path.clone(), None, window, cx)
    });
    written(&cx, &path, None);
    let text = text_of(&mut cx, &docs, &path);
    for mark in ["First note", "An answer", " kept"] {
        assert!(text.contains(mark), "{mark} lands");
    }
    assert!(!text.contains("gone"), "the rejected words go");
}

#[test]
fn one_unreadable_time_spares_the_other_notes() {
    let note = |id: &str, parent: &str, at: &str| Note {
        id: id.into(),
        parent: parent.into(),
        author: "A".into(),
        text: "t".into(),
        resolved: false,
        at: at.into(),
        anchor: (0, 0, 0, 0),
        ranges: Vec::new(),
    };
    // The kernel writes an empty date as 0-00-00.
    let mut undated = note("2", "0", "0-00-00");
    undated.author = "Undated".into();
    let notes = vec![note("1", "0", "2025-03-01T10:00:00"), undated, note("3", "1", "2025-03-02T09:00:00")];
    let (threads, ids, unread) = threads::threads(&notes);
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].comments.len(), 2, "the reply stays in its thread");
    // Edit and delete find each shown comment's note.
    assert_eq!(ids[&threads[0].key], ["1", "3"], "the unread note is no comment");
    assert_eq!((unread.len(), unread[0].author.as_str(), unread[0].at.as_str()), (1, "Undated", "0-00-00"));
    // Newest first, as the kernel lists them: shown oldest first.
    let at = "2025-03-01T10:00:00";
    let listed = [note("10", "1", at), note("9", "1", at), note("1", "0", at)];
    let (_, order, _) = threads::threads(&listed);
    assert_eq!(order[&gpui::SharedString::from("1")],["1", "9", "10"], "the first comment, then replies as made");
    // Through the refresh path: changes stay, the line names both.
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = gpui::TestAppContext::single();
    let Some(docs) = crate::tests::docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let review = cx.update(|app| docs.read(app).review.clone());
    let change = Change { index: 7, author: "B".into(), kind: "Insert".into(), description: String::new(), words: String::new(), at: "2025-03-01T10:00:00".into(), anchor: None, ranges: Vec::new(), node: 0, from: 0, to: 0, paragraph: None };
    let lines = cx.update(|cx| review.update(cx, |review, cx| {
        review.take_lists(notes, vec![change]);
        assert_eq!(review.changes.len(), 1, "the changes stay");
        review.unread_lines(cx)
    }));
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("Undated") && lines[0].contains(&unread[0].error), "{}", lines[0]);
}

#[test]
fn commented_words_in_a_table_cell_wash_and_the_picked_thread_darkens() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    blank(&mut cx, &docs);
    let review = panel(&mut cx, &docs);
    let view = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.clone());
    type_into(&mut cx, &docs, "Top");
    // #118: a comment on words selected in a table cell.
    docs.update(&mut cx, |docs, cx| docs.insert_table(2.0, 2.0, cx));
    type_into(&mut cx, &docs, "Cell words");
    view.update(&mut cx, |view, _| view.run("edit.select_all", &[]).expect("the cell selects"));
    settle(&cx, |app| view.read(app).settled());
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_comment(&InsertComment, window, cx);
        let Some(Dialog::Ask { ask, .. }) = &docs.dialog else {
            panic!("the comment prompt is open");
        };
        let ask = ask.clone();
        docs.answer(&ask, "About the cell", cx);
    });
    settle(&cx, |app| !view.read(app).washed().is_empty());
    let first = cx.update(|app| view.read(app).washed().to_vec());
    assert!(first.iter().all(|(rect, _)| rect.2 > 0 && rect.3 > 0), "the words have an extent: {first:?}");
    // A click at twips, as the page posts one.
    let click = |cx: &mut TestAppContext, (x, y): (i32, i32)| {
        cx.update(|app| {
            let docs = docs.read(app);
            let doc = docs.active_tab().expect("a tab").doc.clone();
            let mut worker = docs.core.worker.lock().expect("the worker lock");
            for kind in [0, 1] {
                let mouse = ely_lok::Mouse { kind, x, y, count: 1, buttons: 1, modifiers: 0 };
                worker.post_mouse(&doc, mouse).expect("the click posts");
            }
        })
    };
    // A plain wash is 0.18 strong, a picked one 0.36.
    let picked = |app: &gpui::App| view.read(app).washed().first().map(|wash| wash.1.a > 0.3);
    let (words, _) = first[0];
    let (inside, above) = ((words.0 + words.2 / 2, words.1 + words.3 / 2), (words.0, words.1 - words.3));
    // The caret leaves the words, enters them, then leaves again.
    click(&mut cx, above);
    settle(&cx, |app| review.read(app).active.is_none() && picked(app) == Some(false));
    click(&mut cx, inside);
    settle(&cx, |app| review.read(app).active.is_some() && picked(app) == Some(true));
    click(&mut cx, above);
    settle(&cx, |app| review.read(app).active.is_none() && picked(app) == Some(false));
    let key = cx.update(|app| review.read(app).threads[0].key.clone());
    cx.update(|app| review.update(app, |review, cx| review.select(&key, cx)));
    assert_eq!(cx.update(|app| picked(app)), Some(true), "picking the thread darkens it");
    // The sidebar's pick outlasts the caret passing through and out.
    click(&mut cx, inside);
    settle(&cx, |app| review.read(app).inside.is_some());
    click(&mut cx, above);
    settle(&cx, |app| review.read(app).inside.is_none());
    cx.update(|app| assert_eq!(review.read(app).active.as_ref(), Some(&key), "the sidebar's pick stays"));
    assert_eq!(cx.update(|app| picked(app)), Some(true), "and stays dark");
    // Paragraphs above the table move the words; their wash follows.
    view.update(&mut cx, |view, _| {
        let args = [r"\QTop\E", "false", "3", "0", "0", "2", r"Top\n\n\n", "true"];
        view.run("text.replace_breaks", &args).expect("the breaks queue")
    });
    settle(&cx, |app| view.read(app).washed().first().is_some_and(|moved| moved.0.1 > first[0].0.1));
}

#[test]
fn comments_edit_and_delete_one_undo_step_each_and_docx_keeps_them() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Hello there");
    let review = panel(&mut cx, &docs);
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_comment(&InsertComment, window, cx);
        let Some(Dialog::Ask { ask, .. }) = &docs.dialog else {
            panic!("the comment prompt is open");
        };
        let ask = ask.clone();
        docs.answer(&ask, "First note", cx);
    });
    settle(&cx, |app| review.read(app).threads.len() == 1);
    let key = cx.update(|app| review.read(app).threads[0].key.clone());
    cx.update(|app| review.update(app, |review, cx| review.select(&key, cx)));
    let bodies = |cx: &mut TestAppContext| -> Vec<String> {
        cx.update(|app| review.read(app).threads.iter().flat_map(|it| it.comments.iter().map(|it| it.body.to_string())).collect())
    };
    for (n, answer) in [(2, "An answer"), (3, "Second answer")] {
        cx.update(|app| review.update(app, |review, cx| review.reply(answer, cx)));
        settle(&cx, |app| review.read(app).threads.first().is_some_and(|it| it.comments.len() == n));
    }
    assert_eq!(bodies(&mut cx), ["First note", "An answer", "Second answer"], "replies in the order given");
    let view = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.clone());
    let caret = cx.update(|app| view.read(app).cursor());
    let undo = |cx: &mut TestAppContext| cx.update(|app| docs.update(app, |docs, cx| docs.run_active("edit.undo", &[], cx)));
    // Edit: the text changes in place, one step back.
    cx.update(|app| review.update(app, |review, cx| review.edit(&key, 1, "Changed\nanswer", cx)));
    settle(&cx, |app| review.read(app).threads[0].comments[1].body == "Changed\nanswer");
    undo(&mut cx);
    settle(&cx, |app| review.read(app).threads[0].comments[1].body == "An answer");
    cx.update(|app| review.update(app, |review, cx| review.edit(&key, 1, "Changed\nanswer", cx)));
    settle(&cx, |app| review.read(app).threads[0].comments[1].body == "Changed\nanswer");
    // A reply goes alone, one step back.
    cx.update(|app| review.update(app, |review, cx| review.delete(&key, 2, cx)));
    settle(&cx, |app| review.read(app).threads[0].comments.len() == 2);
    undo(&mut cx);
    settle(&cx, |app| review.read(app).threads[0].comments.len() == 3);
    cx.update(|app| review.update(app, |review, cx| review.delete(&key, 2, cx)));
    settle(&cx, |app| review.read(app).threads[0].comments.len() == 2);
    assert_eq!(bodies(&mut cx), ["First note", "Changed\nanswer"]);
    settled(&cx, &docs);
    assert_eq!(cx.update(|app| view.read(app).cursor()), caret, "the cursor stays");
    // The .docx keeps the edit and the deletion.
    let path = dir.path().join("notes.docx");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    written(&cx, &path, None);
    let text = text_of(&mut cx, &docs, &path);
    assert!(text.contains("First note") && text.contains("Changed") && text.contains("answer"), "the edit is saved");
    assert!(!text.contains("Second answer") && !text.contains("An answer"), "the old texts are gone");
    // The first comment takes its replies, one step back.
    cx.update(|app| review.update(app, |review, cx| review.delete(&key, 0, cx)));
    settle(&cx, |app| review.read(app).threads.is_empty());
    undo(&mut cx);
    settle(&cx, |app| review.read(app).threads.first().is_some_and(|it| it.comments.len() == 2));
    assert_eq!(bodies(&mut cx), ["First note", "Changed\nanswer"], "the thread comes back whole");
    // Undo may give the thread new ids.
    let key = cx.update(|app| review.read(app).threads[0].key.clone());
    cx.update(|app| review.update(app, |review, cx| review.delete(&key, 0, cx)));
    settle(&cx, |app| review.read(app).threads.is_empty());
    settled(&cx, &docs);
    assert_eq!(cx.update(|app| view.read(app).cursor()), caret, "the cursor stays");
    let before = std::fs::read(&path).expect("the first save");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    written(&cx, &path, Some(before.as_slice()));
    let text = text_of(&mut cx, &docs, &path);
    assert!(text.contains("Hello there") && !text.contains("First note") && !text.contains("Changed"), "no comment is left");
}

#[test]
fn arrows_step_through_changes_and_comments_and_hover_names_the_author() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    blank(&mut cx, &docs);
    let review = panel(&mut cx, &docs);
    let comment = |cx: &mut TestAppContext, words: &str| {
        with_window(cx, &docs, |docs, window, cx| {
            docs.on_comment(&InsertComment, window, cx);
            let Some(Dialog::Ask { ask, .. }) = &docs.dialog else {
                panic!("the comment prompt is open");
            };
            let ask = ask.clone();
            docs.answer(&ask, words, cx);
        });
    };
    type_into(&mut cx, &docs, "Hello");
    comment(&mut cx, "First");
    settle(&cx, |app| review.read(app).threads.len() == 1);
    type_into(&mut cx, &docs, " world");
    comment(&mut cx, "Second");
    settle(&cx, |app| review.read(app).threads.len() == 2);
    let track = |cx: &mut TestAppContext, on: &'static str| {
        with_window(cx, &docs, |docs, window, cx| docs.on_track(&TrackChanges, window, cx));
        settle(cx, |app| {
            let tab = docs.read(app).active_tab().expect("a tab");
            tab.view.read(app).state(".uno:TrackChanges") == Some(on)
        });
    };
    track(&mut cx, "true");
    type_into(&mut cx, &docs, " Alpha");
    track(&mut cx, "false");
    type_into(&mut cx, &docs, " plain");
    track(&mut cx, "true");
    type_into(&mut cx, &docs, " Beta");
    settle(&cx, |app| review.read(app).changes.len() == 2);
    // Each changed rect carries its author's name.
    settle(&cx, |app| {
        let review = review.read(app);
        let view = review.shown.as_ref().expect("a tab").view.read(app);
        let rects: usize = review.changes.iter().map(|change| change.ranges.len()).sum();
        rects > 0
            && view.tipped().len() == rects
            && view.tipped().iter().all(|(_, tip)| tip.contains(review.changes[0].author.as_str()))
    });
    let inside = |app: &gpui::App, at: usize| {
        let review = review.read(app);
        let caret = review.shown.as_ref().expect("a tab").view.read(app).cursor();
        caret.is_some_and(|caret| nav::holds(&review.changes[at].ranges, caret))
    };
    // The changes in reading order, whatever order the kernel lists.
    let mut order = [0, 1];
    let anchors = cx.update(|app| review.read(app).changes.iter().map(|change| change.anchor).collect::<Vec<_>>());
    order.sort_by_key(|&at| anchors[at].map(|(x, y, _, _)| (y, x)));
    let go = |cx: &mut TestAppContext, forward: bool| {
        cx.update(|app| review.update(app, |review, cx| review.step_change(forward, cx)));
    };
    // From the end, forward wraps round.
    for want in [order[0], order[1], order[0]] {
        go(&mut cx, true);
        settle(&cx, |app| inside(app, want));
    }
    go(&mut cx, false);
    settle(&cx, |app| inside(app, order[1]));
    // Comments step by their place in the text.
    let first_key = |app: &gpui::App, words: &str| {
        let review = review.read(app);
        review.threads.iter().find(|thread| thread.comments[0].body == words).expect("the thread").key.clone()
    };
    let (first, second) = cx.update(|app| (first_key(app, "First"), first_key(app, "Second")));
    // No thread picked: the caret, at the end, decides.
    cx.update(|app| review.update(app, |review, _| review.active = None));
    for want in [&first, &second, &first] {
        cx.update(|app| review.update(app, |review, cx| review.step_comment(true, cx)));
        settle(&cx, |app| review.read(app).active.as_ref() == Some(want));
    }
    cx.update(|app| review.update(app, |review, cx| review.step_comment(false, cx)));
    settle(&cx, |app| review.read(app).active.as_ref() == Some(&second));
}

#[test]
fn a_reviser_filter_counts_and_steps_only_their_changes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    blank(&mut cx, &docs);
    let review = panel(&mut cx, &docs);
    type_into(&mut cx, &docs, "Start");
    with_window(&mut cx, &docs, |docs, window, cx| docs.on_track(&TrackChanges, window, cx));
    let author = |cx: &mut TestAppContext, name: &str| {
        let worker = cx.update(|app| review.read(app).worker.clone());
        worker.lock().expect("the worker").set_author(name).expect("the author is set");
    };
    author(&mut cx, "Ann");
    type_into(&mut cx, &docs, " one");
    author(&mut cx, "Zoe");
    type_into(&mut cx, &docs, " two");
    author(&mut cx, "Ann");
    type_into(&mut cx, &docs, " three");
    settle(&cx, |app| review.read(app).changes.len() == 3);
    let counts = cx.update(|app| authors::authors(&review.read(app).changes));
    assert_eq!(counts.iter().map(|(name, n)| (name.as_str(), *n)).collect::<Vec<_>>(), [("Ann", 2), ("Zoe", 1)]);
    // Zoe's only: the arrows stay on her one change.
    cx.update(|app| review.update(app, |review, _| review.who = Some("Zoe".into())));
    let zoe = |app: &gpui::App| {
        let review = review.read(app);
        let caret = review.shown.as_ref().expect("a tab").view.read(app).cursor();
        let change = review.changes.iter().find(|change| change.author == "Zoe").expect("her change");
        caret.is_some_and(|caret| nav::holds(&change.ranges, caret))
    };
    for forward in [true, true, false] {
        cx.update(|app| review.update(app, |review, cx| review.step_change(forward, cx)));
        settle(&cx, zoe);
    }
    // Her change decided, the filter ends.
    let index = cx.update(|app| {
        let review = review.read(app);
        review.changes.iter().find(|change| change.author == "Zoe").expect("her change").index.to_string()
    });
    cx.update(|app| review.update(app, |review, cx| review.run("track.accept", &[&index], cx)));
    settle(&cx, |app| review.read(app).changes.len() == 2 && review.read(app).who.is_none());
}

#[test]
fn the_change_card_opens_on_a_move_not_on_typing_and_decides_one_change() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    let id = blank(&mut cx, &docs);
    // The panel stays hidden: the card needs no sidebar.
    let review = cx.update(|app| docs.read(app).review.clone());
    cx.update(|app| docs.update(app, |docs, cx| docs.side_view(cx)));
    let track = |cx: &mut TestAppContext, on: &'static str| {
        with_window(cx, &docs, |docs, window, cx| docs.on_track(&TrackChanges, window, cx));
        settle(cx, |app| {
            let tab = docs.read(app).active_tab().expect("a tab");
            tab.view.read(app).state(".uno:TrackChanges") == Some(on)
        });
    };
    type_into(&mut cx, &docs, "Start");
    track(&mut cx, "true");
    type_into(&mut cx, &docs, " Alpha");
    type_into(&mut cx, &docs, " Beta");
    settle(&cx, |app| review.read(app).changes.len() == 1);
    // Typing inside one's own insertion opens nothing.
    assert_eq!(cx.update(|app| review.read(app).card.open), None, "typing opens no card");
    track(&mut cx, "false");
    type_into(&mut cx, &docs, " plain");
    let words = cx.update(|app| review.read(app).changes[0].words.clone());
    assert!(words.contains("Alpha"), "the change carries its words: {words}");
    // An arrow lands in it: its card opens.
    cx.update(|app| review.update(app, |review, cx| review.step_change(true, cx)));
    let index = cx.update(|app| review.read(app).changes[0].index);
    settle(&cx, |app| review.read(app).card.open == Some(index));
    // Escape closes it; it stays closed in place.
    assert!(cx.update(|app| review.update(app, |review, cx| review.close_card(cx))));
    assert!(!cx.update(|app| review.update(app, |review, cx| review.close_card(cx))), "nothing left to close");
    // Open again, then reject from the card: that change only.
    cx.update(|app| review.update(app, |review, cx| review.step_change(true, cx)));
    settle(&cx, |app| review.read(app).card.open == Some(index));
    cx.update(|app| review.update(app, |review, cx| {
        review.run("track.reject", &[&index.to_string()], cx);
        review.close_card(cx);
    }));
    settle(&cx, |app| review.read(app).changes.is_empty() && review.read(app).card.open.is_none());
    let path = dir.path().join("card.odt");
    with_window(&mut cx, &docs, |docs, window, cx| docs.write_as(id, path.clone(), None, window, cx));
    written(&cx, &path, None);
    let text = text_of(&mut cx, &docs, &path);
    assert!(text.contains("Start") && text.contains("plain"), "the rest stays");
    assert!(!text.contains("Alpha"), "the rejected words go");
}
