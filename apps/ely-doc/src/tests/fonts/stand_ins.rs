//! A missing Word font draws as the nearest free font.

use ely_office::prelude::*;
use gpui::TestAppContext;

use crate::tests::{docs, settle, settled, with_window};

/// Missing font, and the family that draws it.
const STAND_INS: [(&str, &str); 6] = [
    ("Calibri Light", "Carlito"),
    ("Verdana", "DejaVu Sans"),
    ("Century Gothic", "DejaVu Sans"),
    ("Georgia", "Noto Serif"),
    ("Consolas", "Liberation Mono"),
    ("Lucida Console", "Liberation Mono"),
];

/// A Chinese Word font's two names, and its stand-in.
const CHINESE: [(&str, &str, &str); 6] = [
    ("宋体", "SimSun", "Noto Serif SC"),
    ("仿宋", "FangSong", "Noto Serif SC"),
    ("楷体", "KaiTi", "Noto Serif SC"),
    ("黑体", "SimHei", "Noto Sans SC"),
    ("微软雅黑", "Microsoft YaHei", "Noto Sans SC"),
    ("等线", "DengXian", "Noto Sans SC"),
];

/// The families fontconfig lists on this machine.
fn installed() -> Vec<String> {
    let out = std::process::Command::new("fc-list")
        .args(["--format", "%{family}\n"])
        .output()
        .expect("fc-list runs");
    assert!(out.status.success(), "fc-list: {out:?}");
    String::from_utf8_lossy(&out.stdout).lines().flat_map(|line| line.split(',')).map(str::to_string).collect()
}

/// A document with one span in each font.
fn document() -> String {
    let (mut faces, mut styles, mut spans) = (String::new(), String::new(), String::new());
    let chinese = CHINESE.iter().flat_map(|(zh, en, _)| [*zh, *en]);
    let fonts = STAND_INS.iter().map(|(font, _)| *font).chain(chinese);
    for (at, font) in fonts.enumerate() {
        faces += &format!("<style:font-face style:name=\"{font}\" svg:font-family=\"&apos;{font}&apos;\"/>");
        styles += &format!(
            "<style:style style:name=\"T{at}\" style:family=\"text\"><style:text-properties style:font-name=\"{font}\"/></style:style>"
        );
        spans += &format!("<text:span text:style-name=\"T{at}\">word{at} 中文字体</text:span> ");
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text"><office:font-face-decls>{faces}</office:font-face-decls><office:automatic-styles>{styles}</office:automatic-styles><office:body><office:text><text:p>{spans}</text:p></office:text></office:body></office:document>"#
    )
}

#[test]
fn missing_word_fonts_draw_as_the_nearest_free_font() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let path = dir.path().join("fonts.fodt");
    std::fs::write(&path, document()).expect("the file writes");
    with_window(&mut cx, &docs, |docs, _, cx| docs.open_file(path.clone(), cx));
    settle(&cx, |app| {
        let tab = docs.read(app).active_tab();
        tab.is_some_and(|tab| !tab.view.read(app).missing_fonts().is_empty())
    });
    settled(&cx, &docs);
    let missing = cx.update(|app| docs.read(app).active_tab().expect("a tab").view.read(app).missing_fonts().to_vec());
    let families = installed();
    let installed = |font: &str| families.iter().any(|family| family == font);
    let drawn = |font: &str| missing.iter().find(|(name, _)| name == font).map(|(_, drawn)| drawn);
    // An installed original wins over its stand-in.
    let mut checked = 0;
    for (font, used) in STAND_INS {
        match drawn(font) {
            Some(drawn) => {
                assert_eq!(drawn, &[used.to_string()], "{font}");
                checked += 1;
            }
            None => assert!(installed(font), "{font} is neither missing nor installed: {missing:?}"),
        }
    }
    assert!(checked > 0, "some stand-in was needed: {missing:?}");
    // Another font may draw their Latin letters.
    let mut checked = 0;
    for (zh, en, family) in CHINESE {
        for font in [zh, en] {
            match drawn(font) {
                Some(drawn) => {
                    assert!(drawn.iter().any(|used| used == family), "{font}: {drawn:?}");
                    checked += 1;
                }
                // The kernel takes either name for the other.
                None => assert!(installed(zh) || installed(en), "{font} is neither missing nor installed: {missing:?}"),
            }
        }
    }
    assert!(checked > 0, "some Chinese stand-in was needed: {missing:?}");
}
