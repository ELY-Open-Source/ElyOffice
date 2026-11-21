//! Impress links the kernel opens itself run no macro (R20.51).

use std::time::Duration;

use super::macros::{MACRO_URL, module, seed_basic};
use super::*;

const MOD1: i32 = 0x2000;
const SCRIPT_URL: &str = "vnd.sun.star.script:Standard.Module1.Sneaky?language=Basic&amp;location=application";
const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAAEUlEQVR4nGNQSDiAFTEMLQkAGplQAT43V5gAAAAASUVORK5CYII=";

/// Web, macro and script text links; two image maps.
fn deck(basic: bool) -> String {
    let rows = [("TW", "https://example.com/w"), ("TM", MACRO_URL), ("TS", SCRIPT_URL)];
    let mut shapes = String::new();
    for (row, (name, href)) in rows.iter().enumerate() {
        let y = 0.5 + row as f64 * 1.8;
        shapes += &format!(
            r#"<draw:frame draw:name="{name}" svg:x="1cm" svg:y="{y}cm" svg:width="12cm" svg:height="1.5cm"><draw:text-box><text:p><text:a xlink:type="simple" xlink:href="{href}">{name} link</text:a></text:p></draw:text-box></draw:frame>"#
        );
    }
    for (row, href) in [MACRO_URL, SCRIPT_URL].iter().enumerate() {
        let y = 0.5 + row as f64 * 2.1;
        shapes += &format!(
            r#"<draw:frame svg:x="15cm" svg:y="{y}cm" svg:width="2cm" svg:height="2cm"><draw:image><office:binary-data>{PNG}</office:binary-data></draw:image><draw:image-map><draw:area-rectangle xlink:type="simple" xlink:href="{href}" svg:x="0" svg:y="0" svg:width="2cm" svg:height="2cm"/></draw:image-map></draw:frame>"#
        );
    }
    // A document library changes what Ely hears.
    let scripts = match basic {
        true => r#"<office:scripts><office:script script:language="ooo:Basic"><ooo:libraries><ooo:library-embedded ooo:name="Standard"><ooo:module ooo:name="Module1"><ooo:source-code>Sub Noop&#x0A;End Sub</ooo:source-code></ooo:module></ooo:library-embedded></ooo:libraries></office:script></office:scripts>"#,
        false => "",
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:presentation="urn:oasis:names:tc:opendocument:xmlns:presentation:1.0" xmlns:script="urn:oasis:names:tc:opendocument:xmlns:script:1.0" xmlns:ooo="http://openoffice.org/2004/office" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
{scripts}<office:automatic-styles><style:page-layout style:name="PM1"><style:page-layout-properties fo:page-width="28cm" fo:page-height="15.75cm"/></style:page-layout></office:automatic-styles>
<office:master-styles><style:master-page style:name="Default" style:page-layout-name="PM1"/></office:master-styles>
<office:body><office:presentation><draw:page draw:name="One" draw:master-page-name="Default">{shapes}</draw:page></office:presentation></office:body>
</office:document>"#
    )
}

fn press(worker: &mut Worker, doc: &Doc, (x, y): (f64, f64), modifiers: i32) {
    for kind in [0, 1] {
        let at = Mouse { kind, x: (x * 567.0) as i32, y: (y * 567.0) as i32, count: 1, buttons: 1, modifiers };
        worker.post_mouse(doc, at).expect("the click posts");
    }
}

/// Centimetres: a text link's row, an image map's row.
fn text(row: f64) -> (f64, f64) {
    (1.6, 0.95 + row * 1.8)
}

fn image(row: f64) -> (f64, f64) {
    (16.0, 1.5 + row * 2.1)
}

/// Twips: the cursor shows inside a frame at the point.
fn editing_at(seen: &Events, (x, y): (f64, f64)) -> bool {
    let inside = |(left, top, width, height): Rect, (x, y): (i32, i32)| {
        (left..=left + width).contains(&x) && (top..=top + height).contains(&y)
    };
    let point = ((x * 567.0) as i32, (y * 567.0) as i32);
    match (seen.graphic, seen.cursor) {
        (Some(frame), Some(cursor)) => seen.cursor_visible && inside(frame, point) && inside(frame, (cursor.0, cursor.1)),
        _ => false,
    }
}

/// Clicks into a box; the kernel must say it edits.
fn edit(worker: &mut Worker, doc: &Doc, seen: &Events, at: (f64, f64), what: &str) -> Events {
    let before = seen.seq;
    press(worker, doc, at, 0);
    let seen = poll_until(worker, doc, seen, Duration::from_secs(2), |seen| seen.seq > before && editing_at(seen, at));
    assert!(
        seen.seq > before && editing_at(&seen, at),
        "{what}: the box never started editing: cursor {:?} visible {} frame {:?}",
        seen.cursor,
        seen.cursor_visible,
        seen.graphic
    );
    seen
}

/// Image maps and clicks while editing run no macro.
#[test]
fn a_macro_link_the_kernel_opens_itself_never_runs() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    // Profile and marker stay in this process's test root.
    let dir = tempfile::tempdir_in(ely_platform::test_root()).expect("a folder in the test root");
    let (profile, marker) = (dir.path().join("profile"), dir.path().join("macro-ran.txt"));
    seed_basic(&install, &profile);
    std::fs::write(profile.join("user/basic/Standard/Module1.xba"), module(&marker)).expect("the module writes");
    let mut worker = Worker::spawn_dev(&install, &profile).expect("the worker");
    // Links the kernel follows without asking Ely.
    let clicks = [
        ("image map, macro", None, image(0.0), MOD1),
        ("image map, script", None, image(1.0), MOD1),
        ("another box while editing, macro", Some(0.0), text(1.0), 0),
        ("another box while editing, script", Some(0.0), text(2.0), MOD1),
    ];
    for basic in [true, false] {
        let source = dir.path().join(format!("links-{basic}.fodp"));
        std::fs::write(&source, deck(basic)).expect("the deck writes");
        for (what, editing, at, modifiers) in clicks {
            // macOS: an image map's question upsets later clicks.
            let doc = worker.load(&crate::file_url(&source)).expect("the deck loads");
            let mut seen = poll_until(&mut worker, &doc, &Events::default(), Duration::from_secs(2), |_| false);
            if let Some(row) = editing {
                seen = edit(&mut worker, &doc, &seen, text(row), what);
            }
            press(&mut worker, &doc, at, modifiers);
            // Without Basic, Ely hears a closed question.
            let asked = |seen: &Events| !basic && !seen.blocked.is_empty();
            seen = poll_until(&mut worker, &doc, &seen, Duration::from_secs(3), asked);
            let label = format!("{what}, document Basic {basic}");
            assert!(!marker.try_exists().expect("the marker check"), "{label}: the macro ran");
            assert!(seen.links.is_empty(), "{label}: the link callback came: {:?}", seen.links);
            match basic {
                // With Basic, Ely hears nothing at all.
                true => assert!(seen.blocked.is_empty(), "{label}: {:?}", seen.blocked),
                false => assert!(
                    matches!(&seen.blocked[..], [(_, body)] if body.contains("dangerous to open")),
                    "{label}: {:?}",
                    seen.blocked
                ),
            }
            worker.close_doc(doc).expect("the deck closes");
        }
    }
    // The control: the same macro runs when asked.
    assert!(worker.run_macro(MACRO_URL).expect("the macro channel") != 0, "the library macro runs");
    assert!(marker.try_exists().expect("the marker check"), "the sample is real: it writes when run");
}
