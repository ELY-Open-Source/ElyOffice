use super::input::{open, saved};
use super::*;

const SETTLE: std::time::Duration = std::time::Duration::from_secs(10);

/// The latest value of `uno` seen so far.
fn state<'a>(seen: &'a Events, uno: &str) -> Option<&'a str> {
    seen.states
        .iter()
        .rev()
        .find(|(name, _)| name == uno)
        .map(|(_, value)| value.as_str())
}

fn run(worker: &mut Worker, doc: &Doc, id: &str, values: &[&str]) {
    worker
        .run(doc, id, values)
        .unwrap_or_else(|error| panic!("{id}: {error}"));
}

/// Lets the kernel settle a selection, as a person does.
fn pause(worker: &mut Worker, doc: &Doc) {
    poll_until(
        worker,
        doc,
        &Events::default(),
        std::time::Duration::from_millis(500),
        |_| false,
    );
}

/// A solid PNG on disk, as a file URL.
pub(super) fn png(dir: &std::path::Path, name: &str, rgba: [u8; 4]) -> String {
    let mut bytes = Vec::new();
    image::RgbaImage::from_pixel(300, 200, image::Rgba(rgba))
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .expect("the png encodes");
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("the png writes");
    crate::file_url(&path)
}

#[test]
fn page_setup_lands_and_reports() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    run(&mut worker, &doc, "page.margins_lr", &["1270", "2540"]);
    run(&mut worker, &doc, "page.margins_ul", &["2540", "1270"]);
    run(&mut worker, &doc, "page.columns", &["2"]);
    run(&mut worker, &doc, "page.size", &["21000", "29700"]);
    run(
        &mut worker,
        &doc,
        "page.orientation",
        &["29700", "21000", "true", "0", "4"],
    );
    let want = [
        (".uno:PageLRMargin", "1270,2540"),
        (".uno:PageULMargin", "2540,1270"),
        (".uno:PageColumnType", "2"),
        (".uno:AttributePage", "true,0,4"),
        (".uno:AttributePageSize", "29700 x 21001"),
    ];
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        want.iter()
            .all(|(uno, value)| state(seen, uno) == Some(*value))
    });
    for (uno, value) in want {
        assert_eq!(state(&seen, uno), Some(value), "{uno}");
    }
    let text = saved(&dir, &mut worker, &doc, "page.fodt");
    for mark in [
        "fo:page-width=\"11.6929in\" fo:page-height=\"8.2681in\"",
        "style:print-orientation=\"landscape\"",
        "fo:margin-top=\"1in\" fo:margin-bottom=\"0.5in\" fo:margin-left=\"0.5in\" fo:margin-right=\"1in\"",
        "fo:column-count=\"2\"",
    ] {
        assert!(text.contains(mark), "{mark} lands");
    }
}

#[test]
fn headers_footers_and_sections_land() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    run(&mut worker, &doc, "header.on", &["true"]);
    run(&mut worker, &doc, "footer.on", &["true"]);
    run(&mut worker, &doc, "header.layout", &["3"]);
    run(&mut worker, &doc, "field.page", &[]);
    run(&mut worker, &doc, "field.pages", &[]);
    let want = [
        (".uno:SwPageHeaderItem", "true"),
        (".uno:SwPageFooterItem", "true"),
        (".uno:SwPageHeaderLayout", "3"),
    ];
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        want.iter()
            .all(|(uno, value)| state(seen, uno) == Some(*value))
    });
    for (uno, value) in want {
        assert_eq!(state(&seen, uno), Some(value), "{uno}");
    }
    run(&mut worker, &doc, "page.section", &["3", "Landscape"]);
    let section = poll_until(&mut worker, &doc, &seen, HANG, |seen| {
        state(seen, ".uno:PageStyleName") == Some("Landscape")
    });
    assert_eq!(state(&section, ".uno:PageStyleName"), Some("Landscape"));
    run(&mut worker, &doc, "header.on", &["false"]);
    let text = saved(&dir, &mut worker, &doc, "header.fodt");
    for mark in [
        "<style:header>",
        "<style:header-left>",
        "<style:header-first>",
        "<style:footer>",
        "style:master-page-name=\"Landscape\"",
        "<text:page-number",
        "<text:page-count",
    ] {
        assert!(text.contains(mark), "{mark} lands");
    }
    let landscape = text
        .split("<style:master-page style:name=\"Landscape\"")
        .nth(1)
        .and_then(|rest| rest.split("</office:master-styles>").next())
        .expect("the section's page style");
    assert!(
        !landscape.contains("<style:header>"),
        "the section's header is off"
    );
}

