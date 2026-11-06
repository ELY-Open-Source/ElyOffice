use crate::markup::html;

#[test]
fn raw_html_and_unsafe_links_stay_text() {
    let made = html("**Yes** <script>x()</script> [go](javascript:x) ![p](data:x)");
    assert!(made.contains("<strong>Yes</strong>"), "{made}");
    assert!(made.contains("&lt;script&gt;x()&lt;/script&gt;"), "{made}");
    assert!(!made.contains("javascript:") && !made.contains("data:"), "{made}");
    assert!(made.contains(" go p</p>"), "{made}");
}

#[test]
fn bare_addresses_link_as_the_composer_shows() {
    let made = html("See https://ex.am/ple and www.ex.am now");
    assert!(made.contains("<a href=\"https://ex.am/ple\">https://ex.am/ple</a>"), "{made}");
    assert!(made.contains("<a href=\"http://www.ex.am\">www.ex.am</a>"), "{made}");
}

#[test]
fn lists_tables_references_and_notes() {
    let words = "# Plan\n\n3. three\n4. four\n\n- [x] done\n- [ ] not\n\n\
                 | a | b |\n|:--|--:|\n| 1 | 2 |\n\n\
                 ~~old~~ [site][s] and a note[^n].\n\n[s]: https://s.ex\n[^n]: The *note*.\n";
    let made = html(words);
    for part in [
        "<h1>Plan</h1>",
        "<ol start=\"3\">\n<li>three</li>",
        "<li>☑ done</li>\n<li>☐ not</li>",
        "<th style=\"text-align:left\">a</th><th style=\"text-align:right\">b</th>",
        "<td style=\"text-align:left\">1</td>",
        "<del>old</del> <a href=\"https://s.ex\">site</a>",
        "a note<sup>1</sup>.",
        "<hr>\n<ol>\n<li>The <em>note</em>.</li>",
    ] {
        assert!(made.contains(part), "{part} not in {made}");
    }
}

#[test]
fn code_and_formulas_keep_their_text() {
    let made = html("```rust\nlet a = 1 < 2;\n```\n\n$$\nx^2\n$$\n\nand $y$\n");
    assert!(made.contains("<pre><code class=\"language-rust\">let a = 1 &lt; 2;</code></pre>"), "{made}");
    assert!(made.contains("<pre>$$\nx^2\n$$</pre>"), "{made}");
    assert!(made.contains("and $y$"), "{made}");
}
