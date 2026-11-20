use ely_mail_engine::{
    Part, Shown,
    page::{self, Block, Image, Inline, Page, Source, Style},
};

fn text(text: &str) -> Block {
    Block::Text(Inline {
        text: text.into(),
        ..Inline::default()
    })
}

fn logo() -> Part {
    Part {
        name: None,
        content_type: "image/png".into(),
        params: Vec::new(),
        content_id: Some("logo@atrium".into()),
        bytes: b"\x89PNG".to_vec(),
        section: Some("1.2".into()),
    }
}

#[test]
fn words_collapse_and_keep_their_styles_and_safe_links() {
    let page = page::html(
        "<p>Hello   <b>bold\n words</b> and <a href=' https://ex.am/ple '>a link</a>\
         <a href='javascript:alert(1)'> not one</a></p>",
        &[],
    );
    let [Block::Text(inline)] = &page.blocks[..] else {
        panic!("one paragraph: {page:?}");
    };
    assert_eq!(inline.text, "Hello bold words and a link not one");
    assert_eq!(inline.styles, [(6..16, Style::Strong)]);
    assert_eq!(inline.links, [(21..27, "https://ex.am/ple".to_owned())]);
}

#[test]
fn nothing_hidden_or_run_reaches_the_page() {
    let page = page::html(
        "<html><head><title>T</title><style>p{}</style></head><body>\
         <script>steal()</script>\
         <div style='display: none !important'>preheader</div>\
         <span hidden>gone</span>\
         <div style='max-height:0; overflow:hidden'>also gone</div>\
         <p style='opacity:0.5'>shown</p></body></html>",
        &[],
    );
    assert_eq!(page.blocks, [text("shown")]);
}

#[test]
fn pictures_come_from_the_mail_and_remote_ones_wait() {
    let html = "<p>Before<img src='cid:logo%40atrium' alt=' Logo '>after</p>\
        <img src='https://cdn.ex/a.png'><img src='//cdn.ex/b.png'>\
        <a href='https://shop.ex'><img src='https://cdn.ex/a.png' alt='again'></a>\
        <img src='https://track.ex/p.gif' width='1' height='1px'>\
        <div style='display:none'><img src='https://track.ex/q.gif'></div>\
        <img src='data:image/png;base64,iVBO Rw=='><img src='cid:nobody'>\
        <img src='file:///etc/passwd'>";
    let page = page::html(html, &[logo()]);
    let image = |alt: &str, source, link: Option<&str>| {
        Block::Image(Image {
            alt: alt.into(),
            source,
            link: link.map(str::to_owned),
        })
    };
    let a = "https://cdn.ex/a.png".to_owned();
    assert_eq!(
        page.blocks,
        [
            text("Before"),
            image("Logo", Source::Inline(0), None),
            text("after"),
            image("", Source::Remote(a.clone()), None),
            image("", Source::Remote("https://cdn.ex/b.png".into()), None),
            image("again", Source::Remote(a.clone()), Some("https://shop.ex")),
            image(
                "",
                Source::Data {
                    content_type: "image/png".into(),
                    bytes: b"\x89PNG".to_vec(),
                },
                None
            ),
            image("", Source::Broken, None),
            image("", Source::Broken, None),
        ]
    );
    assert_eq!(page.remote, [a, "https://cdn.ex/b.png".into()]);
    assert_eq!(page.trackers, 2);
}

#[test]
fn structure_keeps_headings_lists_quotes_code_and_lines() {
    let page = page::html(
        "<h2>Plan</h2><ol start='3'><li>one<ul><li>deep</li></ul></li><li>two</li></ol>\
         <blockquote><p>said</p><blockquote>before</blockquote></blockquote>\
         <pre>  let x = 1;\n  x</pre><hr><p>a<br>  b</p><table><tr><td>cell</td><td>next</td></tr></table>",
        &[],
    );
    let plain = |words: &str| Inline {
        text: words.into(),
        ..Inline::default()
    };
    assert_eq!(
        page.blocks,
        [
            Block::Heading(2, plain("Plan")),
            Block::List {
                start: Some(3),
                items: vec![
                    vec![
                        text("one"),
                        Block::List {
                            start: None,
                            items: vec![vec![text("deep")]],
                        },
                    ],
                    vec![text("two")],
                ],
            },
            Block::Quote(vec![text("said"), Block::Quote(vec![text("before")])]),
            Block::Code("  let x = 1;\n  x".into()),
            Block::Rule,
            text("a\nb"),
            text("cell"),
            text("next"),
        ]
    );
}

