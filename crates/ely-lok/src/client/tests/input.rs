use super::*;

/// How long an input may take to show.
const SETTLE: std::time::Duration = std::time::Duration::from_secs(3);
/// vcl KEY_A, KEY_MOD1.
const KEY_A: i32 = 512;
const MOD1: i32 = 0x2000;

/// A flat document holding a 2x1 table.
const TABLE_FODT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><table:table table:name="T"><table:table-column table:number-columns-repeated="2"/>
 <table:table-row><table:table-cell><text:p>left</text:p></table:table-cell><table:table-cell><text:p>right</text:p></table:table-cell></table:table-row>
 </table:table><text:p>after</text:p></office:text></office:body>
</office:document>"#;

/// A worker with one loaded document.
pub(super) fn open(body: &str) -> (tempfile::TempDir, Worker, Doc) {
    let install = install().expect("the pinned LOK build");
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let source = dir.path().join("in.fodt");
    write(&source, body);
    let doc = worker
        .load(&crate::file_url(&source))
        .expect("the document loads");
    (dir, worker, doc)
}

/// The document saved as flat XML.
pub(super) fn saved(dir: &tempfile::TempDir, worker: &mut Worker, doc: &Doc, name: &str) -> String {
    let out = dir.path().join(name);
    doc.save_as(worker, &crate::file_url(&out), "fodt")
        .expect("the save works");
    std::fs::read_to_string(&out).expect("the save reads")
}

/// Each paragraph's text, tags stripped.
pub(crate) fn paragraphs(xml: &str) -> Vec<String> {
    xml.split("<text:p")
        .skip(1)
        .map(|rest| {
            let body = &rest[rest.find('>').map_or(0, |at| at + 1)..];
            let body = &body[..body.find("</text:p>").unwrap_or(0)];
            let mut text = String::new();
            let mut tag = false;
            for c in body.chars() {
                match c {
                    '<' => tag = true,
                    '>' => tag = false,
                    c if !tag => text.push(c),
                    _ => {}
                }
            }
            text
        })
        .collect()
}

/// A 4x4 red PNG.
pub(super) fn red_png() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("the png encodes");
    png
}

#[test]
fn an_ime_composition_commits_once() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    for step in ["n", "ni", "你"] {
        worker
            .post_text(&doc, false, step)
            .expect("the update posts");
    }
    worker.post_text(&doc, true, "").expect("the commit posts");
    let text = paragraphs(&saved(&dir, &mut worker, &doc, "out.fodt"));
    assert_eq!(text, ["你Hello 世界"], "the composition lands once");
}

#[test]
fn a_cancelled_composition_leaves_nothing() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker
        .post_text(&doc, false, "ni")
        .expect("the update posts");
    worker.post_text(&doc, false, "").expect("the cancel posts");
    worker.post_text(&doc, true, "").expect("the end posts");
    let text = paragraphs(&saved(&dir, &mut worker, &doc, "out.fodt"));
    assert_eq!(text, ["Hello 世界"], "the paragraph is untouched");
}

#[test]
fn keys_carry_chars_and_modifiers() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker.post_key(&doc, 'a' as u32, KEY_A).expect("a posts");
    let text = paragraphs(&saved(&dir, &mut worker, &doc, "out.fodt"));
    assert_eq!(text, ["aHello 世界"], "the char lands");
    let base = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |_| false);
    worker
        .post_key(&doc, 0, MOD1 | KEY_A)
        .expect("select all posts");
    let seen = poll_until(&mut worker, &doc, &base, SETTLE, |seen| {
        !seen.selection.is_empty()
    });
    assert!(!seen.selection.is_empty(), "mod1+A selects: {seen:?}");
}

#[test]
fn the_kernels_close_keys_are_refused() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    // Mod1+W, Mod1+F4, Mod1+Q close the frame or quit.
    for key in [534, 771, 528] {
        let refused = worker.post_key(&doc, 0, MOD1 | key).expect_err("refused");
        assert!(refused.to_string().contains("closes the kernel's frame"), "{refused}");
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
    let text = paragraphs(&saved(&dir, &mut worker, &doc, "out.fodt"));
    assert_eq!(text, ["Hello 世界"], "the document stays open");
}

