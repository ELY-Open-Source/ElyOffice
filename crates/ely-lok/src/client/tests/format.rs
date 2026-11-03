use super::input::{open, saved};
use super::*;

/// How long a command may take to report.
const SETTLE: std::time::Duration = std::time::Duration::from_secs(3);

#[test]
fn text_and_paragraph_formatting_land_in_the_file() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    for (id, values) in [
        ("format.superscript", vec![]),
        ("format.color", vec!["16711680"]),
        ("format.highlight", vec!["65280"]),
        ("paragraph.center", vec![]),
        ("paragraph.spacing_15", vec![]),
        ("paragraph.indent", vec![]),
        ("list.numbering", vec![]),
    ] {
        worker
            .run(&doc, id, &values)
            .unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    let text = saved(&dir, &mut worker, &doc, "out.fodt");
    for mark in [
        "style:text-position=\"super",
        "fo:color=\"#ff0000\"",
        "fo:background-color=\"#00ff00\"",
        "fo:text-align=\"center\"",
        "fo:line-height=\"150%\"",
        "<text:list",
    ] {
        assert!(text.contains(mark), "{mark} lands");
    }
}

#[test]
fn tab_stops_add_and_remove_through_the_ruler() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    let state = worker.now(&doc).expect("the state");
    worker
        .run(&state, "paragraph.tab", &["-1", "1134", "false"])
        .expect("add a 2 cm stop");
    let added = poll_until(&mut worker, &doc, &base, HANG, |seen| {
        seen.tabs.len() == 1
    });
    assert_eq!(added.tabs.len(), 1, "the ruler shows one stop");
    assert!(
        (added.tabs[0] - 2000).abs() < 10,
        "2 cm in 1/100 mm: {:?}",
        added.tabs
    );
    let state = worker.now(&doc).expect("the state");
    worker
        .run(&state, "paragraph.tab", &["0", "0", "true"])
        .expect("remove it");
    let removed = poll_until(&mut worker, &doc, &added, HANG, |seen| {
        seen.tabs.is_empty()
    });
    assert!(removed.tabs.is_empty(), "the stop is gone");
}

#[test]
fn styles_are_created_and_updated_from_the_selection() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker.run(&doc, "format.bold", &[]).expect("bold");
    worker
        .run(&doc, "style.new", &["Ely Notes", "2"])
        .expect("a paragraph style");
    let styles = worker
        .values(&doc, ".uno:StyleApply")
        .expect("the style list");
    assert!(
        styles.contains("\"Ely Notes\""),
        "the kernel lists the new style"
    );
    worker.run(&doc, "format.italic", &[]).expect("italic");
    worker
        .run(&doc, "style.update", &["Ely Notes", "2"])
        .expect("the style updates");
    worker
        .run(&doc, "style.new", &["Ely Mark", "1"])
        .expect("a character style");
    assert!(
        worker.run(&doc, "style.new", &["", "2"]).is_err(),
        "a style needs a name"
    );
    let text = saved(&dir, &mut worker, &doc, "out.fodt");
    assert!(
        text.contains("style:display-name=\"Ely Notes\"")
            || text.contains("style:name=\"Ely Notes\"")
    );
    assert!(text.contains("Ely Mark"), "the character style lands");
}

#[test]
fn the_remaining_rows_reach_the_kernel() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    for id in [
        "format.subscript",
        "paragraph.right",
        "paragraph.justify",
        "paragraph.left",
        "paragraph.indent",
        "paragraph.outdent",
        "paragraph.spacing_2",
        "paragraph.spacing_1",
        "list.bullets",
        "list.deeper",
        "list.higher",
    ] {
        worker
            .run(&doc, id, &[])
            .unwrap_or_else(|error| panic!("{id}: {error}"));
    }
}

