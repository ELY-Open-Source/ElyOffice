use super::input::{FODS, open, paragraphs, saved};
use super::*;
use crate::commands;

/// How long a command may take to report.
const SETTLE: std::time::Duration = std::time::Duration::from_secs(10);

/// The latest value the kernel reported for `uno`.
fn state<'a>(seen: &'a Events, uno: &str) -> Option<&'a str> {
    seen.states
        .iter()
        .rev()
        .find(|(name, _)| name == uno)
        .map(|(_, state)| state.as_str())
}

/// Runs `id` and waits for `uno` to report `want`.
fn run_until(
    worker: &mut Worker,
    doc: &Doc,
    from: &Events,
    (id, values): (&str, &[&str]),
    (uno, want): (&str, &str),
) -> Events {
    worker.run(doc, id, values).expect("the command runs");
    let seen = poll_until(worker, doc, from, SETTLE, |seen| {
        state(seen, uno) == Some(want)
    });
    assert_eq!(state(&seen, uno), Some(want), "{id}: {uno}");
    seen
}

#[test]
fn every_mapped_command_reaches_the_kernel_and_reports_state() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    assert_eq!(
        state(&base, ".uno:Bold"),
        Some("false"),
        "the kernel seeds state"
    );
    let typed = {
        worker.post_key(&doc, 'a' as u32, 512).expect("a posts");
        poll_until(&mut worker, &doc, &base, SETTLE, |seen| {
            state(seen, ".uno:Undo") == Some("enabled")
        })
    };
    assert_eq!(
        state(&typed, ".uno:Undo"),
        Some("enabled"),
        "typing enables undo"
    );
    assert_eq!(state(&typed, ".uno:ModifiedStatus"), Some("true"));
    let seen = run_until(
        &mut worker,
        &doc,
        &typed,
        ("edit.undo", &[]),
        (".uno:Redo", "enabled"),
    );
    let seen = run_until(
        &mut worker,
        &doc,
        &seen,
        ("edit.redo", &[]),
        (".uno:Undo", "enabled"),
    );
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    let seen = poll_until(&mut worker, &doc, &seen, SETTLE, |seen| {
        !seen.selection.is_empty()
    });
    assert!(!seen.selection.is_empty(), "select all selects");
    let mut seen = seen;
    for (id, uno) in [
        ("format.bold", ".uno:Bold"),
        ("format.italic", ".uno:Italic"),
        ("format.underline", ".uno:Underline"),
        ("format.strikeout", ".uno:Strikeout"),
    ] {
        seen = run_until(&mut worker, &doc, &seen, (id, &[]), (uno, "true"));
    }
    // A Western font on "Hello 世界": two fonts now.
    seen = run_until(
        &mut worker,
        &doc,
        &seen,
        ("format.font", &["Liberation Mono"]),
        (".uno:CharFontName", crate::MIXED),
    );
    seen = run_until(
        &mut worker,
        &doc,
        &seen,
        ("format.size", &["18"]),
        (".uno:FontHeight", "18"),
    );
    let styles = worker
        .values(&doc, ".uno:StyleApply")
        .expect("the style list");
    assert!(
        styles.contains("ParagraphStyles"),
        "the kernel lists styles"
    );
    run_until(
        &mut worker,
        &doc,
        &seen,
        ("format.style", &["Heading 1", "ParagraphStyles"]),
        (".uno:StyleApply", "Heading 1"),
    );
    let text = saved(&dir, &mut worker, &doc, "out.fodt");
    assert!(
        paragraphs(&text).is_empty(),
        "the paragraph became a heading"
    );
    let body = text
        .replace("<text:h", "<text:p")
        .replace("</text:h>", "</text:p>");
    assert_eq!(
        paragraphs(&body),
        ["aHello 世界"],
        "the heading holds the text"
    );
    assert!(text.contains("fo:font-weight=\"bold\""), "bold landed");
    assert_eq!(
        commands::all().count(),
        252,
        "the rows of text, layout, refs, tools, sheet, calc, data, analysis, page, show and draw"
    );
}