#[test]
fn the_cursor_and_selection_arrive_in_twips() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(FODT);
    let (width, height) = doc.size(&mut worker).expect("the size");
    click(&mut worker, &doc, 150, 150);
    let seen = poll_until(&mut worker, &doc, &Events::default(), SETTLE, |seen| {
        seen.cursor.is_some()
    });
    let (x, y, w, h) = seen.cursor.expect("a cursor rect");
    assert!(
        x >= 0 && y >= 0 && x < width as i32 && y < height as i32 && w >= 0 && h > 0,
        "the cursor sits in the page: {:?}",
        seen.cursor
    );
    assert!(seen.cursor_visible, "the cursor shows");
    let down = |kind, x| Mouse {
        kind,
        x,
        y: y + h / 2,
        count: 1,
        buttons: 1,
        modifiers: 0,
    };
    worker.post_mouse(&doc, down(0, x)).expect("press");
    worker.post_mouse(&doc, down(2, x + 1500)).expect("drag");
    worker.post_mouse(&doc, down(1, x + 1500)).expect("release");
    let dragged = poll_until(&mut worker, &doc, &seen, SETTLE, |seen| {
        !seen.selection.is_empty()
    });
    assert!(!dragged.selection.is_empty(), "a drag selects: {dragged:?}");
}

#[test]
fn copy_gives_text_and_html_and_paste_takes_html() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    assert!(
        worker.copy(&doc).expect("copy runs").is_empty(),
        "nothing selected, nothing copied"
    );
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    let flavors = worker.copy(&doc).expect("copy runs");
    let get = |mime: &str| {
        flavors
            .iter()
            .find(|(name, _)| name == mime)
            .map(|(_, data)| String::from_utf8_lossy(data).into_owned())
    };
    assert_eq!(
        get("text/plain;charset=utf-8").as_deref(),
        Some("Hello 世界")
    );
    let html = get("text/html").expect("an html flavor");
    assert!(html.contains("Hello"), "the html holds the text: {html}");
    assert!(!html.contains('\0'), "no NUL rides along");
    let target = dir.path().join("b.fodt");
    write(&target, FODT);
    let other = worker
        .load(&crate::file_url(&target))
        .expect("b loads");
    click(&mut worker, &other, 150, 150);
    worker
        .paste(&other, "text/html", html.as_bytes())
        .expect("the html pastes");
    let text = saved(&dir, &mut worker, &other, "b-out.fodt");
    assert_eq!(
        text.matches("Hello").count(),
        2,
        "b gained the text: {text}"
    );
}

#[test]
fn a_cut_after_copy_removes_the_selection() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    assert!(
        !worker.copy(&doc).expect("copy runs").is_empty(),
        "the copy has flavors"
    );
    worker.cut(&doc).expect("the cut runs");
    let text = paragraphs(&saved(&dir, &mut worker, &doc, "out.fodt"));
    assert_eq!(text, [""], "the selection is gone");
}

#[test]
fn a_paste_the_engine_drops_is_an_error() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    let broken = worker.paste(&doc, "image/png", b"this is not a png");
    assert!(broken.is_err(), "a corrupt image is refused: {broken:?}");
    assert!(
        worker.paste(&doc, "text/html", b"").is_err(),
        "an empty paste is refused"
    );
    let text = saved(&dir, &mut worker, &doc, "out.fodt");
    assert_eq!(paragraphs(&text), ["Hello 世界"]);
    assert!(!text.contains("<draw:image"));
}

#[test]
fn a_corrupt_image_over_a_selection_keeps_the_text() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    let mut bad = red_png();
    bad.truncate(bad.len() / 2);
    assert!(
        worker.paste(&doc, "image/png", &bad).is_err(),
        "a cut-off png is refused"
    );
    assert!(
        !worker.copy(&doc).expect("copy runs").is_empty(),
        "the selection stays"
    );
    let text = saved(&dir, &mut worker, &doc, "out.fodt");
    assert_eq!(paragraphs(&text), ["Hello 世界"], "the text stays");
}

/// A spreadsheet whose A1 holds text.
pub(crate) const FODS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S"><table:table-row><table:table-cell><text:p>Hello 世界</text:p></table:table-cell></table:table-row></table:table></office:spreadsheet></office:body>
</office:document>"#;

