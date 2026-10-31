//! Exports read the file as the visual mode does (R4.4).

use super::{Pictures, page, plain};
use crate::output::{Page, html::write, markdown_tree, pictures::Placed};

fn html(text: &str) -> String {
    page(text, None, &Pictures::Inline).html
}

fn text(text: &str) -> String {
    plain(text).expect("plain text")
}

#[test]
fn shortcodes_turn_as_github_turns_them() {
    let page = html("Go :smile: x:smile:\n\nx:smile: :smile:\n\na <code>:smile:</code> :smile:\n");
    for part in [
        "<p>Go 😄 x😄</p>",
        "<p>x:smile: :smile:</p>",
        "<p>a <code>:smile:</code> 😄</p>",
    ] {
        assert!(page.contains(part), "{part} in {page}");
    }
    assert_eq!(
        text("Go :smile:\n\nx:smile: :smile:\n"),
        "Go 😄\n\nx:smile: :smile:\n"
    );
}

#[test]
fn inline_tags_keep_what_they_mean_and_nothing_runs() {
    let source = "a <u>u</u> <mark>m</mark> <kbd>k</kbd> <span>s</span> <script>x</script> \
        <a href=\"javascript:x\">j</a> <a href=\"https://e.test\">e</a>\n";
    let page = html(source);
    for part in [
        "<u>u</u>",
        "<mark>m</mark>",
        "<code>k</code>",
        " s ",
        "&lt;script&gt;x&lt;/script&gt;",
        "<a>j</a>",
        "<a href=\"https://e.test\">e</a>",
    ] {
        assert!(page.contains(part), "{part} in {page}");
    }
    assert!(
        !page.contains("<span") && !page.contains("<script"),
        "{page}"
    );
    assert_eq!(
        text("a <b>b</b> <script>x</script>\n"),
        "a b <script>x</script>\n"
    );
}

#[test]
fn an_html_block_shows_its_text() {
    let source = "<div align=\"center\">\n  <b>Bold</b>   and :smile:\n</div>\n";
    let page = html(source);
    assert!(
        page.contains("<p><strong>Bold</strong> and 😄</p>"),
        "{page}"
    );
    assert!(!page.contains("<div"), "{page}");
    assert_eq!(text(source), "Bold and 😄\n");
}

#[test]
fn an_alert_writes_its_kind_and_title() {
    let page = html("> [!NOTE]\n> Read this.\n\n- > [!TIP]\n  > Not here.\n");
    let alert = "<div style=\"padding:.25em 1em;margin:1em 0;border-left:.25em solid #0969da\"><p style=\"font-weight:600;color:#0969da\">Note</p><p>Read this.</p></div>";
    assert!(page.contains(alert), "{page}");
    assert!(!page.contains("#1a7f37"), "only root quotes alert: {page}");
    assert_eq!(text("> [!WARNING]\n> Hot.\n"), "Warning\n\nHot.\n");
}

#[test]
fn a_toc_lists_the_headings_and_links_them() {
    let page = html("[toc]\n\n# A\n\n## B :smile:\n");
    let toc = "<ul style=\"list-style:none;padding-left:0\"><li style=\"margin-left:0em\"><a href=\"#h-7\">A</a></li><li style=\"margin-left:1em\"><a href=\"#h-12\">B 😄</a></li></ul>";
    assert!(page.contains(toc), "{page}");
    assert!(page.contains("<h1 id=\"h-7\">A</h1>") && page.contains("<h2 id=\"h-12\">B 😄</h2>"));
    assert_eq!(text("[toc]\n\n# A\n\n## B\n"), "A\nB\n\nA\n\nB\n");
}

