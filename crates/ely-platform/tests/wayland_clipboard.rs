//! Native Wayland apps share the clipboard (#24).
#![cfg(target_os = "linux")]

use std::{
    io::Write,
    process::{Command, Stdio},
};

use ely_platform::clipboard::{self, Clip};

/// wl-copy puts `data` on the Wayland clipboard as `mime`.
fn wl_copy(mime: &str, data: &[u8]) {
    let mut copy = Command::new("wl-copy")
        .args(["--type", mime])
        .stdin(Stdio::piped())
        .spawn()
        .expect("wl-copy starts");
    copy.stdin.take().expect("stdin").write_all(data).expect("wl-copy reads");
    assert!(copy.wait().expect("wl-copy ends").success(), "wl-copy copies");
}

/// wl-paste reads `mime` off the Wayland clipboard.
fn wl_paste(mime: &str) -> Vec<u8> {
    let out = Command::new("wl-paste")
        .args(["--no-newline", "--type", mime])
        .output()
        .expect("wl-paste runs");
    assert!(out.status.success(), "wl-paste: {}", String::from_utf8_lossy(&out.stderr));
    out.stdout
}

fn blue_png() -> Vec<u8> {
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(3, 2, image::Rgba([0, 0, 255, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("the png encodes");
    png
}

/// Under a compositor; X11's clipboard stays empty.
#[test]
fn wayland_copies_cross_both_ways() {
    if std::env::var_os("ELY_WAYLAND_CLIPBOARD_TEST").is_none() {
        eprintln!("SKIP: tests/scripts/wayland-clipboard.sh runs this");
        return;
    }
    wl_copy("text/plain;charset=utf-8", "outside 中文".as_bytes());
    assert_eq!(
        clipboard::read().expect("text reads"),
        Some(Clip::Text("outside 中文".into()))
    );
    wl_copy("text/html", b"<b>bold</b>");
    let Some(Clip::Html { html, .. }) = clipboard::read().expect("html reads") else {
        panic!("html comes in as html");
    };
    assert_eq!(html, "<b>bold</b>");
    wl_copy("image/png", &blue_png());
    let Some(Clip::Png(png)) = clipboard::read().expect("png reads") else {
        panic!("an image comes in as png");
    };
    let image = image::load_from_memory(&png).expect("decodes").to_rgba8();
    assert_eq!((image.width(), image.height()), (3, 2));
    assert_eq!(image.get_pixel(2, 1).0, [0, 0, 255, 255]);

    clipboard::write(&Clip::Text("from Ely 中文".into())).expect("text writes");
    assert_eq!(wl_paste("text/plain;charset=utf-8"), "from Ely 中文".as_bytes());
    clipboard::write(&Clip::Html {
        html: "<i>x</i>".into(),
        text: "x".into(),
    })
    .expect("html writes");
    assert_eq!(wl_paste("text/html"), b"<i>x</i>");
    clipboard::write(&Clip::Png(blue_png())).expect("png writes");
    let image = image::load_from_memory(&wl_paste("image/png")).expect("decodes").to_rgba8();
    assert_eq!(image.get_pixel(0, 0).0, [0, 0, 255, 255]);
}
