//! Mixed script, spacing and lists report mixed (patch 0135).

use super::super::input::open;
use super::super::*;
use super::state;

const HEAD: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:automatic-styles>
  <style:style style:name="Up" style:family="text"><style:text-properties style:text-position="super 58%"/></style:style>
  <style:style style:name="Down" style:family="text"><style:text-properties style:text-position="sub 58%"/></style:style>
  <style:style style:name="One" style:family="paragraph"><style:paragraph-properties fo:line-height="100%"/></style:style>
  <style:style style:name="Half" style:family="paragraph"><style:paragraph-properties fo:line-height="150%"/></style:style>
  <text:list-style style:name="Dots"><text:list-level-style-bullet text:level="1" text:bullet-char="•"/></text:list-style>
  <text:list-style style:name="Count"><text:list-level-style-number text:level="1" style:num-format="1"/></text:list-style>
 </office:automatic-styles>
 <office:body><office:text>"##;

const TAIL: &str = "</office:text></office:body></office:document>";

/// `uno` with all text selected.
fn selected(body: &str, uno: &str) -> Option<String> {
    let (_dir, mut worker, doc) = open(&format!("{HEAD}{body}{TAIL}"));
    worker.run(&doc, "edit.select_all", &[]).expect("select all");
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| state(seen, uno).is_some());
    state(&seen, uno).map(str::to_string)
}

fn mixed() -> Option<String> {
    Some(crate::MIXED.to_string())
}

#[test]
fn half_raised_text_reads_mixed_and_all_raised_reads_on() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let up = r#"<text:p>Alpha<text:span text:style-name="Up">Beta</text:span></text:p>"#;
    assert_eq!(selected(up, ".uno:SuperScript"), mixed());
    let down = r#"<text:p>Alpha<text:span text:style-name="Down">Beta</text:span></text:p>"#;
    assert_eq!(selected(down, ".uno:SubScript"), mixed());
    let all = r#"<text:p><text:span text:style-name="Up">Alpha Beta</text:span></text:p>"#;
    assert_eq!(selected(all, ".uno:SuperScript").as_deref(), Some("true"));
}

#[test]
fn two_spacings_read_mixed_and_one_reads_its_preset() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let two = r#"<text:p text:style-name="One">Alpha</text:p><text:p text:style-name="Half">Beta</text:p>"#;
    assert_eq!(selected(two, ".uno:SpacePara1"), mixed());
    assert_eq!(selected(two, ".uno:SpacePara15"), mixed());
    let one = r#"<text:p text:style-name="Half">Alpha</text:p><text:p text:style-name="Half">Beta</text:p>"#;
    assert_eq!(selected(one, ".uno:SpacePara15").as_deref(), Some("true"));
}

#[test]
fn a_list_over_some_paragraphs_reads_mixed() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let listed = |style: &str, rest: &str| {
        format!(r#"<text:list text:style-name="{style}"><text:list-item><text:p>Alpha</text:p></text:list-item></text:list>{rest}"#)
    };
    let plain = "<text:p>Beta</text:p>";
    assert_eq!(selected(&listed("Dots", plain), ".uno:DefaultBullet"), mixed());
    assert_eq!(selected(&listed("Count", plain), ".uno:DefaultNumbering"), mixed());
    assert_eq!(selected(&listed("Dots", ""), ".uno:DefaultBullet").as_deref(), Some("true"));
    assert_eq!(selected("<text:p>Alpha</text:p><text:p>Beta</text:p>", ".uno:DefaultBullet").as_deref(), Some("false"));
}

/// #112: 10pt then 10.5pt, Asian sizes optional.
fn two_sizes(asian: bool) -> String {
    let size = |pt: &str| {
        let east = if asian { format!(r#" style:font-size-asian="{pt}pt""#) } else { String::new() };
        format!(r#"<style:text-properties fo:font-size="{pt}pt"{east}/>"#)
    };
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:automatic-styles><style:style style:name="Ten" style:family="text">{}</style:style><style:style style:name="More" style:family="text">{}</style:style></office:automatic-styles>
 <office:body><office:text><text:p><text:span text:style-name="Ten">这部分是10号字体</text:span> <text:span text:style-name="More">这部分是10.5号字体</text:span></text:p>{TAIL}"##,
        size("10"),
        size("10.5")
    )
}

/// Selects the line by keys or drag, either way.
fn select_line(worker: &mut Worker, doc: &Doc, line: Rect, drag: bool, backward: bool) {
    const SHIFT: i32 = 0x1000;
    const HOME: i32 = 1028;
    const END: i32 = 1029;
    if !drag {
        worker.post_key(doc, 0, if backward { END } else { HOME }).expect("a key");
        worker.post_key(doc, 0, if backward { HOME | SHIFT } else { END | SHIFT }).expect("a key");
        return;
    }
    let (left, right, y) = (line.0 + 2, line.0 + line.2 - 2, line.1 + line.3 / 2);
    let (from, to) = if backward { (right, left) } else { (left, right) };
    let mut mouse = |kind, x| {
        worker.post_mouse(doc, Mouse { kind, x, y, count: 1, buttons: 1, modifiers: 0 }).expect("a mouse event");
    };
    mouse(0, from);
    for step in 1..=8 {
        mouse(2, from + (to - from) * step / 8);
    }
    mouse(1, to);
}

#[test]
fn a_line_of_two_sizes_reads_mixed_from_either_end() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    for asian in [false, true] {
        for (drag, backward) in [(false, false), (false, true), (true, false), (true, true)] {
            let (_dir, mut worker, doc) = open(&two_sizes(asian));
            worker.run(&doc, "edit.select_all", &[]).expect("select all");
            let all = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| !seen.selection.is_empty());
            let line = all.selection[0];
            worker.post_key(&doc, 0, 1028).expect("home");
            let one = poll_until(&mut worker, &doc, &all, HANG, |seen| {
                state(seen, ".uno:FontHeight").is_some_and(|size| size != crate::MIXED)
            });
            select_line(&mut worker, &doc, line, drag, backward);
            let seen = poll_until(&mut worker, &doc, &one, HANG, |seen| state(seen, ".uno:FontHeight") == Some(crate::MIXED));
            let case = format!("asian sizes {asian}, drag {drag}, backward {backward}");
            assert_eq!(state(&seen, ".uno:FontHeight"), Some(crate::MIXED), "{case}");
        }
    }
}