#[test]
fn list_levels_go_deeper_and_back() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker.run(&doc, "list.numbering", &[]).expect("numbering");
    worker
        .run(&doc, "list.deeper", &[])
        .expect("one level deeper");
    let deeper = saved(&dir, &mut worker, &doc, "deeper.fodt");
    assert_eq!(
        deeper.matches("<text:list-item").count(),
        2,
        "level 2 nests"
    );
    worker.run(&doc, "list.higher", &[]).expect("back up");
    let higher = saved(&dir, &mut worker, &doc, "higher.fodt");
    assert_eq!(
        higher.matches("<text:list-item").count(),
        1,
        "level 1 again"
    );
}

#[test]
fn spacing_above_and_below_set_apart() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker
        .run(&doc, "paragraph.spacing", &["0", "212"])
        .expect("6 pt below");
    let below = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        state(seen, ".uno:ULSpacing") == Some("0,212")
    });
    worker
        .run(&doc, "paragraph.spacing", &["423", "212"])
        .expect("12 pt above");
    poll_until(&mut worker, &doc, &below, HANG, |seen| {
        state(seen, ".uno:ULSpacing") == Some("423,212")
    });
    let text = saved(&dir, &mut worker, &doc, "spacing.fodt");
    assert!(
        text.contains("fo:margin-top=\"0.1665in\" fo:margin-bottom=\"0.0835in\""),
        "above changed, below kept"
    );
}

#[test]
fn the_character_style_is_reported() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    worker
        .run(&doc, "format.style", &["Emphasis", "CharacterStyles"])
        .expect("emphasis");
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        state(seen, ".uno:CharStyle") == Some("Emphasis")
    });
    assert_eq!(state(&seen, ".uno:CharStyle"), Some("Emphasis"));
}

/// The latest value of `uno` in a batch.
fn state<'a>(seen: &'a Events, uno: &str) -> Option<&'a str> {
    seen.states
        .iter()
        .rev()
        .find(|(name, _)| name == uno)
        .map(|(_, value)| value.as_str())
}

#[test]
fn mixed_selections_report_dont_care() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    for key in "One".chars() {
        worker.post_key(&doc, key as u32, 0).expect("a key");
    }
    worker
        .run(&doc, "format.style", &["Heading 1", "ParagraphStyles"])
        .expect("a heading");
    worker.post_key(&doc, 0, 1029).expect("end");
    worker.post_key(&doc, 13, 1280).expect("enter");
    for key in "Two".chars() {
        worker.post_key(&doc, key as u32, 0).expect("a key");
    }
    worker
        .post_key(&doc, 0, 1028 | 0x1000)
        .expect("select the word");
    worker
        .run(&doc, "format.style", &["Emphasis", "CharacterStyles"])
        .expect("emphasis");
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    let names = [
        ".uno:Italic",
        ".uno:CharStyle",
        ".uno:StyleApply",
        ".uno:ULSpacing",
    ];
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        names
            .iter()
            .all(|name| state(seen, name) == Some(crate::MIXED))
    });
    for name in names {
        assert_eq!(state(&seen, name), Some(crate::MIXED), "{name} is mixed");
    }
}

#[test]
fn character_styles_mix_their_attributes() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    for key in "Alpha Beta".chars() {
        worker.post_key(&doc, key as u32, 0).expect("a key");
    }
    worker.post_key(&doc, 0, 1028).expect("home");
    for _ in 0..5 {
        worker.post_key(&doc, 0, 1027 | 0x1000).expect("select");
    }
    worker
        .run(&doc, "format.style", &["Source Text", "CharacterStyles"])
        .expect("a monospace style");
    worker.post_key(&doc, 0, 1029).expect("end");
    for _ in 0..4 {
        worker.post_key(&doc, 0, 1026 | 0x1000).expect("select");
    }
    worker
        .run(
            &doc,
            "format.style",
            &["Strong Emphasis", "CharacterStyles"],
        )
        .expect("a bold style");
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    let names = [".uno:CharFontName", ".uno:Bold", ".uno:CharStyle"];
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        names
            .iter()
            .all(|name| state(seen, name) == Some(crate::MIXED))
    });
    for name in names {
        assert_eq!(state(&seen, name), Some(crate::MIXED), "{name} is mixed");
    }
}