#[test]
fn unmapped_commands_and_lists_are_refused() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(FODT);
    assert!(
        worker.run(&doc, "format.dialog", &[]).is_err(),
        "unknown ids fail here"
    );
    assert!(
        worker.run(&doc, "format.size", &[]).is_err(),
        "missing values fail here"
    );
    let raw = |name: &str, value: &str| Pack::new().u64(doc.id).str(name).u64(1).str(value).bytes();
    assert!(
        worker
            .request(tag::POST_UNO, &raw(".uno:FontDialog", ""))
            .is_err(),
        "the worker refuses unmapped names"
    );
    assert!(
        worker
            .request(tag::POST_UNO, &raw("format.size", "junk"))
            .is_err(),
        "the worker checks values too"
    );
    assert!(
        worker.values(&doc, ".uno:Copy").is_err(),
        "only listed values read"
    );
}

#[test]
fn failed_commands_surface_and_change_nothing() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    let size = state(&base, ".uno:FontHeight")
        .expect("a seeded size")
        .to_string();
    assert!(
        worker.run(&doc, "edit.undo", &[]).is_err(),
        "nothing to undo is an error"
    );
    assert!(worker.run(&doc, "format.size", &["junk"]).is_err());
    assert!(
        worker
            .run(&doc, "format.font", &["Ely No Such Font"])
            .is_err()
    );
    let missing = worker.run(
        &doc,
        "format.style",
        &["Ely No Such Style", "ParagraphStyles"],
    );
    assert!(missing.is_err(), "a missing style is an error: {missing:?}");
    let seen = poll_until(&mut worker, &doc, &base, SETTLE, |_| false);
    assert_eq!(
        state(&seen, ".uno:FontHeight").unwrap_or(&size),
        size,
        "the size stays"
    );
    assert_ne!(
        state(&seen, ".uno:ModifiedStatus"),
        Some("true"),
        "nothing changed"
    );
    let text = saved(&dir, &mut worker, &doc, "out.fodt");
    assert_eq!(paragraphs(&text), ["Hello 世界"]);
}

#[test]
fn a_kernel_dialog_is_closed_and_reported() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let (done, wait) = std::sync::mpsc::channel();
    let (pid_out, pid_in) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let dir = tempfile::tempdir().expect("a temp dir");
        let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
        pid_out.send(worker.child.id()).expect("the test waits");
        let source = dir.path().join("in.fods");
        write(&source, FODS);
        let doc = worker
            .load(&crate::file_url(&source))
            .expect("the sheet loads");
        let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
        // Calc's Backspace opens Delete Contents.
        worker.post_key(&doc, 0, 1283).expect("backspace posts");
        let seen = poll_until(&mut worker, &doc, &base, SETTLE, |seen| {
            !seen.blocked.is_empty()
        });
        worker
            .post_key(&doc, 'z' as u32, 537)
            .expect("typing still works");
        done.send(seen.blocked).expect("the test waits");
    });
    let pid = pid_in.recv().expect("the worker starts");
    let outcome = wait.recv_timeout(std::time::Duration::from_secs(30));
    if outcome.is_err() {
        // A blocked worker would hold the test open.
        kill(pid);
    }
    let blocked = outcome.expect("the dialog never blocked the worker");
    assert_eq!(blocked, ["Delete Contents"], "the dialog is reported");
}

#[test]
fn a11y_follows_the_focused_paragraph() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(FODT);
    let opened = worker.poll_events(&doc, 0, 250).expect("the first poll");
    assert_eq!(
        opened.a11y.text, "Hello 世界",
        "the open seeds the paragraph"
    );
    assert_eq!(opened.a11y.caret, Some(0));
    click(&mut worker, &doc, 150, 150);
    worker.post_key(&doc, 'a' as u32, 512).expect("a posts");
    let seen = poll_until(&mut worker, &doc, &opened, SETTLE, |seen| {
        seen.a11y.text == "aHello 世界" && seen.a11y.caret == Some(1)
    });
    assert_eq!(seen.a11y.text, "aHello 世界", "the paragraph text bridges");
    assert_eq!(seen.a11y.caret, Some(1), "the caret bridges");
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    let all = poll_until(&mut worker, &doc, &seen, SETTLE, |seen| {
        seen.a11y.selection == Some((0, 9))
    });
    assert_eq!(all.a11y.selection, Some((0, 9)), "the selection bridges");
    worker
        .post_key(&doc, 0, 1027)
        .expect("Right drops the selection");
    let none = poll_until(&mut worker, &doc, &all, SETTLE, |seen| {
        seen.a11y.selection.is_none()
    });
    assert_eq!(none.a11y.selection, None, "-1 clears it");
}

