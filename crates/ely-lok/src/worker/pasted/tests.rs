use super::*;

/// A one-pixel PNG as base64.
fn png() -> String {
    let mut bytes = Vec::new();
    image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
        .write_to(&mut io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("the png encodes");
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

const BAD: &str = "dGhpcyBpcyBub3QgYSBwbmc=";

#[test]
fn every_spelling_of_a_broken_image_is_refused() {
    let spellings = [
        r#"<img src="data:image/png;base64,{}">"#,
        r#"<img src="DATA:image/png;base64,{}">"#,
        r#"<img src="d&#97;ta:image/png;base64,{}">"#,
        r#"<img src="d&#97ta:image/png;base64,{}">"#,
        r#"<img src="da&#10;ta:image/png;base64,{}">"#,
        r#"<img src="d\ata:image/png;base64,{}">"#,
        r#"<img src=" data:image/png;base64,{}">"#,
        r#"<img data="data:image/png;base64,{}">"#,
        r#"<img src="" data="data:image/png;base64,{}">"#,
        r#"<table background="data:image/png;base64,{}">"#,
        r#"<p style="background:url(data:image/png;base64,{})">"#,
        r#"<p style="background:url('d&#97;ta:image/png;base64,{}')">"#,
        r#"<style>p {{ background: url(data:image/png;base64,{}) }}</style>"#,
        r#"<style><!-- p {{ background: url(DATA:image/png;base64,{}) }} --></style>"#,
        r#"<title><img src="data:image/png;base64,{}"></title>"#,
        r#"<textarea><img src="data:image/png;base64,{}"></textarea>"#,
        r#"<plaintext><img src="data:image/png;base64,{}">"#,
        r#"<noframes><img src="data:image/png;base64,{}"></noframes>"#,
        r#"<img src="d\\ata:image/png;base64,{}">"#,
        r#"<img src="data:image/png;bas\\e64,{}">"#,
        r#"<img src="DATA:IMAGE/PNG;base64,{}">"#,
    ];
    for spelling in spellings {
        let html = spelling.replace("{}", BAD).replace("{{", "{").replace("}}", "}");
        assert!(rebuild(&html).is_err(), "{html} passed");
    }
    let escaped = r#"<p style="background:url(\64 ata:image/png;base64,iVBO)">"#;
    assert!(rebuild(escaped).is_err(), "a CSS escape hid an image");
}

#[test]
fn a_good_image_reaches_the_engine_written_plainly() {
    let png = png();
    let plain = format!(r#"<img src="data:image/png;base64,{png}">"#);
    for spelling in [r#"src="DATA:"#, r#"src="d&#97;ta:"#, r#"src="da&#10;ta:"#, r#"src="d\ata:"#] {
        let html = format!(r#"<img {spelling}image/png;bas\e64,{png}">"#);
        assert_eq!(rebuild(&html).expect("a good image"), plain, "{html}");
    }
    for wrap in ["textarea", "plaintext", "noframes"] {
        let html = format!(r#"<{wrap}><img src="data:image/png;bas\e64,{png}"></{wrap}>"#);
        assert_eq!(rebuild(&html).expect("a good image"), format!("<{wrap}>{plain}</{wrap}>"), "{html}");
    }
    let style = format!(r#"<p style="background:url(data:image/png;base64,{png})">x</p>"#);
    assert_eq!(rebuild(&style).expect("a good background"), style);
}

#[test]
fn only_what_the_tokenizer_read_reaches_the_engine() {
    let cases = [
        // Comments carry nothing.
        (format!(r#"<p>a<!-- <img src="data:image/png;base64,{BAD}"> --></p>"#), "<p>a</p>"),
        ("<p>1 &lt; 2 &amp;&amp; <b>x</b></p>".to_owned(), "<p>1 &lt; 2 &amp;&amp; <b>x</b></p>"),
        (r#"<a href="data:text/plain,hi" onclick='x"y'>l</a>"#.to_owned(), r#"<a href="data:text/plain,hi" onclick="x&quot;y">l</a>"#),
        (r#"<img a"b="1" src="x.png"><x"y>z"#.to_owned(), r#"<img src="x.png">z"#),
        // Word's escaped CJK font names stay as written.
        (r#"<style><!-- p { font-family:"\5B8B\4F53"; } a > b {} --></style>"#.to_owned(), r#"<style><!-- p { font-family:"\5B8B\4F53"; } a > b {} --></style>"#),
        (format!(r#"<img src="x.png" src="data:image/png;base64,{BAD}">"#), r#"<img src="x.png">"#),
    ];
    for (html, rebuilt) in cases {
        assert_eq!(rebuild(&html).expect("the HTML rebuilds"), rebuilt, "{html}");
    }
}

#[test]
fn raw_text_holding_markup_is_refused() {
    let png = png();
    for html in [
        format!(r#"<style></head><img src="d&#97;ta:image/png;base64,{BAD}"></style>"#),
        format!(r#"<style><body><img src="data:image/png;base64,{png}"></style>"#),
        format!(r#"<script></script1><img src="data:image/png;bas\e64,{png}"></script>"#),
        format!(r#"<xmp></body><img src="data:image/png;bas\e64,{png}"></xmp>"#),
        format!(r#"<noembed></html><img src="data:image/png;base64,{png}"></noembed>"#),
        "<script>if (a < b) {}</script>".to_owned(),
    ] {
        assert!(rebuild(&html).is_err(), "{html} passed");
    }
    let kept = "<style><!-- p { color: red } --></style><script>go()</script><xmp>a & b</xmp>";
    assert_eq!(rebuild(kept).expect("raw text without markup"), kept);
}
