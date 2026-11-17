//! Inline images in pasted HTML (C07.3).

use super::input::{open, paragraphs, red_png, saved};
use super::*;

/// HTML carrying `png` where `open` starts its URI.
fn inline_html(open: &str, png: &[u8]) -> String {
    use base64::Engine;
    let data = base64::engine::general_purpose::STANDARD.encode(png);
    format!(r#"<html><body><img {open}image/png;base64,{data}"></body></html>"#)
}

/// Every review round's spelling of an image the engine loads.
const OPENS: [&str; 7] = [r#"src="data:"#, r#"src="DATA:"#, r#"src="d&#97;ta:"#, r#"src="da&#10;ta:"#, r#"src="d\ata:"#, r#"data="data:"#, r#"src="" data="data:"#];

/// Each spelling of `png` as an image, in plain HTML.
fn spellings(png: &[u8]) -> Vec<String> {
    let plain = inline_html(r#"src="data:"#, png);
    let mut all: Vec<String> = OPENS.iter().map(|open| inline_html(open, png)).collect();
    all.push(plain.replace(";base64", ";bas\\e64"));
    all.push(plain.replace("data:", "d\\\\ata:"));
    all.push(plain.replace(";base64", ";bas\\\\e64"));
    all.push(plain.replace("data:image/png", "DATA:IMAGE/PNG"));
    // The engine reads these as markup.
    for wrap in ["textarea", "plaintext", "noframes"] {
        let escaped = plain.replace(";base64", ";bas\\e64");
        all.push(escaped.replace("<img", &format!("<{wrap}><img")).replace("</body>", &format!("</{wrap}></body>")));
    }
    all
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
    let broken = b"this is not a png";
    let mut refused = spellings(broken);
    // The engine ends these raw texts early.
    for png in [&broken[..], &red_png()] {
        use base64::Engine;
        let data = base64::engine::general_purpose::STANDARD.encode(png);
        for (name, inside) in [("style", "</head>"), ("style", "<body>"), ("script", "</script1>"), ("xmp", "</body>"), ("xmp", "</html>")] {
            refused.push(format!(r#"<html><body><{name}>{inside}<img src="data:image/png;bas\e64,{data}"></{name}></body></html>"#));
        }
    }
    refused.push(inline_html(r#"src="data:"#, broken).replace("<img src=", "<table background="));
    refused.push(inline_html(r#"style="background:url(data:"#, broken));
    for html in refused {
        assert!(worker.paste(&doc, "text/html", html.as_bytes()).is_err(), "{html} pasted");
        assert!(!worker.copy(&doc).expect("copy runs").is_empty(), "{html}: the selection went");
        let text = saved(&dir, &mut worker, &doc, "kept.fodt");
        assert_eq!(paragraphs(&text), ["Hello 世界"], "{html}: the text went");
    }
    worker
        .post_key(&doc, 0, 1029)
        .expect("End drops the selection");
    let good = spellings(&red_png());
    for html in &good {
        worker.paste(&doc, "text/html", html.as_bytes()).unwrap_or_else(|error| panic!("{html}: {error}"));
    }
    let mention = "<html><body><p>Use data:image/png here.</p></body></html>";
    worker.paste(&doc, "text/html", mention.as_bytes()).expect("text naming an image pastes");
    let text = saved(&dir, &mut worker, &doc, "image.fodt");
    // Each checked image is embedded, never a placeholder.
    assert_eq!(text.matches("<office:binary-data").count(), good.len(), "the images land");
    assert_eq!(text.matches("<draw:image").count(), good.len(), "no placeholder lands");
    assert!(text.contains("Use data:image/png here."), "the text lands");
}