#[test]
fn tables_edit_and_land() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker.post_key(&doc, 0, 1029).expect("end");
    worker.post_key(&doc, 13, 1280).expect("enter");
    run(&mut worker, &doc, "table.insert", &["3", "2"]);
    let inside = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        state(seen, ".uno:DeleteTable") == Some("enabled")
    });
    assert_eq!(
        state(&inside, ".uno:DeleteTable"),
        Some("enabled"),
        "in a table"
    );
    run(&mut worker, &doc, "table.repeat_heading", &[]);
    for (id, values) in [
        ("table.row_below", vec![]),
        ("table.row_above", vec![]),
        ("table.delete_rows", vec![]),
        ("table.column_right", vec![]),
        ("table.column_left", vec![]),
        ("table.delete_columns", vec![]),
        ("table.shading", vec!["16776960"]),
        ("table.row_split", vec!["false"]),
        ("table.split", vec!["2", "false"]),
        ("table.select", vec![]),
        ("table.borders_all", vec![]),
        ("table.even_columns", vec![]),
        ("table.optimal_height", vec![]),
        ("table.even_rows", vec![]),
    ] {
        run(&mut worker, &doc, id, &values);
    }
    let repeated = poll_until(&mut worker, &doc, &inside, HANG, |seen| {
        state(seen, ".uno:HeadingRowsRepeat") == Some("true")
    });
    assert_eq!(state(&repeated, ".uno:HeadingRowsRepeat"), Some("true"));
    let text = saved(&dir, &mut worker, &doc, "table.fodt");
    for mark in [
        "<table:table-header-rows>",
        "fo:background-color=\"#ffff00\"",
        "fo:keep-together=\"always\"",
        "fo:border=\"0.5pt solid #000000\"",
        "table:number-columns-spanned=\"2\"",
    ] {
        assert!(text.contains(mark), "{mark} lands");
    }
    run(&mut worker, &doc, "table.borders_none", &[]);
    run(&mut worker, &doc, "table.merge", &[]);
    run(&mut worker, &doc, "table.delete", &[]);
    let text = saved(&dir, &mut worker, &doc, "gone.fodt");
    assert!(!text.contains("<table:table "), "the table is gone");
}

#[test]
fn images_insert_scale_wrap_crop_and_stack() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    let (blue, red, green) = (
        png(dir.path(), "blue.png", [0, 0, 255, 255]),
        png(dir.path(), "red.png", [255, 0, 0, 255]),
        png(dir.path(), "green.png", [0, 255, 0, 255]),
    );
    click(&mut worker, &doc, 150, 150);
    run(&mut worker, &doc, "image.insert", &[&blue]);
    run(&mut worker, &doc, "image.insert", &[&red]);
    pause(&mut worker, &doc);
    run(&mut worker, &doc, "image.size", &["2835", "1890"]);
    run(&mut worker, &doc, "image.anchor_paragraph", &[]);
    run(&mut worker, &doc, "image.wrap_through", &[]);
    let want = [
        (".uno:SetAnchorToPara", "true"),
        (".uno:WrapThrough", "true"),
    ];
    let framed = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        want.iter()
            .all(|(uno, value)| state(seen, uno) == Some(*value))
    });
    for (uno, value) in want {
        assert_eq!(state(&framed, uno), Some(value), "{uno}");
    }
    let (x, y, w, h) = framed.graphic.expect("the image is selected");
    worker.post_key(&doc, 0, 1281).expect("escape");
    let cleared = poll_until(&mut worker, &doc, &framed, HANG, |seen| {
        seen.graphic.is_none()
    });
    click(&mut worker, &doc, x + w / 2, y + h / 2);
    let picked = poll_until(&mut worker, &doc, &cleared, HANG, |seen| {
        seen.graphic.is_some()
    });
    assert!(picked.graphic.is_some(), "the click selects the image");
    pause(&mut worker, &doc);
    run(&mut worker, &doc, "image.crop", &[]);
    pause(&mut worker, &doc);
    worker
        .drag_graphic(&doc, (x + w, y + h), (x + w - w / 3, y + h - h / 3))
        .expect("the handle drags");
    let text = saved(&dir, &mut worker, &doc, "image.fodt");
    assert_eq!(
        text.matches("<draw:frame").count(),
        1,
        "the insert replaced it"
    );
    assert!(text.contains("style:wrap=\"run-through\""), "wrap lands");
    assert!(
        text.contains("text:anchor-type=\"paragraph\""),
        "anchor lands"
    );
    assert!(
        !text.contains("fo:clip=\"rect(0in, 0in, 0in, 0in)\""),
        "the drag crops"
    );
    worker.post_key(&doc, 0, 1281).expect("escape");
    click(&mut worker, &doc, 150, 150);
    poll_until(&mut worker, &doc, &framed, SETTLE, |_| false);
    run(&mut worker, &doc, "image.insert", &[&green]);
    pause(&mut worker, &doc);
    run(&mut worker, &doc, "image.wrap_through", &[]);
    run(&mut worker, &doc, "image.back", &[]);
    let text = saved(&dir, &mut worker, &doc, "stack.fodt");
    let frames: Vec<&str> = text.split("<draw:frame").skip(1).collect();
    assert_eq!(frames.len(), 2, "two images");
    let back = frames
        .iter()
        .find(|frame| frame.contains("draw:z-index=\"0\""))
        .expect("one at the back");
    assert!(back.contains("Image2"), "the new image went back");
}

