//! A picture on a slide by index (patch 0159).

use super::analysis::steps;
use super::layout::png;
use super::show::open_deck;
use super::*;
use crate::show;

/// One attribute of an XML tag, in cm.
fn cm(tag: &str, name: &str) -> f64 {
    let start = tag.find(&format!(" {name}=\"")).unwrap_or_else(|| panic!("{name} in {tag}")) + name.len() + 3;
    let value = &tag[start..start + tag[start..].find('"').expect("closed")];
    value.strip_suffix("cm").unwrap_or_else(|| panic!("{value} in cm")).parse().expect("a number")
}

/// Left, top, width, height of each picture on page `name`.
fn pictures(xml: &str, name: &str) -> Vec<(f64, f64, f64, f64)> {
    let page = &xml[xml.find(&format!("<draw:page draw:name=\"{name}\"")).expect("the page")..];
    let page = &page[..page.find("</draw:page>").expect("its end")];
    page.split("<draw:frame")
        .skip(1)
        .filter(|frame| frame.contains("<draw:image"))
        .map(|frame| {
            let tag = &frame[..frame.find('>').expect("a tag")];
            (cm(tag, "svg:x"), cm(tag, "svg:y"), cm(tag, "svg:width"), cm(tag, "svg:height"))
        })
        .collect()
}

#[test]
fn a_picture_lands_centered_fitted_and_the_view_stays() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    // 300 by 200 pixels: 3 to 2.
    let red = png(dir.path(), "red.png", [255, 0, 0, 255]);
    let before = steps(&mut worker, &doc);
    worker.run(&doc, "slide.image", &["1", &red, ""]).expect("its own size");
    worker.run(&doc, "slide.image", &["1", &red, "4535"]).expect("8 cm wide");
    worker.run(&doc, "slide.image", &["1", &red, "56693"]).expect("100 cm, fitted");
    assert_eq!(steps(&mut worker, &doc), before + 3, "one step each");
    assert_eq!(doc.sheets(&mut worker).expect("sheets").current, 0, "the view stays");
    let shown = show::objects(&worker.values(&doc, show::OBJECTS).expect("reads")).expect("parses");
    assert!(shown.selection.is_none() && shown.objects.len() == 1, "{shown:?}");
    let out = dir.path().join("pictures.fodp");
    doc.save_as(&mut worker, &crate::file_url(&out), "fodp").expect("saves");
    let xml = std::fs::read_to_string(&out).expect("reads");
    assert!(pictures(&xml, "One").is_empty(), "the shown slide has none");
    let found = pictures(&xml, "Two");
    let (width, height) = doc.size(&mut worker).expect("the slide");
    let (slide_w, slide_h) = (width as f64 * 2.54 / 1440.0, height as f64 * 2.54 / 1440.0);
    assert_eq!(found.len(), 3, "{found:?}");
    for (x, y, w, h) in &found {
        assert!(((x + w / 2.0) - slide_w / 2.0).abs() < 0.01 && ((y + h / 2.0) - slide_h / 2.0).abs() < 0.01, "centered: {found:?}");
        assert!((w / h - 1.5).abs() < 0.01, "ratio kept: {found:?}");
    }
    assert!((found[0].2 - 7.937).abs() < 0.01, "300 px at 96 dpi: {found:?}");
    assert!((found[1].2 - 8.0).abs() < 0.01, "8 cm: {found:?}");
    let (_, _, w, h) = found[2];
    assert!(w <= slide_w + 0.01 && h <= slide_h + 0.01, "fitted: {found:?}");
    assert!((w - slide_w).abs() < 0.01 || (h - slide_h).abs() < 0.01, "as large as fits: {found:?}");
    worker.run(&doc, "edit.undo", &[]).expect("undoes");
    let undone = dir.path().join("undone.fodp");
    doc.save_as(&mut worker, &crate::file_url(&undone), "fodp").expect("saves");
    assert_eq!(pictures(&std::fs::read_to_string(&undone).expect("reads"), "Two").len(), 2, "undo takes the last");
}

#[test]
fn a_non_picture_or_missing_slide_changes_nothing() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    let broken = dir.path().join("broken.png");
    std::fs::write(&broken, "not a picture").expect("writes");
    let red = png(dir.path(), "red.png", [255, 0, 0, 255]);
    let before = steps(&mut worker, &doc);
    worker.run(&doc, "slide.image", &["1", &crate::file_url(&broken), ""]).expect_err("no picture");
    worker.run(&doc, "slide.image", &["2", &red, ""]).expect_err("no third slide");
    worker.run(&doc, "slide.image", &["1", &red, "0"]).expect_err("no width");
    assert_eq!(steps(&mut worker, &doc), before, "nothing changed");
}