#[test]
fn a_calc_cut_finishes_without_a_dialog() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let (done, wait) = std::sync::mpsc::channel();
    let (pid_out, pid_in) = std::sync::mpsc::channel();
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path().to_path_buf();
    std::thread::spawn(move || {
        let mut worker = Worker::spawn(&install, &root.join("profile")).expect("the worker");
        pid_out.send(worker.child.id()).expect("the test waits");
        let source = root.join("in.fods");
        write(&source, FODS);
        let doc = worker
            .load(&crate::file_url(&source))
            .expect("the sheet loads");
        worker
            .post_key(&doc, 0, 1027 | 0x1000)
            .expect("shift+right selects");
        let copied = worker.copy(&doc).expect("copy runs");
        worker.cut(&doc).expect("the cut runs");
        let out = root.join("out.fods");
        doc.save_as(&mut worker, &crate::file_url(&out), "fods")
            .expect("the save works");
        let xml = std::fs::read_to_string(&out).expect("the save reads");
        done.send((copied.len(), xml.contains("Hello")))
            .expect("the test waits");
    });
    let pid = pid_in.recv().expect("the worker starts");
    let outcome = wait.recv_timeout(std::time::Duration::from_secs(20));
    if outcome.is_err() {
        // A blocked worker would hold the test open.
        kill(pid);
    }
    let (flavors, kept) = outcome.expect("the cut never blocked on a dialog");
    assert!(flavors > 0, "the cells copied");
    assert!(!kept, "the cut emptied A1");
}

#[test]
fn a_table_survives_copy_and_paste() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(TABLE_FODT);
    click(&mut worker, &doc, 150, 150);
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select the whole text");
    let flavors = worker.copy(&doc).expect("copy runs");
    let (_, html) = flavors
        .iter()
        .find(|(mime, _)| mime == "text/html")
        .expect("an html flavor");
    let html = String::from_utf8_lossy(html);
    assert!(
        html.contains("<table"),
        "the html carries the table: {html}"
    );
    let target = dir.path().join("b.fodt");
    write(&target, FODT);
    let other = worker
        .load(&crate::file_url(&target))
        .expect("b loads");
    click(&mut worker, &other, 150, 150);
    worker
        .paste(&other, "text/html", html.as_bytes())
        .expect("the table pastes");
    let text = saved(&dir, &mut worker, &other, "b-out.fodt");
    assert!(text.contains("<table:table "), "b gained a table: {text}");
    assert!(text.contains("left") && text.contains("right"));
}

#[test]
fn an_image_pastes_and_copies_back() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker
        .paste(&doc, "image/png", &red_png())
        .expect("the image pastes");
    let text = saved(&dir, &mut worker, &doc, "out.fodt");
    assert!(text.contains("<draw:image"), "the image lands: {text}");
    let flavors = worker.copy(&doc).expect("copy runs");
    let png = flavors
        .iter()
        .find(|(mime, _)| mime == "image/png")
        .map(|(_, data)| data.clone())
        .expect("the selected image copies as png");
    let back = image::load_from_memory(&png)
        .expect("the png decodes")
        .to_rgba8();
    assert_eq!(back.get_pixel(0, 0).0, [255, 0, 0, 255], "red comes back");
}

#[test]
fn a_foreign_paste_flavor_is_refused() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(FODT);
    assert!(worker.paste(&doc, "application/x-evil", b"x").is_err());
    worker.post_key(&doc, 0, 1027).expect("the worker lives");
}

/// HTML carrying `png` as an inline image.
fn inline_html(png: &[u8]) -> String {
    use base64::Engine;
    let data = base64::engine::general_purpose::STANDARD.encode(png);
    format!(r#"<html><body><img src="data:image/png;base64,{data}"></body></html>"#)
}

#[test]
fn inline_images_paste_only_when_they_decode() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    let broken = inline_html(b"this is not a png");
    assert!(worker.paste(&doc, "text/html", broken.as_bytes()).is_err());
    assert!(
        !worker.copy(&doc).expect("copy runs").is_empty(),
        "the selection stays"
    );
    let text = saved(&dir, &mut worker, &doc, "kept.fodt");
    assert_eq!(paragraphs(&text), ["Hello 世界"], "the text stays");
    worker
        .post_key(&doc, 0, 1029)
        .expect("End drops the selection");
    worker
        .paste(&doc, "text/html", inline_html(&red_png()).as_bytes())
        .expect("a real inline image pastes");
    let text = saved(&dir, &mut worker, &doc, "image.fodt");
    assert_eq!(text.matches("<draw:image").count(), 1, "the image lands");
}

#[test]
fn pastes_keep_working_past_the_undo_cap() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    for n in 1..=105 {
        worker
            .paste(
                &doc,
                "text/plain;charset=utf-8",
                format!("p{n} ").as_bytes(),
            )
            .unwrap_or_else(|error| panic!("paste {n} failed: {error}"));
    }
    let text = paragraphs(&saved(&dir, &mut worker, &doc, "out.fodt"));
    assert!(
        text[0].contains("p105 ") && text[0].contains("p1 "),
        "every paste landed"
    );
}
