use std::path::Path;

use super::{Pictures, body, document, plain};

mod visual;

fn page(text: &str, beside: Option<&Path>, pictures: &Pictures) -> super::Page {
    body(text, beside, pictures).expect("a page")
}

/// A 1×1 PNG, as base64.
const DOT: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVQI12P4z8AAAAMBAQAY3Y2wAAAAAElFTkSuQmCC";

#[test]
fn formulas_and_diagrams_draw_as_svg() {
    let text =
        "Inline $x^2$ here.\n\n$$\n\\sqrt{y}\n$$\n\n```mermaid\nflowchart LR\n  A --> B\n```\n";
    let page = page(text, None, &Pictures::Inline);
    assert_eq!(page.html.matches("<svg").count(), 3, "{}", page.html);
    assert!(
        page.html.contains("<span class=\"math\"><svg")
            && page.html.contains("<div class=\"diagram\"><svg")
    );
    assert!(page.formulas.is_empty() && page.diagrams.is_empty());
}

#[test]
fn what_does_not_draw_stays_as_code_at_its_own_line() {
    let text = "$$\n\\frac{\n$$\n\n$$\n\\frac{\n$$\n\nText $\\frac{a}$ more.\n\n```mermaid\nflowchart LR\n  A -->\n```\n\n```mermaid\nflowchart LR\n  A[\u{F0000}] --> B\n```\n";
    let page = page(text, None, &Pictures::Inline);
    let lines: Vec<usize> = page.formulas.iter().map(|(line, _)| *line).collect();
    assert_eq!(lines, [1, 5, 9]);
    assert!(page.formulas.iter().all(|(_, why)| !why.is_empty()));
    assert_eq!(page.diagrams, [11, 16]);
    assert!(
        page.html
            .contains("<pre><code class=\"language-math\">\\frac{"),
        "{}",
        page.html
    );
    assert!(page.html.contains("<code>\\frac{a}</code>"), "{}", page.html);
    assert!(
        page.html
            .contains("<pre><code class=\"language-mermaid\">flowchart LR"),
        "{}",
        page.html
    );
}

#[test]
fn markdown_writes_as_html() {
    let text = "# T\n\n- [x] done\n- b\n\n3. c\n\n| l | r |\n|:--|--:|\n| 1 | 2 |\n\n~~gone~~ [a](https://x.org \"t\") [r][ref] [bad](javascript:alert(1))\n\nNote[^1].\n\n[^1]: The note.\n\n[ref]: /docs\n";
    let html = page(text, None, &Pictures::Inline).html;
    for part in [
        "<h1 id=\"h-0\">T</h1>",
        "<li><input type=\"checkbox\" disabled checked> done</li>",
        "<ol start=\"3\">",
        "<th style=\"text-align:left\">l</th>",
        "<td style=\"text-align:right\">2</td>",
        "<del>gone</del>",
        "<a href=\"https://x.org\">a</a>",
        "<a href=\"/docs\">r</a>",
        "<a>bad</a>",
        "<sup><a href=\"#fn-1\">1</a></sup>",
        "<div id=\"fn-1\" style=\"display:flex;gap:.5em;font-size:.9em;color:#59636e\"><span>[1]</span><div><p>The note.</p></div></div>",
    ] {
        assert!(html.contains(part), "{part} in {html}");
    }
}

#[test]
fn front_matter_and_raw_html_do_not_go_live() {
    let page = page(
        "---\ntitle: Secret\n---\n\n<script>alert(1)</script>\n\nText.\n",
        None,
        &Pictures::Inline,
    );
    assert!(!page.html.contains("Secret"), "{}", page.html);
    assert!(!page.html.contains("<script>"), "{}", page.html);
    assert!(page.html.contains("<p>Text.</p>"));
}

