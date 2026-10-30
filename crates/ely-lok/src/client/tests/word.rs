//! Gitea #9: Word's EQ fields and lowered inline objects.

use super::input::saved;
use super::*;

/// `tests/fixtures/office/word.docx` open in a fresh worker.
fn word() -> (tempfile::TempDir, Worker, Doc) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let install = install().expect("a pinned LOK build");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/office/word.docx");
    let doc = worker.load(&crate::file_url(&fixture)).expect("the document loads");
    (dir, worker, doc)
}

/// The text position of the span around the picture's frame.
fn picture_position(fodt: &str) -> String {
    let named = fodt.find("draw:name=\"red\"").expect("the picture");
    let frame = fodt[..named].rfind("<draw:frame").expect("the picture's frame");
    let open = fodt[..frame].rfind("<text:span").expect("a span around the picture");
    let style = fodt[open..].split('"').nth(1).expect("the span's style");
    let named = format!("style:name=\"{style}\"");
    let at = fodt.find(&named).expect("the span's style is written");
    let props = &fodt[at..at + fodt[at..].find("</style:style>").expect("the style ends")];
    let position = props.find("style:text-position=\"").expect("the span is raised or lowered");
    props[position + 21..].split('"').next().expect("a position").to_owned()
}

#[test]
fn eq_fields_open_as_formulas() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = word();
    let fodt = saved(&dir, &mut worker, &doc, "word.fodt");
    assert!(
        fodt.contains("{{a}_{&quot;10&quot;}} over {{a}_{&quot;3&quot;}}"),
        "the fraction keeps its italics and subscripts"
    );
    assert!(fodt.contains("nroot{&quot;3&quot;}{&quot;x&quot;}"), "the radical");
    assert!(fodt.contains("sum from{&quot;1&quot;} to{&quot;5&quot;}"), "an uppercase \\SU is a sum");
    assert!(
        fodt.contains("matrix{{&quot;a&quot;} # {&quot;b&quot;} ## {&quot;c&quot;} # {&quot;d&quot;}}"),
        "an uppercase \\CO2 makes two columns"
    );
    assert!(fodt.contains("eq \\f(a)</text:span>"), "an unreadable field shows its code");
    assert!(fodt.contains("eq \\b\\lc\\★(x)"), "an unknown bracket shows its code");
    let short = formulas(&fodt).into_iter().find(|math| math.contains("# {}}")).expect("the short array");
    assert!(short.contains(">c<"), "a short last row keeps its operand");
    assert_eq!(fodt.matches('∑').count(), 2, "\\SU and \\fc\\∑ both draw a sum");
    assert!(fodt.contains('⌊') && fodt.contains('⌋'), "floor brackets draw");
    assert!(!fodt.contains("MERGEFORMAT"), "a general switch is no formula text");
    assert!(fodt.contains("{&quot;m&quot;} over {&quot;n&quot;}<"), "nothing after the fraction");
    assert_eq!(subscripts(&fodt, ">p<"), 1, "a style change keeps one subscript");
    assert!(fodt.contains("&quot;u&quot; stack{{&quot;v&quot;}}"), "\\ai only spaces lines");
    assert!(fodt.contains("&quot;w&quot; stack{{&quot;k&quot;}}"), "\\di only spaces lines");
    assert!(fodt.contains("_{{&quot;g&quot;}} &quot;+h&quot;"), "a lowercase \\s\\do keeps its tail");
    assert!(fodt.contains("_{{{&quot;r&quot;} over {&quot;s&quot;}}}"), "and its nested fraction");
    assert!(fodt.contains(">lowered</text:span>"), "a plain \\s\\do stays lowered text");
}

#[test]
fn eq_formulas_survive_docx() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = word();
    let docx = dir.path().join("again.docx");
    doc.save_as(&mut worker, &crate::file_url(&docx), "docx").expect("the DOCX saves");
    let back = worker.load(&crate::file_url(&docx)).expect("the DOCX reopens");
    let fodt = saved(&dir, &mut worker, &back, "again.fodt");
    assert!(
        fodt.contains("{{a} rsub {&quot;10&quot;}} over {{a} rsub {&quot;3&quot;}}"),
        "the italic letters stay italic"
    );
    assert!(!fodt.contains("mathvariant=\"normal\">a<"), "no upright a");
    assert_eq!(subscripts(&fodt, ">p<"), 1, "one subscript still");
    assert!(fodt.contains("{&quot;&quot;} rsub {&quot;g&quot;} &quot;+h&quot;"), "the tail survives");
    assert!(!fodt.contains("&lt;?&gt;"), "no placeholder for a missing base");
}

/// Subscripts in the formula holding `mark`.
fn subscripts(fodt: &str, mark: &str) -> usize {
    formulas(fodt).into_iter().find(|math| math.contains(mark)).expect("the formula").matches("<msub>").count()
}

/// Each embedded formula's MathML.
fn formulas(fodt: &str) -> Vec<&str> {
    fodt.split("<math xmlns").skip(1).map(|math| &math[..math.find("</math>").expect("closed")]).collect()
}

#[test]
fn a_lowered_picture_moves_with_its_run_and_keeps_it() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = word();
    let fodt = saved(&dir, &mut worker, &doc, "word.fodt");
    // 11 points down, against the 11 point anchor.
    assert_eq!(picture_position(&fodt), "-100% 100%");
    let docx = dir.path().join("again.docx");
    doc.save_as(&mut worker, &crate::file_url(&docx), "docx").expect("the DOCX saves");
    let back = worker.load(&crate::file_url(&docx)).expect("the DOCX reopens");
    let again = saved(&dir, &mut worker, &back, "again.fodt");
    assert_eq!(picture_position(&again), "-100% 100%", "the position survives DOCX");
    // The square's red sits below the line's text.
    let tile = back.paint_tile(&mut worker, (1440, 1440, 3840, 1920), (256, 128)).expect("a tile");
    let rows = |hit: fn(&[u8]) -> bool| {
        let mut lit: Vec<usize> = tile.pixels.as_chunks::<4>().0.iter().enumerate().filter(|(_, px)| hit(*px)).map(|(at, _)| at / 256).collect();
        lit.dedup();
        let first = *lit.first().expect("some pixels");
        let line = lit.iter().zip(first..).take_while(|(row, want)| **row <= *want).last().expect("a row");
        (first, *line.0)
    };
    let red = rows(|px| px[2] > 200 && px[1] < 60 && px[0] < 60);
    let ink = rows(|px| px[0] < 80 && px[1] < 80 && px[2] < 80);
    assert!(red.1 > ink.1 + 4, "the picture hangs below the text: red {red:?}, text {ink:?}");
}
