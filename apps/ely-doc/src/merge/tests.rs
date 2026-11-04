use gpui::{Entity, TestAppContext};

use super::*;
use crate::tests::{
    blank, commit, docs, settle, settled, text_of, type_into, with_window, written,
};

fn tabs(app: &gpui::App, docs: &Entity<Docs>) -> usize {
    docs.read(app).core.tabs.len()
}

#[test]
fn records_fill_previews_and_files() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let source_id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Dear ");
    let csv = dir.path().join("people.csv");
    std::fs::write(&csv, "Name,City\nAda,London\n«City»,上海\n").expect("the csv writes");
    let table = read_csv(&csv).expect("the csv reads");
    cx.update(|app| docs.update(app, |docs, cx| docs.field_here("Name", cx)));
    type_into(&mut cx, &docs, " of ");
    cx.update(|app| docs.update(app, |docs, cx| docs.field_here("City", cx)));
    let source = cx.update(|app| docs.read(app).core.tabs[0].name.to_string());
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            docs.preview_record(source_id, &table, 1, cx)
        })
    });
    let name = cx.update(|app| {
        Docs::text(
            app,
            "doc.merge.preview_name",
            &[("name", &source), ("n", "2")],
        )
    });
    settle(&cx, |app| {
        tabs(app, &docs) == 2
            && docs.read(app).core.tabs[1].name == name
            && docs.read(app).core.tabs[1].view.read(app).settled()
    });
    let preview = dir.path().join("preview.odt");
    let id = cx.update(|app| docs.read(app).core.tabs[1].id);
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, preview.clone(), None, window, cx)
    });
    written(&cx, &preview, None);
    assert!(
        text_of(&mut cx, &docs, &preview).contains("Dear «City» of 上海"),
        "the preview takes queued input and keeps values literal"
    );
    cx.update(|app| docs.update(app, |docs, _| docs.core.set_active(Some(docs.core.tabs[0].id))));
    let out = dir.path().join("out");
    std::fs::create_dir(&out).expect("the folder");
    let target = |n: usize, extension: &str| out.join(format!("{source} {n}.{extension}"));
    std::fs::write(target(2, "docx"), "mine").expect("a taken name");
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            docs.generate(source_id, table.clone(), false, out.clone(), cx)
        })
    });
    assert_eq!(
        std::fs::read_to_string(target(2, "docx")).expect("reads"),
        "mine",
        "a taken name stops the merge"
    );
    assert!(!target(1, "docx").exists(), "nothing is written");
    std::fs::remove_file(target(2, "docx")).expect("frees the name");
    let staged = cx.update(|app| {
        let docs = docs.read(app);
        docs.core
            .work
            .join(format!("doc-{}", docs.core.next))
            .join("record.pdf")
    });
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            docs.generate(source_id, table.clone(), true, out.clone(), cx)
        })
    });
    std::fs::write(target(1, "pdf"), "EXTERNAL").expect("a file arrives late");
    let worker = cx.update(|app| docs.read(app).core.worker.clone());
    settle(&cx, |_| staged.exists() && worker.try_lock().is_ok());
    assert_eq!(
        std::fs::read_to_string(target(1, "pdf")).expect("reads"),
        "EXTERNAL",
        "a file that arrives late stays"
    );
    assert!(
        !target(2, "pdf").exists(),
        "the merge stops at the taken name"
    );
    std::fs::remove_file(target(1, "pdf")).expect("frees the name");
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            docs.generate(source_id, table, true, out.clone(), cx)
        })
    });
    settle(&cx, |_| target(2, "pdf").exists());
    let first = std::fs::read(target(1, "pdf")).expect("the pdf reads");
    assert!(first.starts_with(b"%PDF"), "a PDF is written");
}

#[test]
fn compare_opens_a_marked_copy_and_keeps_both_files() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "A new line");
    settled(&cx, &docs);
    let older = dir.path().join("older.odt");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, older.clone(), None, window, cx)
    });
    written(&cx, &older, None);
    commit(&mut cx, &docs, " grows");
    let before = std::fs::read(&older).expect("the older file reads");
    let source = cx.update(|app| docs.read(app).core.tabs[0].name.to_string());
    cx.update(|app| docs.update(app, |docs, cx| docs.compare(id, older.clone(), cx)));
    settle(&cx, |app| {
        tabs(app, &docs) == 2 && docs.read(app).side == crate::docs::Side::Review
    });
    let review = cx.update(|app| docs.read(app).review.clone());
    let tab = cx.update(|app| docs.read(app).core.tabs[1].view.clone());
    cx.update(|app| {
        let doc = docs.read(app).core.tabs[1].doc.clone();
        review.update(app, |review, cx| review.show(Some((&tab, &doc)), cx))
    });
    settle(&cx, |app| !review.read(app).changes.is_empty());
    let name = cx.update(|app| docs.read(app).core.tabs[1].name.to_string());
    assert!(name.contains(&source), "the copy names its source: {name}");
    let copy = dir.path().join("copy.odt");
    let into = cx.update(|app| docs.read(app).core.tabs[1].id);
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(into, copy.clone(), None, window, cx)
    });
    written(&cx, &copy, None);
    assert!(
        text_of(&mut cx, &docs, &copy).contains(" grows"),
        "the copy takes the queued input"
    );
    assert_eq!(
        std::fs::read(&older).expect("reads"),
        before,
        "the older file stays"
    );
}

