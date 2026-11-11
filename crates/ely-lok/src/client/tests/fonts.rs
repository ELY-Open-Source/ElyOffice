//! Font names reach the scripts they can draw (GitHub #7).

use super::input::{open, saved};
use super::*;

const ABC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>ABC</text:p></office:text></office:body>
</office:document>"#;

/// Fonts with Chinese glyphs, as each system names them.
const CJK: [&str; 7] = [
    "微软雅黑",
    "宋体",
    "Microsoft YaHei",
    "SimSun",
    "PingFang SC",
    "Noto Sans CJK SC",
    "Noto Serif CJK SC",
];

fn listed(worker: &mut Worker, doc: &Doc) -> Vec<String> {
    let fonts = worker
        .values(doc, ".uno:CharFontName")
        .expect("the font list");
    let fonts: serde_json::Value = serde_json::from_str(&fonts).expect("json");
    let fonts = fonts["commandValues"].as_object().expect("an object");
    fonts.keys().cloned().collect()
}

const MIXED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>ABC 中文</text:p></office:text></office:body>
</office:document>"#;

const CHINESE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>中文</text:p></office:text></office:body>
</office:document>"#;

/// `font` set at the cursor in `body`, then `text` typed.
fn typed_after(body: &str, font: &str, text: &str) -> String {
    let (dir, mut worker, doc) = open(body);
    worker
        .run(&doc, "format.font", &[font])
        .expect("the font runs");
    worker
        .post_text(&doc, false, text)
        .expect("the text posts");
    worker.post_text(&doc, true, "").expect("the commit posts");
    saved(&dir, &mut worker, &doc, "out.fodt")
}

/// `font` set on all of `body`.
fn set_on_all(body: &str, font: &str) -> String {
    let (dir, mut worker, doc) = open(body);
    worker
        .run(&doc, "edit.select_all", &[])
        .expect("select all");
    worker
        .run(&doc, "format.font", &[font])
        .expect("the font runs");
    saved(&dir, &mut worker, &doc, "out.fodt")
}

/// Either way flat ODF names a font.
fn names(xml: &str, attributes: [&str; 2], font: &str) -> bool {
    attributes.iter().any(|attribute| {
        [
            format!("{attribute}=\"{font}\""),
            format!("{attribute}=\"&apos;{font}&apos;\""),
        ]
        .iter()
        .any(|written| xml.contains(written))
    })
}

fn western(xml: &str, font: &str) -> bool {
    names(xml, ["style:font-name", "fo:font-family"], font)
}

fn chinese(xml: &str, font: &str) -> bool {
    names(
        xml,
        ["style:font-name-asian", "style:font-family-asian"],
        font,
    )
}

/// A font with Chinese glyphs, if one is installed.
fn cjk_font() -> Option<String> {
    let (_dir, mut worker, doc) = open(ABC);
    let fonts = listed(&mut worker, &doc);
    CJK.iter()
        .find(|font| fonts.iter().any(|it| it == *font))
        .map(|font| font.to_string())
}

#[test]
fn a_font_with_chinese_glyphs_sets_both_scripts() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let Some(font) = cjk_font() else {
        eprintln!("SKIP: no font with Chinese glyphs here");
        return;
    };
    let xml = typed_after(ABC, &font, "中文");
    assert!(
        western(&xml, &font) && chinese(&xml, &font),
        "at the cursor: {xml}"
    );
    let xml = set_on_all(ABC, &font);
    assert!(
        western(&xml, &font) && chinese(&xml, &font),
        "on Latin text: {xml}"
    );
}

#[test]
fn a_latin_only_font_keeps_the_chinese_font() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    // Chinese at the cursor or selected: the state shows Chinese.
    for xml in [
        typed_after(ABC, "Liberation Mono", "中文"),
        typed_after(CHINESE, "Liberation Mono", "a"),
        set_on_all(MIXED, "Liberation Mono"),
        set_on_all(CHINESE, "Liberation Mono"),
    ] {
        assert!(western(&xml, "Liberation Mono"), "the Western font: {xml}");
        assert!(
            !chinese(&xml, "Liberation Mono"),
            "took the Chinese text: {xml}"
        );
    }
}

#[test]
fn a_selection_past_the_kernel_lookup_still_takes_the_font() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    // Writer reads no attributes over 10,000 paragraphs.
    let paragraphs = "<text:p>ABC 中文</text:p>".repeat(10_001);
    let body = MIXED.replace("<text:p>ABC 中文</text:p>", &paragraphs);
    let xml = set_on_all(&body, "Liberation Mono");
    assert!(western(&xml, "Liberation Mono"), "the font was undone");
    assert!(!chinese(&xml, "Liberation Mono"), "took the Chinese text");
}

/// Many lines in `font`: monospaced if present, else a fallback.
fn in_font(font: &str) -> String {
    let line = "<text:p>The quick brown fox jumps over the lazy dog, then naps in the warm sun.</text:p>";
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:font-face-decls><style:font-face style:name="F" svg:font-family="&apos;{font}&apos;" style:font-family-generic="swiss" style:font-pitch="variable"/></office:font-face-decls>
 <office:styles><style:default-style style:family="paragraph"><style:text-properties style:font-name="F" fo:font-size="12pt"/></style:default-style></office:styles>
 <office:body><office:text>{}</office:text></office:body>
</office:document>"#,
        line.repeat(400)
    )
}

/// Pages and missing fonts once the layout has settled.
fn laid_out(font: &str) -> (usize, Vec<(String, Vec<String>)>) {
    let (_dir, mut worker, doc) = open(&in_font(font));
    // Missing fonts are reported soon after the load.
    let seen = poll_until(&mut worker, &doc, &Events::default(), std::time::Duration::from_secs(5), |seen| {
        !seen.fonts.is_empty()
    });
    let pages = doc.pages(&mut worker).expect("the pages").len();
    (pages, seen.fonts)
}

