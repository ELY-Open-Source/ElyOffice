//! A missing Chinese font falls to its kind's face.

use super::input::open;
use super::*;

/// Names no system has: one Hei, one Song.
pub(super) const ASKED: [&str; 2] = ["Ely 无此黑体", "Ely 无此宋体"];

/// What each falls to, by kind: the kernel's own Noto.
#[cfg(not(target_os = "linux"))]
pub(super) const KINDS: [&[&str]; 2] = [&["Noto Sans SC"], &["Noto Serif SC"]];
/// Fontconfig may take an installed face of the kind.
#[cfg(target_os = "linux")]
pub(super) const KINDS: [&[&str]; 2] = [
    &["Noto Sans SC", "Noto Sans CJK SC", "Liberation Sans"],
    &["Noto Serif SC", "Noto Serif CJK SC", "Liberation Serif"],
];

/// Why each is taken: the language's list, or fontconfig.
#[cfg(not(target_os = "linux"))]
pub(super) const WHY: [&str; 2] = ["zh-CN sans list", "zh-CN text list"];
#[cfg(target_os = "linux")]
pub(super) const WHY: [&str; 2] = ["system substitute", "system substitute"];

/// A Chinese paragraph in each of `ASKED`.
pub(super) fn chinese() -> String {
    let faces: String = (ASKED.iter().enumerate())
        .map(|(n, name)| format!(r#"<style:font-face style:name="F{n}" svg:font-family="&apos;{name}&apos;"/>"#))
        .collect();
    let styles: String = (0..ASKED.len())
        .map(|n| format!(r#"<style:style style:name="P{n}" style:family="paragraph"><style:text-properties style:font-name="F{n}" style:font-name-asian="F{n}" fo:language="zh" fo:country="CN" style:language-asian="zh" style:country-asian="CN"/></style:style>"#))
        .collect();
    let text: String = (0..ASKED.len()).map(|n| format!(r#"<text:p text:style-name="P{n}">演示标题正文内容</text:p>"#)).collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:font-face-decls>{faces}</office:font-face-decls>
 <office:automatic-styles>{styles}</office:automatic-styles>
 <office:body><office:text>{text}</office:text></office:body>
</office:document>"#
    )
}

#[test]
fn a_missing_chinese_font_falls_to_a_text_face_of_its_kind() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open(&chinese());
    let seen = poll_until(&mut worker, &doc, &Events::default(), std::time::Duration::from_secs(5), |seen| {
        seen.fonts.len() == ASKED.len()
    });
    let wrong: Vec<_> = (ASKED.iter().zip(KINDS))
        .filter_map(|(asked, kind)| {
            let (_, used) = (seen.fonts.iter()).find(|(name, _)| name == asked)?;
            let chinese = used.iter().any(|face| face.ends_with(" SC"));
            (!chinese || !used.iter().all(|face| kind.contains(&face.as_str()))).then(|| format!("{asked} drawn in {used:?}, not {kind:?}"))
        })
        .collect();
    assert_eq!(seen.fonts.len(), ASKED.len(), "both are missing: {:?}", seen.fonts);
    assert!(wrong.is_empty(), "{wrong:?}");
}
