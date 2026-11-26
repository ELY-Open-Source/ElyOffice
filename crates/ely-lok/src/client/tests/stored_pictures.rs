//! Office saves keep PNG and JPEG stored, as ODF.

use std::io::Cursor;

use base64::Engine as _;
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};

use super::*;

/// A small noisy picture; every codec keeps its own bytes.
fn picture(format: ImageFormat, seed: u32) -> Vec<u8> {
    let pixels = RgbImage::from_fn(64, 48, |x, y| {
        let n = (x * 7919 + y * 104_729 + seed).wrapping_mul(2_654_435_761);
        Rgb([(n >> 24) as u8, (n >> 16) as u8, (n >> 8) as u8])
    });
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(pixels)
        .write_to(&mut out, format)
        .expect("encodes");
    out.into_inner()
}

/// Both pictures as flat ODF frames.
fn frames(pictures: &[Vec<u8>]) -> String {
    (pictures.iter().enumerate())
        .map(|(n, bytes)| {
            let data = base64::engine::general_purpose::STANDARD.encode(bytes);
            let anchor = if n == 0 { 1 } else { 6 };
            format!(r#"<draw:frame svg:x="1cm" svg:y="{anchor}cm" svg:width="4cm" svg:height="3cm"><draw:image><office:binary-data>{data}</office:binary-data></draw:image></draw:frame>"#)
        })
        .collect()
}

fn flat(kind: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.{kind}">
 <office:body>{body}</office:body>
</office:document>"#
    )
}

/// Each entry's name and compression method.
fn methods(bytes: &[u8]) -> Vec<(String, u16)> {
    let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let end = (0..bytes.len() - 21)
        .rev()
        .find(|&at| bytes[at..].starts_with(b"PK\x05\x06"))
        .expect("an end");
    let mut at =
        u32::from_le_bytes(bytes[end + 16..end + 20].try_into().expect("four bytes")) as usize;
    (0..u16_at(end + 10))
        .map(|_| {
            let name_len = usize::from(u16_at(at + 28));
            let name = String::from_utf8_lossy(&bytes[at + 46..at + 46 + name_len]).into_owned();
            let method = u16_at(at + 10);
            at += 46 + name_len + usize::from(u16_at(at + 30)) + usize::from(u16_at(at + 32));
            (name, method)
        })
        .collect()
}

#[test]
fn office_saves_store_png_and_jpeg_without_deflating() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let pictures = [picture(ImageFormat::Png, 1), picture(ImageFormat::Jpeg, 2)];
    let both = frames(&pictures);
    let cases = [
        (
            "in.fodt",
            "docx",
            flat(
                "text",
                &format!("<office:text><text:p>{both}text</text:p></office:text>"),
            ),
        ),
        (
            "in.fods",
            "xlsx",
            flat(
                "spreadsheet",
                &format!(
                    r#"<office:spreadsheet><table:table table:name="S"><table:table-column/><table:table-row><table:table-cell/></table:table-row><table:shapes>{both}</table:shapes></table:table></office:spreadsheet>"#
                ),
            ),
        ),
        (
            "in.fodp",
            "pptx",
            flat(
                "presentation",
                &format!(
                    r#"<office:presentation><draw:page draw:name="One">{both}</draw:page></office:presentation>"#
                ),
            ),
        ),
    ];
    let mut wrong = Vec::new();
    for (source, format, body) in cases {
        let dir = tempfile::tempdir().expect("a temp dir");
        let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
        let path = dir.path().join(source);
        std::fs::write(&path, body).expect("writes");
        let doc = worker.load(&crate::file_url(&path)).expect("loads");
        let out = dir.path().join(format!("out.{format}"));
        doc.save_owned(&mut worker, &crate::file_url(&out), format)
            .expect("saves");
        worker.close_doc(doc).expect("closes");
        let bytes = std::fs::read(&out).expect("reads");
        // The reader inflates and checks every size.
        let parts = super::zip::entries(&bytes);
        let methods = methods(&bytes);
        for (original, extension) in pictures.iter().zip([".png", ".jp"]) {
            let found = parts
                .iter()
                .find(|(name, _)| name.contains("/media/") && name.contains(extension));
            let Some((name, body)) = found else {
                wrong.push(format!("{format}: no {extension} part in {methods:?}"));
                continue;
            };
            assert_eq!(body, original, "{format}: {name} keeps its bytes");
            let method = methods
                .iter()
                .find(|(entry, _)| entry == name)
                .expect("listed")
                .1;
            if method != 0 {
                wrong.push(format!("{format}: {name} uses method {method}, not stored"));
            }
        }
        let types = methods
            .iter()
            .find(|(name, _)| name == "[Content_Types].xml")
            .expect("content types")
            .1;
        assert_eq!(types, 8, "{format}: XML stays deflated");
        let again = worker
            .load(&crate::file_url(&out))
            .expect("the save reopens");
        worker.close_doc(again).expect("closes");
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