/// Quotes nested `n` deep.
fn depth(blocks: &[Block]) -> usize {
    blocks
        .iter()
        .map(|block| match block {
            Block::Quote(inner) => 1 + depth(inner),
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

#[test]
fn deep_quotes_flatten_but_keep_their_words() {
    let html = format!(
        "{}deepest{}",
        "<blockquote>".repeat(40),
        "</blockquote>".repeat(40)
    );
    let page = page::html(&html, &[]);
    assert_eq!(depth(&page.blocks), 16);
    let quoted = "> ".repeat(40) + "deepest";
    assert_eq!(depth(&page::plain(&quoted).blocks), 16);
}

#[test]
fn a_tree_too_deep_is_cut_and_said() {
    let html = format!("{}x", "<div>".repeat(2000));
    let page = page::html(&html, &[]);
    assert!(page.short);
}

#[test]
fn plain_text_keeps_lines_quotes_and_links() {
    let page = page::plain(
        "Hi Ana,\r\nsee https://ex.am/p?q=1.\r\n\r\n> You wrote:\r\n>> first\r\nThanks",
    );
    let link = Inline {
        text: "Hi Ana,\nsee https://ex.am/p?q=1.".into(),
        styles: Vec::new(),
        links: vec![(12..31, "https://ex.am/p?q=1".into())],
    };
    assert_eq!(
        page,
        Page {
            blocks: vec![
                Block::Text(link),
                Block::Quote(vec![text("You wrote:"), Block::Quote(vec![text("first")])]),
                text("Thanks"),
            ],
            ..Page::default()
        }
    );
}

#[test]
fn hidden_list_items_and_hidden_parts_of_code_stay_out() {
    let page = page::html(
        "<ul><li hidden>secret<img src='https://track.ex/p.png'></li><li>visible</li></ul>\
         <pre>shown<span hidden>secret</span></pre>",
        &[],
    );
    let list = Block::List {
        start: None,
        items: vec![vec![text("visible")]],
    };
    assert_eq!(page.blocks, [list, Block::Code("shown".into())]);
    assert!(page.remote.is_empty());
    assert_eq!(page.trackers, 1);
}

#[test]
fn an_important_hidden_style_in_any_case_stays_out() {
    for style in [
        "display:none !IMPORTANT",
        "display: none ! Important",
        "VISIBILITY:Hidden!important",
    ] {
        let page = page::html(
            &format!(
                "<ul><li style='{style}'>secret<img src='https://t.ex/p.png'></li><li>visible</li></ul>"
            ),
            &[],
        );
        let list = Block::List {
            start: None,
            items: vec![vec![text("visible")]],
        };
        assert_eq!(page.blocks, [list], "{style}");
        assert!(page.remote.is_empty(), "{style}");
        assert_eq!(page.trackers, 1, "{style}");
    }
}

#[test]
fn every_body_reads_in_order_with_pictures_once() {
    let shown = [
        Shown::Text("Appointment is at 09:00".into()),
        Shown::Html(
            "<p>Directions</p><img src='https://cdn.ex/map.png'>\
             <img src='https://track.ex/p.gif' width='1' height='1'>"
                .into(),
        ),
        Shown::Html("<img src='https://cdn.ex/map.png'>".into()),
    ];
    let page = page::read(&shown, &[]);
    let map = "https://cdn.ex/map.png".to_owned();
    let image = Block::Image(Image {
        alt: String::new(),
        source: Source::Remote(map.clone()),
        link: None,
    });
    assert_eq!(
        page.blocks,
        [
            text("Appointment is at 09:00"),
            text("Directions"),
            image.clone(),
            image
        ]
    );
    assert_eq!(page.remote, [map]);
    assert_eq!(page.trackers, 1);
}

#[test]
fn a_snippet_is_the_first_unquoted_words_on_one_line() {
    let page = page::html(
        "<blockquote><p>older words</p></blockquote><h2>Plan</h2>\
         <ul><li>one</li><li>two\n  three</li></ul><p>after</p>",
        &[],
    );
    assert_eq!(page.snippet(80), "Plan one two three after");
    assert_eq!(page.snippet(8), "Plan one");
    let quoted = page::plain("> only\n> quoted words");
    assert_eq!(quoted.snippet(80), "only quoted words");
    assert_eq!(page::plain("").snippet(80), "");
    assert_eq!(page::plain("日本語の本文").snippet(3), "日本語");
}

#[test]
fn a_list_start_past_an_html_integer_counts_from_one() {
    let list = |start: &str| {
        let page = page::html(
            &format!("<ol start={start}><li>one</li><li>two</li></ol>"),
            &[],
        );
        page.text()
    };
    assert_eq!(list("18446744073709551615"), "1. one\n2. two");
    assert_eq!(list("4294967295"), "4294967295. one\n4294967296. two");
}

#[test]
fn words_stray_in_a_table_go_before_it() {
    let page = page::html(
        "<table><tr><td>cell</td></tr>stray</table><p>after</p>",
        &[],
    );
    assert_eq!(
        page.blocks,
        [text("stray"), text("cell"), text("after")],
        "as a browser places them"
    );
}

#[test]
fn a_hostile_href_never_becomes_a_link() {
    let hostile = [
        "file:///etc/passwd",
        "FILE://c:/Windows/System32/calc.exe",
        "javascript:alert(1)",
        " JaVaScRiPt:alert(1)",
        "java\tscript:alert(1)",
        "vbscript:msgbox(1)",
        "data:text/html,<b>x</b>",
        "smb://host/share",
        "\\\\host\\share\\run.exe",
        "ms-msdt:/id PCWDiagnostic",
        "search-ms:query=x",
        "ftp://a.example/",
        "x-apple.systempreferences:",
    ];
    for href in hostile {
        let html = format!("<p><a href='{href}'>text</a><a href='{href}'><img src='cid:logo%40atrium' alt='pic'></a></p>");
        let page = page::html(&html, &[logo()]);
        for block in &page.blocks {
            match block {
                Block::Text(inline) => assert!(inline.links.is_empty(), "{href}: {inline:?}"),
                Block::Image(image) => assert_eq!(image.link, None, "{href}"),
                _ => {}
            }
        }
    }
    // Web and mail stay links.
    let page = page::html("<p><a href='mailto:x@a.example'>m</a></p>", &[]);
    let [Block::Text(inline)] = &page.blocks[..] else {
        panic!("one paragraph: {page:?}");
    };
    assert_eq!(inline.links, [(0..1, "mailto:x@a.example".to_owned())]);
}
