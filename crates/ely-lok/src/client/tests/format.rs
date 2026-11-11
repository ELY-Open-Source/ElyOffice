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

/// A paragraph default with Chinese as its Asian language.
const ZH: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:styles><style:default-style style:family="paragraph"><style:text-properties fo:language="en" fo:country="US" style:language-asian="zh" style:country-asian="CN"/></style:default-style></office:styles>
 <office:body><office:text><text:p/></office:text></office:body>
</office:document>"#;

/// The automatic style named `name` in `xml`.
fn style_of<'a>(xml: &'a str, name: &str) -> &'a str {
    let start = xml.find(&format!("<style:style style:name=\"{name}\"")).expect("the style");
    let end = start + xml[start..].find("</style:style>").expect("its end");
    &xml[start..end]
}

/// Bare-cursor size sets every script (#62).
#[test]
fn a_size_on_an_empty_line_sets_every_script() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(ZH);
    click(&mut worker, &doc, 150, 150);
    worker.run(&doc, "format.size", &["14"]).expect("the size takes on the empty line");
    worker.post_text(&doc, false, "中文").expect("the text posts");
    worker.post_text(&doc, true, "").expect("the commit posts");
    let xml = saved(&dir, &mut worker, &doc, "out.fodt");
    let at = xml.find("中文</text:p>").expect("the typed text");
    let tag = "<text:p text:style-name=\"";
    let open = xml[..at].rfind(tag).expect("its paragraph") + tag.len();
    let name = &xml[open..open + xml[open..].find('"').expect("the name")];
    let style = style_of(&xml, name);
    assert!(style.contains("fo:font-size=\"14pt\""), "Western: {style}");
    assert!(style.contains("style:font-size-asian=\"14pt\""), "Asian: {style}");
}

/// Two looks, `pairs` times; `kind` says how.
fn two_looks(kind: &str, pairs: usize) -> String {
    let look = |name: &str, family: &str, font: &str, size: &str, ink: &str, back: &str| {
        format!(r#"<style:style style:name="{name}" style:family="{family}"><style:text-properties style:font-name="{font}" fo:font-size="{size}" fo:color="{ink}" fo:background-color="{back}"/></style:style>"#)
    };
    let (styles, automatic, span) = match kind {
        "styles" => (look("ElyA", "text", "Serif", "12pt", "#ff0000", "#ffff00") + &look("ElyB", "text", "Mono", "18pt", "#0000ff", "#00ff00"), String::new(), ["ElyA", "ElyB"]),
        _ => (String::new(), look("T1", "text", "Serif", "12pt", "#ff0000", "#ffff00") + &look("T2", "text", "Mono", "18pt", "#0000ff", "#00ff00"), ["T1", "T2"]),
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:font-face-decls><style:font-face style:name="Serif" svg:font-family="&apos;Liberation Serif&apos;"/><style:font-face style:name="Mono" svg:font-family="&apos;Liberation Mono&apos;"/></office:font-face-decls>
 <office:styles>{styles}</office:styles>
 <office:automatic-styles>{automatic}</office:automatic-styles>
 <office:body><office:text><text:p>{}</text:p></office:text></office:body>
</office:document>"#,
        format!(r#"<text:span text:style-name="{}">Alpha</text:span> <text:span text:style-name="{}">Beta</text:span>"#, span[0], span[1]).repeat(pairs)
    )
}

#[test]
fn mixed_colors_and_highlights_report_dont_care() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    for kind in ["styles", "direct"] {
        let (_dir, mut worker, doc) = open(&two_looks(kind, 1));
        worker.run(&doc, "edit.select_all", &[]).expect("select all");
        let names = [".uno:FontColor", ".uno:CharBackColor", ".uno:FontHeight"];
        let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
            names.iter().all(|name| state(seen, name) == Some(crate::MIXED))
        });
        for name in names {
            assert_eq!(state(&seen, name), Some(crate::MIXED), "{kind}: {name} is mixed");
        }
    }
}

#[test]
fn one_size_and_font_go_over_two_character_styles() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    for (id, value, mark) in [("format.size", "14", "fo:font-size=\"14pt\""), ("format.font", "Liberation Sans", "Liberation Sans")] {
        let (dir, mut worker, doc) = open(&two_looks("styles", 1));
        worker.run(&doc, "edit.select_all", &[]).expect("select all");
        worker.run(&doc, id, &[value]).unwrap_or_else(|error| panic!("{id}: {error}"));
        let text = saved(&dir, &mut worker, &doc, "out.fodt");
        assert!(text.contains(mark), "{id} {value} is not in the file");
    }
}

#[test]
fn a_long_paragraph_of_styles_answers_its_state_at_once() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(&two_looks("styles", 8000));
    worker.run(&doc, "edit.select_all", &[]).expect("select all");
    let asked = ".uno:CommandStates?commands=.uno:FontHeight";
    worker.values(&doc, asked).expect("the state");
    let fastest = (0..3)
        .map(|_| {
            let started = std::time::Instant::now();
            worker.values(&doc, asked).expect("the state");
            started.elapsed()
        })
        .min()
        .expect("three asks");
    // A scan per style and attribute took 400 ms.
    assert!(fastest < std::time::Duration::from_millis(50), "16000 styled runs took {fastest:?}");
}

/// Two styles under one direct look, and an empty span.
const EMPTY_SPAN: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:styles><style:style style:name="A" style:family="text"><style:text-properties fo:font-size="12pt" fo:color="#ff0000"/></style:style><style:style style:name="B" style:family="text"><style:text-properties fo:font-size="18pt" fo:color="#0000ff"/></style:style></office:styles>
 <office:automatic-styles><style:style style:name="X" style:family="text"><style:text-properties fo:font-size="14pt" fo:color="#00aa00"/></style:style><style:style style:name="Z" style:family="text"><style:text-properties fo:font-size="16pt"/></style:style></office:automatic-styles>
 <office:body><office:text><text:p><text:span text:style-name="A"><text:span text:style-name="X">Alpha<text:span text:style-name="Z"></text:span></text:span></text:span><text:span text:style-name="B"><text:span text:style-name="X">Beta</text:span></text:span></text:p></office:text></office:body>
</office:document>"##;

#[test]
fn an_empty_span_hides_no_direct_formatting() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(EMPTY_SPAN);
    worker.run(&doc, "edit.select_all", &[]).expect("select all");
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        state(seen, ".uno:FontHeight") == Some("14") && state(seen, ".uno:FontColor").is_some()
    });
    assert_eq!(state(&seen, ".uno:FontHeight"), Some("14"), "one height covers both styles");
    assert_ne!(state(&seen, ".uno:FontColor"), Some(crate::MIXED), "one color covers both styles");
}