#[test]
fn inline_pictures_are_data_and_addresses_stay_as_written() {
    let dir = tempfile::tempdir().expect("a dir");
    std::fs::write(dir.path().join("dot.png"), b"png bytes").expect("writes");
    let text = format!(
        "![a](dot.png) ![b](gone.png) ![c](https://example.com/a%2Fb.png?q=a%26b&x=1) ![d](data:image/png;base64,{DOT})\n"
    );
    let page = page(&text, Some(dir.path()), &Pictures::Inline);
    assert!(
        page.html
            .contains("src=\"data:image/png;base64,cG5nIGJ5dGVz\""),
        "{}",
        page.html
    );
    assert!(
        page.html
            .contains("src=\"https://example.com/a%2Fb.png?q=a%26b&amp;x=1\""),
        "{}",
        page.html
    );
    assert!(
        page.html
            .contains(&format!("src=\"data:image/png;base64,{DOT}\"")),
        "{}",
        page.html
    );
    assert!(
        page.html.contains("<img src=\"gone.png\" alt=\"b\">"),
        "{}",
        page.html
    );
    assert_eq!(page.missing, ["gone.png"]);
}

#[test]
fn folder_pictures_copy_once_under_names_of_their_own() {
    let dir = tempfile::tempdir().expect("a dir");
    for (folder, name) in [("one", "image.png"), ("two", "IMAGE.PNG")] {
        std::fs::create_dir(dir.path().join(folder)).expect("makes");
        std::fs::write(dir.path().join(folder).join(name), folder).expect("writes");
    }
    let shelf = dir.path().join("note#1_files");
    let text = "![a](one/image.png) ![b](two/IMAGE.PNG) ![c](one/image.png)\n";
    let page = page(text, Some(dir.path()), &Pictures::Folder(shelf.clone()));
    let mut held: Vec<Vec<u8>> = std::fs::read_dir(&shelf)
        .expect("the folder")
        .map(|entry| std::fs::read(entry.expect("an entry").path()).expect("reads"))
        .collect();
    held.sort();
    assert_eq!(
        held,
        [b"one".to_vec(), b"two".to_vec()],
        "no picture overwrote another"
    );
    assert_eq!(
        page.html
            .matches("src=\"note%231_files/image.png\"")
            .count(),
        2,
        "{}",
        page.html
    );
}

#[test]
fn a_folder_already_there_is_refused() {
    let dir = tempfile::tempdir().expect("a dir");
    std::fs::write(dir.path().join("dot.png"), b"p").expect("writes");
    let shelf = dir.path().join("taken");
    std::fs::create_dir(&shelf).expect("makes");
    let error = body(
        "![a](dot.png)\n",
        Some(dir.path()),
        &Pictures::Folder(shelf),
    )
    .expect_err("refused");
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
}

#[test]
fn a_pdf_leaves_out_what_mupdf_cannot_read_as_words() {
    let dir = tempfile::tempdir().expect("a dir");
    std::fs::write(dir.path().join("a.webp"), b"w").expect("writes");
    let text = "![first](a.webp) ![second](https://example.com/c.png) ![third](gone.png)\n";
    let page = page(
        text,
        Some(dir.path()),
        &Pictures::Pdf(dir.path().join("files")),
    );
    assert_eq!(
        page.missing,
        ["a.webp", "https://example.com/c.png", "gone.png"]
    );
    assert!(!page.html.contains("<img"), "{}", page.html);
    assert!(
        page.html.contains("[first]") && page.html.contains("[third]"),
        "{}",
        page.html
    );
}

#[test]
fn other_schemes_and_untitled_pictures_are_missing() {
    let page = page("![a](a.png) ![b](javascript:x)\n", None, &Pictures::Inline);
    assert_eq!(page.missing, ["a.png", "javascript:x"]);
}

#[test]
fn a_file_address_reads_its_path() {
    let dir = tempfile::tempdir().expect("a dir");
    let picture = dir.path().join("d o t.png");
    std::fs::write(&picture, b"png").expect("writes");
    let path = picture
        .to_string_lossy()
        .replace('\\', "/")
        .replace(' ', "%20");
    let url = if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    };
    let page = page(&format!("![a](<{url}>)\n"), None, &Pictures::Inline);
    assert!(page.missing.is_empty(), "{:?}", page.missing);
    assert!(
        page.html.contains("data:image/png;base64,cG5n"),
        "{}",
        page.html
    );
}

#[test]
fn a_document_wraps_the_body_and_escapes_its_title() {
    let html = document("a<b & c", "<p>x</p>");
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<title>a&lt;b &amp; c</title>") && html.contains("<p>x</p>"));
}