#[test]
fn tags_crossing_containers_read_as_drawn() {
    for (source, shown) in [
        ("*a <code>b* :smile:</code> :smile:\n", "a b :smile: 😄"),
        ("a <code>*b</code>* :smile:\n", "a b 😄"),
    ] {
        assert_eq!(text(source), format!("{shown}\n"));
        let page = html(source);
        assert_eq!(page.matches("😄").count(), 1, "{page}");
        assert_eq!(
            page.matches(":smile:").count(),
            shown.matches(":smile:").count(),
            "{page}"
        );
    }
}

#[test]
fn each_cell_and_block_reads_on_its_own() {
    let page = html("| x:smile: | :smile: |\n| --- | --- |\n| :smile: | :smile: |\n");
    assert_eq!(page.matches("😄").count(), 3, "{page}");
    let page = html("# a <code>x\n\n:smile:\n");
    assert!(page.contains("<p>😄</p>"), "{page}");
}

#[test]
fn a_block_closes_the_tags_left_open_in_it() {
    let page = html("a <b>bold\n\nnormal\n");
    assert!(
        page.contains("<p>a <strong>bold</strong></p><p>normal</p>"),
        "{page}"
    );
    let page = html("a <a href=\"https://example.test\">link\n\nnormal\n");
    assert!(page.ends_with("<p>normal</p>"), "{page}");
}

#[test]
fn every_toc_entry_has_its_heading() {
    let page = html("[toc]\n\n# Main\n\n[^n]:\n    ## Unused\n");
    let targets: Vec<&str> = page
        .split("href=\"#")
        .skip(1)
        .map(|rest| &rest[..rest.find('"').expect("a quote")])
        .collect();
    assert_eq!(targets.len(), 2, "{page}");
    for target in targets {
        assert!(
            page.contains(&format!("id=\"{target}\"")),
            "{target} in {page}"
        );
    }
}

/// The quickest of five: load only ever adds time.
fn fastest(mut time: impl FnMut() -> std::time::Duration) -> std::time::Duration {
    (0..5).map(|_| time()).min().expect("five runs")
}

#[test]
fn a_long_html_block_writes_in_linear_time() {
    let took = |count: usize| {
        let source = format!("<div>{}</div>\n", "<b>x</b> ".repeat(count));
        fastest(|| {
            let start = std::time::Instant::now();
            html(&source);
            start.elapsed()
        })
    };
    let (small, large) = (took(8_000), took(32_000));
    // Four times the input; squared would be sixteen.
    assert!(large < small * 8, "{small:?} then {large:?}");
}

#[test]
fn notes_under_deep_marks_write_in_linear_time() {
    let took = |count: usize| {
        let source = format!(
            "start {}{}

[^n]: note
",
            "<b>".repeat(count),
            "x[^n] ".repeat(count)
        );
        let tree = markdown_tree(&source);
        fastest(|| {
            let mut placed = Placed::new(None, &Pictures::Inline);
            let start = std::time::Instant::now();
            write(&tree, &source, &mut placed, &mut Page::default()).expect("written");
            start.elapsed()
        })
    };
    // Parsing grows squared upstream; untimed.
    let (small, large) = (took(4_000), took(16_000));
    assert!(large < small * 8, "{small:?} then {large:?}");
}

#[test]
fn a_blank_formula_is_dropped_with_its_text() {
    for source in [
        "$ $\n",
        "$ $ ![p](p.png)\n",
        "| $ $ | a |\n| - | - |\n| b | c |\n",
    ] {
        html(source);
    }
    let page = page("$ $ ![p](p.png)\nnext $\\frac{a}$\n", None, &Pictures::Inline);
    let lines: Vec<usize> = page.formulas.iter().map(|(line, _)| *line).collect();
    assert_eq!(lines, [2], "{}", page.html);
}

#[test]
fn a_note_in_a_link_keeps_the_link_whole() {
    let page = html("a <a href=\"https://example.test\">b[^n] c</a> after\n\n[^n]: note\n");
    assert!(
        page.contains("<a href=\"https://example.test\">b<sup>1</sup> c</a>"),
        "{page}"
    );
}