#[test]
fn a_missing_font_is_named_and_its_pages_counted_apart() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (here, none) = laid_out("DejaVu Sans Mono");
    assert!(none.is_empty(), "a bundled font is not missing: {none:?}");
    // "Mono" picks a stand-in; macOS's Menlo has DejaVu's widths.
    let (gone, missing) = laid_out("Nowhere Sans");
    let [(name, used)] = missing.as_slice() else { panic!("one lost font: {missing:?}") };
    assert_eq!(name, "Nowhere Sans", "the lost font is named");
    // The stand-in shows too (PRD §19.2).
    assert!(!used.is_empty() && !used.contains(name), "what drew it: {used:?}");
    eprintln!("Nowhere Sans drawn in {used:?}");
    eprintln!("pages: {here} with the font, {gone} without");
    assert!(here > 1 && gone > 1, "both lay out pages: {here}, {gone}");
    assert_ne!(here, gone, "a stand-in moves the pages, and they show");
}

/// One font as each system names it, in two languages.
#[cfg(target_os = "linux")]
const BOTH_NAMES: [&str; 2] = ["Ely Alias Sans", "别名黑体"];
#[cfg(target_os = "macos")]
const BOTH_NAMES: [&str; 2] = ["PingFang SC", "苹方-简"];
#[cfg(windows)]
const BOTH_NAMES: [&str; 2] = ["Malgun Gothic", "맑은 고딕"];
/// Its PostScript name, as a PDF embeds it.
#[cfg(target_os = "linux")]
const POSTSCRIPT: &str = "ElyAliasSans";
#[cfg(target_os = "macos")]
const POSTSCRIPT: &str = "PingFangSC";
#[cfg(windows)]
const POSTSCRIPT: &str = "MalgunGothic";
/// An installed font, and its PostScript name.
#[cfg(target_os = "linux")]
const NEXT: [&str; 2] = ["DejaVu Sans", "DejaVuSans"];
#[cfg(target_os = "macos")]
const NEXT: [&str; 2] = ["Helvetica", "Helvetica"];
#[cfg(windows)]
const NEXT: [&str; 2] = ["Arial", "ArialMT"];

/// A span per family list in `fonts`.
fn spans(fonts: &[String]) -> String {
    let faces: String = fonts
        .iter()
        .enumerate()
        .map(|(n, list)| {
            let family: Vec<String> = list.split(';').map(|name| format!("&apos;{name}&apos;")).collect();
            format!(r#"<style:font-face style:name="F{n}" svg:font-family="{}"/>"#, family.join(", "))
        })
        .collect();
    let styles: String = (0..fonts.len())
        .map(|n| {
            format!(r#"<style:style style:name="T{n}" style:family="text"><style:text-properties style:font-name="F{n}" style:font-name-asian="F{n}"/></style:style>"#)
        })
        .collect();
    let text: String = (0..fonts.len())
        .map(|n| format!(r#"<text:span text:style-name="T{n}">Abc{n}</text:span>"#))
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:font-face-decls>{faces}</office:font-face-decls>
 <office:automatic-styles>{styles}</office:automatic-styles>
 <office:body><office:text><text:p>{text}</text:p></office:text></office:body>
</office:document>"#
    )
}

/// `body` loaded by a worker that has the font.
fn with_the_font(body: &str) -> (tempfile::TempDir, Worker, Doc) {
    let install = install().expect("the pinned LOK build");
    let dir = tempfile::tempdir().expect("a temp dir");
    let profile = dir.path().join("profile");
    // Linux reads the profile's fonts; others have theirs.
    if cfg!(target_os = "linux") {
        let fonts = profile.join("user").join("fonts");
        std::fs::create_dir_all(&fonts).expect("the font folder");
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/alias.ttf");
        std::fs::copy(fixture, fonts.join("alias.ttf")).expect("the font copies");
    }
    let mut worker = Worker::spawn(&install, &profile).expect("the worker");
    let source = dir.path().join("in.fodt");
    write(&source, body);
    let doc = worker.load(&crate::file_url(&source)).expect("the document loads");
    (dir, worker, doc)
}

#[test]
fn a_font_is_found_by_its_name_in_another_language() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let fonts = [BOTH_NAMES[0].to_string(), BOTH_NAMES[1].to_string(), "Nowhere Sans".to_string()];
    let (_dir, mut worker, doc) = with_the_font(&spans(&fonts));
    let seen = poll_until(&mut worker, &doc, &Events::default(), std::time::Duration::from_secs(5), |seen| {
        !seen.fonts.is_empty()
    });
    let names: Vec<&str> = seen.fonts.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["Nowhere Sans"], "{BOTH_NAMES:?} name one font");
}

#[test]
fn a_font_in_another_language_comes_before_the_next_in_its_list() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    // A script suffix is stripped before the lookup.
    let suffixed = BOTH_NAMES.map(|name| format!("{name} CYR"));
    for name in BOTH_NAMES.into_iter().chain(suffixed.iter().map(String::as_str)) {
        let (dir, mut worker, doc) = with_the_font(&spans(&[format!("{name};{}", NEXT[0])]));
        let pdf = dir.path().join("out.pdf");
        doc.save_as(&mut worker, &crate::file_url(&pdf), "pdf").expect("the PDF exports");
        let bytes = std::fs::read(&pdf).expect("the PDF reads");
        let names = |font: &str| bytes.windows(font.len()).any(|window| window == font.as_bytes());
        assert!(names(POSTSCRIPT) && !names(NEXT[1]), "{name} drew in another font");
    }
}