#[test]
fn a_selected_image_reports_its_frame() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(200, 200, image::Rgba([255, 0, 0, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("the png encodes");
    worker
        .paste(&doc, "image/png", &png)
        .expect("the image pastes");
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    // The image sits as a character at the text start.
    click(&mut worker, &doc, 1418 + 1000, 1418 + 1500);
    let seen = poll_until(&mut worker, &doc, &base, SETTLE, |seen| {
        seen.graphic.is_some()
    });
    let (x, y, w, h) = seen.graphic.expect("the image frame arrives");
    assert!(
        (1300..1600).contains(&x) && (1300..1600).contains(&y),
        "{:?}",
        seen.graphic
    );
    assert!(w > 0 && h > 0, "the frame has a size");
}

#[test]
fn undo_and_redo_follow_the_live_history() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker.post_key(&doc, 'a' as u32, 512).expect("a posts");
    worker
        .run(&doc, "edit.undo", &[])
        .expect("undo right after typing");
    let error = worker.run(&doc, "edit.undo", &[]).expect_err("a second undo has nothing");
    assert!(crate::is_nothing(&error), "{error}");
    worker
        .run(&doc, "edit.redo", &[])
        .expect("redo right after undo");
    let error = worker.run(&doc, "edit.redo", &[]).expect_err("a second redo has nothing");
    assert!(crate::is_nothing(&error), "{error}");
    let text = saved(&dir, &mut worker, &doc, "out.fodt");
    assert_eq!(paragraphs(&text), ["aHello 世界"]);
}

#[test]
fn a_size_finer_than_the_kernel_keeps_is_refused() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    assert!(worker.run(&doc, "format.size", &["18.01"]).is_err());
    let raw = Pack::new()
        .u64(doc.id)
        .str("format.size")
        .u64(1)
        .str("18.01")
        .bytes();
    assert!(
        worker.request(tag::POST_UNO, &raw).is_err(),
        "the worker refuses it too"
    );
    worker
        .run(&doc, "format.size", &["18.05"])
        .expect("0.05 steps run");
    let text = saved(&dir, &mut worker, &doc, "out.fodt");
    let sizes: Vec<f64> = text
        .split("fo:font-size=\"")
        .skip(1)
        .filter_map(|rest| rest.split("pt").next()?.parse().ok())
        .collect();
    assert!(
        sizes.iter().any(|size| (size - 18.05).abs() < 0.001),
        "the kernel keeps 18.05: {sizes:?}"
    );
}

#[test]
fn a_refused_size_keeps_the_redo() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker.post_key(&doc, 'a' as u32, 512).expect("a posts");
    worker.run(&doc, "edit.undo", &[]).expect("undo");
    let raw = Pack::new()
        .u64(doc.id)
        .str("format.size")
        .u64(1)
        .str("18.05000001")
        .bytes();
    assert!(
        worker.request(tag::POST_UNO, &raw).is_err(),
        "the worker refuses it"
    );
    worker
        .run(&doc, "edit.redo", &[])
        .expect("the redo survives");
    let text = saved(&dir, &mut worker, &doc, "out.fodt");
    assert_eq!(paragraphs(&text), ["aHello 世界"]);
}

#[test]
fn an_owned_save_clears_the_modified_state() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker.post_key(&doc, 'a' as u32, 512).expect("a posts");
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |seen| {
        state(seen, ".uno:ModifiedStatus") == Some("true")
    });
    let copy = dir.path().join("copy.docx");
    doc.save_owned(&mut worker, &crate::file_url(&copy), "docx")
        .expect("the owned save");
    let saved = poll_until(&mut worker, &doc, &base, SETTLE, |seen| {
        state(seen, ".uno:ModifiedStatus") == Some("false")
    });
    assert_eq!(state(&saved, ".uno:ModifiedStatus"), Some("false"));
    assert!(
        std::fs::read(&copy).expect("the copy").starts_with(b"PK"),
        "a docx zip"
    );
    worker.post_key(&doc, 'b' as u32, 513).expect("b posts");
    let edited = poll_until(&mut worker, &doc, &saved, SETTLE, |seen| {
        state(seen, ".uno:ModifiedStatus") == Some("true")
    });
    assert_eq!(state(&edited, ".uno:ModifiedStatus"), Some("true"));
    let bad = Pack::new()
        .u64(doc.id)
        .str("file:///tmp/x.odt")
        .str("odt")
        .str("Evil")
        .bytes();
    assert!(
        worker.request(tag::SAVE_AS, &bad).is_err(),
        "unknown options are refused"
    );
}