#[test]
fn a_reference_reaches_its_own_note() {
    let page = html("[^1]: unused\n\nCall[^used].\n\n[^used]: right target\n");
    assert_eq!(page.matches("id=\"fn-1\"").count(), 1, "{page}");
    let at = page.find("id=\"fn-1\"").expect("an anchor");
    assert!(
        page[at..].contains("right target") && !page[at..].contains("unused"),
        "{page}"
    );
}

#[test]
fn plain_notes_keep_the_labels_drawn_beside_them() {
    assert_eq!(
        text("A[^b] B[^a].\n\n[^a]: Alpha\n\n[^b]: Beta\n"),
        "A\u{b9} B\u{b2}.\n\n[2] Alpha\n\n[1] Beta\n"
    );
    assert_eq!(text("x\n\n[^n]: note\n"), "x\n\n[n] note\n");
}

#[test]
fn a_copied_fragment_carries_its_look() {
    let page = html("> [!NOTE]\n> Read this.\n\n[toc]\n\n# Title\n\nCall[^n].\n\n[^n]: note\n");
    for part in [
        "border-left:.25em solid #0969da",
        "font-weight:600",
        "list-style:none",
        "display:flex",
    ] {
        assert!(page.contains(part), "{part} in {page}");
    }
    assert!(
        !page.contains("class="),
        "the look needs no page style: {page}"
    );
}

#[test]
fn a_soft_break_joins_in_exports_as_typora_exports_it() {
    let page = html("你好\n测试  \nend\n");
    assert!(page.contains("<p>你好 测试<br>end</p>"), "{page}");
    assert_eq!(text("你好\n测试\n"), "你好 测试\n");
}

#[test]
fn tex_delimiters_export_as_formulas() {
    let source = r"a \(x^2\) b

\[
y+1
\]
";
    let page = page(source, None, &Pictures::Inline);
    assert_eq!(page.html.matches("<svg").count(), 2, "{}", page.html);
    assert!(page.formulas.is_empty(), "{:?}", page.formulas);
    assert_eq!(text(source), "a x^2 b\n\ny+1\n");
}

#[test]
fn a_loose_list_keeps_its_paragraphs_and_a_tight_one_drops_them() {
    assert!(html("- a\n- b\n").contains("<li>a</li><li>b</li>"));
    assert!(html("- a\n\n- b\n").contains("<li><p>a</p></li><li><p>b</p></li>"));
}

#[test]
fn a_formula_line_counts_crlf_once_and_a_lone_cr_as_a_line() {
    for end in ["\n", "\r\n", "\r"] {
        let text = ["one", "", "two", "", r"x $\frac{a}$"].join(end);
        let page = page(&text, None, &Pictures::Inline);
        let lines: Vec<usize> = page.formulas.iter().map(|(line, _)| *line).collect();
        assert_eq!(lines, [5], "{end:?}");
    }
}

#[test]
fn cells_past_the_head_stay_out_as_they_stay_undrawn() {
    let text = "| a | b |\n| - | - |\n| 1 | 2 | 3 |\n";
    let page = html(text);
    assert!(page.contains("<tr><td>1</td><td>2</td></tr>"), "{page}");
    assert!(!page.contains('3'), "{page}");
    assert_eq!(plain(text).expect("plain"), "a\tb\n1\t2\n");
}

#[test]
fn formulas_far_down_a_page_write_in_linear_time() {
    let took = |count: usize| {
        let filler = "word ".repeat(40);
        let source = format!("x $\\frac{{a}}$ {filler}\n\n").repeat(count);
        let tree = markdown_tree(&source);
        fastest(|| {
            let mut placed = Placed::new(None, &Pictures::Inline);
            let start = std::time::Instant::now();
            write(&tree, &source, &mut placed, &mut Page::default()).expect("written");
            start.elapsed()
        })
    };
    let (small, large) = (took(1_000), took(4_000));
    assert!(large < small * 8, "{small:?} then {large:?}");
}