#[test]
fn a_merge_keeps_its_source_when_the_tab_changes() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let other = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "SourceB ");
    let source = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "SourceA ");
    let csv = dir.path().join("one.csv");
    std::fs::write(&csv, "Name\nAda\n").expect("the csv writes");
    let table = read_csv(&csv).expect("the csv reads");
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            docs.field_here("Name", cx);
            docs.preview_record(source, &table, 0, cx);
            docs.core.set_active(Some(other));
        })
    });
    let name = cx.update(|app| {
        let source = docs.read(app).tab(source).expect("a tab").name.clone();
        Docs::text(
            app,
            "doc.merge.preview_name",
            &[("name", &source), ("n", "1")],
        )
    });
    settle(&cx, |app| {
        let docs = docs.read(app);
        // A filled preview is a modified copy.
        docs.core.tabs.len() == 3 && docs.core.tabs[2].name == name && docs.core.tabs[2].modified
    });
    let preview = dir.path().join("preview.odt");
    let id = cx.update(|app| docs.read(app).core.tabs[2].id);
    // The preview is active now; the template stays.
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            docs.core.set_active(Some(id));
            assert_eq!(docs.source(cx), Some(source), "the template stays after a preview");
            assert!(docs.merge.read(cx).previews.contains(&id), "the preview is known");
        })
    });
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, preview.clone(), None, window, cx)
    });
    written(&cx, &preview, None);
    let text = text_of(&mut cx, &docs, &preview);
    assert!(text.contains("SourceA Ada"), "the preview keeps its source");
    let stem = cx.update(|app| docs.read(app).tab(source).expect("a tab").name.to_string());
    let out = dir.path().join("out");
    std::fs::create_dir(&out).expect("the folder");
    cx.update(|app| {
        docs.update(app, |docs, cx| {
            docs.core.set_active(Some(other));
            docs.generate(source, table, false, out.clone(), cx)
        })
    });
    let target = out.join(format!("{stem} 1.docx"));
    let worker = cx.update(|app| docs.read(app).core.worker.clone());
    settle(&cx, |_| target.exists() && worker.try_lock().is_ok());
    assert!(
        text_of(&mut cx, &docs, &target).contains("SourceA Ada"),
        "the file keeps its source"
    );
}

#[test]
fn two_generations_keep_their_own_templates() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let mut sources = Vec::new();
    for text in ["SourceA ", "SourceB "] {
        sources.push(blank(&mut cx, &docs));
        type_into(&mut cx, &docs, text);
        cx.update(|app| docs.update(app, |docs, cx| docs.field_here("Name", cx)));
        settled(&cx, &docs);
    }
    let csv = dir.path().join("one.csv");
    std::fs::write(&csv, "Name\nAda\n").expect("the csv writes");
    let table = read_csv(&csv).expect("the csv reads");
    let outs: Vec<_> = (0..2).map(|n| dir.path().join(format!("out{n}"))).collect();
    for (source, out) in sources.iter().zip(&outs) {
        std::fs::create_dir(out).expect("the folder");
        cx.update(|app| {
            docs.update(app, |docs, cx| {
                docs.generate(*source, table.clone(), false, out.clone(), cx)
            })
        });
    }
    let names: Vec<String> = cx.update(|app| {
        let docs = docs.read(app);
        sources
            .iter()
            .map(|id| docs.tab(*id).expect("a tab").name.to_string())
            .collect()
    });
    let targets: Vec<_> = outs
        .iter()
        .zip(&names)
        .map(|(out, name)| out.join(format!("{name} 1.docx")))
        .collect();
    let worker = cx.update(|app| docs.read(app).core.worker.clone());
    settle(&cx, |_| {
        targets.iter().all(|target| target.exists()) && worker.try_lock().is_ok()
    });
    for (target, want) in targets.iter().zip(["SourceA Ada", "SourceB Ada"]) {
        assert!(
            text_of(&mut cx, &docs, target).contains(want),
            "{want} lands"
        );
    }
}

#[test]
fn the_template_chosen_with_the_data_stays_through_a_preview() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let template = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Dear ");
    let csv = dir.path().join("two.csv");
    std::fs::write(&csv, "Name\nAda\nGrace\n").expect("the csv writes");
    cx.update(|app| docs.update(app, |docs, cx| docs.choose_csv(cx)));
    cx.simulate_path_prompt_response(move |_| Some(vec![csv]));
    settle(&cx, |app| docs.read(app).merge.read(app).table.is_some());
    cx.update(|app| docs.update(app, |docs, cx| docs.insert_field("Name", cx)));
    let table = cx.update(|app| docs.read(app).merge.read(app).table.clone().expect("the data"));
    // As the panel's buttons do: preview, then generate.
    cx.update(|app| docs.update(app, |docs, cx| {
        let source = docs.source(cx).expect("the template");
        docs.preview_record(source, &table, 0, cx);
    }));
    settle(&cx, |app| tabs(app, &docs) == 2 && docs.read(app).core.active() != Some(template));
    let out = dir.path().join("out");
    std::fs::create_dir(&out).expect("the folder");
    cx.update(|app| docs.update(app, |docs, cx| {
        let source = docs.source(cx).expect("the template, not the preview");
        assert_eq!(source, template);
        docs.generate(source, table.clone(), false, out.clone(), cx);
    }));
    let stem = cx.update(|app| docs.read(app).tab(template).expect("a tab").name.to_string());
    let target = |n: usize| out.join(format!("{stem} {n}.docx"));
    settle(&cx, |_| target(2).exists());
    let (first, second) = (text_of(&mut cx, &docs, &target(1)), text_of(&mut cx, &docs, &target(2)));
    assert!(first.contains("Dear Ada") && second.contains("Dear Grace"), "{first:?} {second:?}");
}