#[test]
fn plain_text_keeps_words_code_and_tex() {
    let text = "---\ntitle: x\n---\n\n# Head *one*\n\n- a\n- b `c`\n\n| h | i |\n|---|---|\n| 1 | 2 |\n\n$$\n\\sqrt{x}\n$$\n\n> quoted $y$ ![a cat][cat]\n\n[cat]: pic.png\n";
    assert_eq!(
        plain(text).expect("text"),
        "Head one\n\na\nb c\n\nh\ti\n1\t2\n\n\\sqrt{x}\n\nquoted y a cat\n"
    );
}

mod files {
    use std::path::Path;

    use super::super::{Format, Target, write};
    use super::DOT;

    fn target<'a>(path: &'a Path, work: &'a Path) -> Target<'a> {
        Target {
            path,
            session: None,
            revision: 1,
            work,
        }
    }

    #[test]
    fn a_page_carries_its_pictures_inside() {
        let dir = tempfile::tempdir().expect("a dir");
        std::fs::write(dir.path().join("dot.png"), b"png").expect("writes");
        let out = dir.path().join("out.html");
        write(
            Format::Page,
            "# Hi\n\n![d](dot.png)\n",
            Some(dir.path()),
            &target(&out, dir.path()),
        )
        .expect("writes");
        let html = std::fs::read_to_string(&out).expect("reads");
        assert!(html.starts_with("<!doctype html>") && html.contains("<title>out</title>"));
        assert!(html.contains("data:image/png;base64,"), "{html}");
    }

    #[test]
    fn a_linked_page_copies_into_a_new_folder_only() {
        let dir = tempfile::tempdir().expect("a dir");
        std::fs::write(dir.path().join("dot.png"), b"png").expect("writes");
        let out = dir.path().join("out.html");
        write(
            Format::Linked,
            "![d](dot.png)\n",
            Some(dir.path()),
            &target(&out, dir.path()),
        )
        .expect("writes");
        assert_eq!(
            std::fs::read(dir.path().join("out_files/dot.png")).expect("copied"),
            b"png"
        );
        assert!(
            std::fs::read_to_string(&out)
                .expect("reads")
                .contains("src=\"out_files/dot.png\"")
        );
        std::fs::remove_file(&out).expect("removes");
        let error = write(
            Format::Linked,
            "![d](dot.png)\n",
            Some(dir.path()),
            &target(&out, dir.path()),
        )
        .expect_err("the folder is there");
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert!(error.to_string().contains("out_files"), "{error}");
        assert!(!out.exists(), "nothing written");
        assert!(
            dir.path().join("out_files/dot.png").exists(),
            "the folder found stays"
        );
    }

    #[test]
    fn a_linked_page_without_pictures_makes_no_folder() {
        let dir = tempfile::tempdir().expect("a dir");
        let out = dir.path().join("out.html");
        write(
            Format::Linked,
            "Text.\n",
            Some(dir.path()),
            &target(&out, dir.path()),
        )
        .expect("writes");
        assert!(!dir.path().join("out_files").exists());
    }

    #[test]
    fn text_is_plain_and_the_open_file_is_never_written() {
        let dir = tempfile::tempdir().expect("a dir");
        let out = dir.path().join("doc.md");
        std::fs::write(&out, "# Kept\n").expect("writes");
        let mut aimed = target(&out, dir.path());
        aimed.session = Some(&out);
        write(Format::Text, "# Other\n", None, &aimed).expect_err("refused");
        assert_eq!(std::fs::read_to_string(&out).expect("reads"), "# Kept\n");
        let txt = dir.path().join("doc.txt");
        write(
            Format::Text,
            "# Title *here*\n",
            None,
            &target(&txt, dir.path()),
        )
        .expect("writes");
        assert_eq!(
            std::fs::read_to_string(&txt).expect("reads"),
            "Title here\n"
        );
    }

    /// The text of a PDF's first page.
    fn words(pdf: &Path) -> String {
        let mut worker = ely_pdf_engine::Worker::spawn().expect("the worker");
        let doc = worker.open(pdf).expect("opens").doc;
        worker
            .text(doc, 0)
            .expect("text")
            .iter()
            .map(|ch| ch.c)
            .collect()
    }