#[test]
fn the_remaining_layout_rows_reach_the_kernel() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    let (blue, green) = (
        png(dir.path(), "blue.png", [0, 0, 255, 255]),
        png(dir.path(), "green.png", [0, 255, 0, 255]),
    );
    click(&mut worker, &doc, 150, 150);
    run(&mut worker, &doc, "page.break", &[]);
    run(&mut worker, &doc, "image.insert", &[&blue]);
    pause(&mut worker, &doc);
    for id in [
        "image.anchor_page",
        "image.anchor_char",
        "image.wrap_off",
        "image.wrap_on",
        "image.wrap_ideal",
        "image.wrap_left",
        "image.wrap_right",
    ] {
        run(&mut worker, &doc, id, &[]);
    }
    worker.post_key(&doc, 0, 1281).expect("escape");
    click(&mut worker, &doc, 150, 150);
    pause(&mut worker, &doc);
    run(&mut worker, &doc, "image.insert", &[&green]);
    pause(&mut worker, &doc);
    run(&mut worker, &doc, "image.wrap_through", &[]);
    for id in [
        "image.backward",
        "image.forward",
        "image.back",
        "image.front",
    ] {
        run(&mut worker, &doc, id, &[]);
    }
    run(&mut worker, &doc, "image.anchor_as_char", &[]);
    let text = saved(&dir, &mut worker, &doc, "rest.fodt");
    assert!(
        text.contains("fo:break-before=\"page\""),
        "the page break lands"
    );
    assert_eq!(text.matches("<draw:frame").count(), 2, "both images stay");
    assert!(
        text.contains("text:anchor-type=\"as-char\""),
        "as-char lands"
    );
}

#[test]
fn turning_the_page_undoes_in_one_step() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    run(
        &mut worker,
        &doc,
        "page.orientation",
        &["27940", "21590", "true", "0", "4"],
    );
    let text = saved(&dir, &mut worker, &doc, "turned.fodt");
    assert!(text.contains("fo:page-width=\"11in\" fo:page-height=\"8.5in\""));
    assert!(text.contains("style:print-orientation=\"landscape\""));
    run(&mut worker, &doc, "edit.undo", &[]);
    let text = saved(&dir, &mut worker, &doc, "back.fodt");
    assert!(
        text.contains("fo:page-width=\"8.5in\" fo:page-height=\"11in\""),
        "size back"
    );
    assert!(
        text.contains("style:print-orientation=\"portrait\""),
        "flag back"
    );
}

#[test]
fn a_section_keeps_its_own_page_style() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    worker.post_key(&doc, 0, 1029).expect("end");
    run(&mut worker, &doc, "style.new", &["Section 2", "8"]);
    run(&mut worker, &doc, "page.section", &["3", "Section 2"]);
    run(&mut worker, &doc, "page.columns", &["2"]);
    run(&mut worker, &doc, "footer.on", &["true"]);
    run(&mut worker, &doc, "footer.layout", &["2"]);
    let seen = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| {
        state(seen, ".uno:SwPageFooterLayout") == Some("2")
    });
    assert_eq!(state(&seen, ".uno:PageStyleName"), Some("Section 2"));
    let text = saved(&dir, &mut worker, &doc, "section.fodt");
    assert_eq!(
        text.matches("fo:column-count=\"2\"").count(),
        1,
        "one section has columns"
    );
    let section = text
        .split("style:display-name=\"Section 2\"")
        .nth(1)
        .and_then(|rest| rest.split("</style:master-page>").next())
        .expect("the section's master page");
    assert!(
        section.contains("<style:footer-left>"),
        "its footer differs on even pages"
    );
    let standard = text
        .split("<style:master-page style:name=\"Standard\"")
        .nth(1)
        .and_then(|rest| rest.split("<style:master-page").next())
        .expect("the first section's master page");
    assert!(
        !standard.contains("<style:footer"),
        "the first section keeps no footer"
    );
}
