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

/// The pushed state of `uno` once the whole text is selected.
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