    #[test]
    fn a_pdf_sets_the_page_and_leaves_no_scratch() {
        let dir = tempfile::tempdir().expect("a dir");
        let work = tempfile::tempdir().expect("work");
        let out = dir.path().join("out.pdf");
        let text = format!(
            "# Heading 标题\n\nInline $x^2$.\n\n![dot](data:image/png;base64,{DOT}) ![lost](gone.png)\n\n```mermaid\nflowchart LR\n  A --> B\n```\n"
        );
        let page = write(
            Format::Pdf,
            &text,
            Some(dir.path()),
            &target(&out, work.path()),
        )
        .expect("writes");
        assert_eq!(page.missing, ["gone.png"]);
        assert!(page.formulas.is_empty() && page.diagrams.is_empty());
        assert_eq!(
            std::fs::read_dir(work.path()).expect("work").count(),
            0,
            "the scratch is gone"
        );
        let words = words(&out);
        assert!(
            words.contains("Heading") && words.contains("标题") && words.contains("[lost]"),
            "{words:?}"
        );
    }

    #[test]
    fn pdfs_at_once_keep_to_their_own_pages() {
        let (dir, work) = (
            tempfile::tempdir().expect("a dir"),
            tempfile::tempdir().expect("work"),
        );
        let jobs: Vec<_> = (0..4)
            .map(|n| {
                let (out, work) = (
                    dir.path().join(format!("{n}.pdf")),
                    work.path().to_path_buf(),
                );
                std::thread::spawn(move || {
                    write(
                        Format::Pdf,
                        &format!("# Page{n}\n"),
                        None,
                        &target(&out, &work),
                    )
                    .expect("writes");
                    out
                })
            })
            .collect();
        for (n, job) in jobs.into_iter().enumerate() {
            let out = job.join().expect("the job");
            assert!(words(&out).contains(&format!("Page{n}")), "job {n}");
        }
    }
}

#[test]
fn a_disguised_script_link_does_not_follow() {
    let text = "[a](< javascript:alert(1)>) [b](java&#x09;script:alert(1)) [c](java&#10;script:alert(1)) [d](< https://x.org>)\n";
    let html = page(text, None, &Pictures::Inline).html;
    assert_eq!(html.matches("<a>").count(), 3, "{html}");
    assert!(html.contains("href=\" https://x.org\""), "{html}");
}

#[test]
fn a_failed_linked_page_takes_the_folder_it_made() {
    let dir = tempfile::tempdir().expect("a dir");
    std::fs::write(dir.path().join("dot.png"), b"png").expect("writes");
    // A folder in the way: no page.
    let out = dir.path().join("out.html");
    std::fs::create_dir(&out).expect("makes");
    let target = super::Target {
        path: &out,
        session: None,
        revision: 1,
        work: dir.path(),
    };
    super::write(
        super::Format::Linked,
        "![d](dot.png)\n",
        Some(dir.path()),
        &target,
    )
    .expect_err("fails");
    assert!(
        !dir.path().join("out_files").exists(),
        "the folder it made is gone"
    );
}

#[test]
fn a_formula_letter_no_font_has_stays_as_tex() {
    let page = page(
        "Before\n\n$\\text{\u{F0000}}$ after.\n",
        None,
        &Pictures::Inline,
    );
    let lines: Vec<usize> = page.formulas.iter().map(|(line, _)| *line).collect();
    assert_eq!(lines, [3]);
    assert!(
        page.html.contains("<code>\\text{\u{F0000}}</code>"),
        "{}",
        page.html
    );
}

#[test]
fn a_large_document_exports_whole() {
    // Long paragraphs: markdown-rs slows with the block count.
    let words = "word ".repeat(200);
    let mut text: String = (0..2_200)
        .map(|n| format!("Paragraph {n}: {words}\n\n"))
        .collect();
    text.push_str("The last line.\n");
    assert!(text.len() > ely_gpui_component::documents::LARGEST_READ);
    assert!(plain(&text).expect("text").ends_with("The last line.\n"));
    assert!(
        page(&text, None, &Pictures::Inline)
            .html
            .ends_with("<p>The last line.</p>")
    );
}
