use gpui::{Entity, TestAppContext};

use super::Props;
use super::tests::panel;
use crate::docs::Docs;
use crate::insert::InsertPageNumber;
use crate::tests::{blank, docs, settle, settled, text_of, type_into, with_window, written};
use ely_office::panel::Panel as _;
use ely_office::prelude::*;

fn state(cx: &TestAppContext, props: &Entity<Props>, uno: &str) -> Option<String> {
    cx.update(|app| {
        props
            .read(app)
            .state(uno, app)
            .map(|state| state.to_string())
    })
}

#[test]
fn the_page_turns_resizes_and_keeps_quick_margins() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Page");
    let props = panel(&mut cx, &docs);
    settle(&cx, |app| props.read(app).sheet(app).is_some());
    cx.update(|app| props.update(app, |props, cx| props.set_orientation(true, cx)));
    settle(&cx, |app| {
        let props = props.read(app);
        let size = props.state(".uno:AttributePageSize", app);
        let size = size.as_deref().and_then(super::sheet::size_of);
        props
            .state(".uno:AttributePage", app)
            .is_some_and(|page| page.starts_with("true,"))
            && size.is_some_and(|(w, h)| w > h)
    });
    cx.update(|app| props.update(app, |props, cx| props.set_paper("A5", cx)));
    settle(&cx, |app| {
        let size = props.read(app).state(".uno:AttributePageSize", app);
        size.as_deref()
            .and_then(super::sheet::size_of)
            .is_some_and(|(w, h)| (w - 21000).abs() <= 2 && (h - 14800).abs() <= 2)
    });
    let margins = (".uno:PageLRMargin", "page.margins_lr");
    let twips = |cm: f64| (cm * 1440.0 / 2.54).round() as i32;
    cx.update(|app| {
        props.update(app, |props, cx| {
            props.set_pair(margins, false, twips(2.0), cx);
            props.set_pair(margins, true, twips(3.0), cx);
        })
    });
    settle(&cx, |app| {
        props.read(app).state(".uno:PageLRMargin", app).as_deref() == Some("2000,3000")
    });
    settled(&cx, &docs);
    let path = dir.path().join("page.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, path.clone(), None, window, cx)
    });
    written(&cx, &path, None);
    let text = text_of(&mut cx, &docs, &path);
    assert!(
        text.contains("style:print-orientation=\"landscape\""),
        "landscape lands"
    );
    assert!(
        text.contains("fo:page-width=\"8.2681in\""),
        "A5 across lands"
    );
    assert!(
        text.contains("fo:margin-left=\"0.7874in\" fo:margin-right=\"1.1811in\""),
        "margins land"
    );
}

#[test]
fn insert_actions_and_object_size_land() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Insert");
    let props = panel(&mut cx, &docs);
    settle(&cx, |app| {
        props.read(app).state(".uno:PageStyleName", app).is_some()
    });
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.on_page_number(&InsertPageNumber, window, cx);
        docs.insert_section(cx);
        docs.insert_table(3.0, 2.0, cx);
    });
    settle(&cx, |app| {
        props.read(app).state(".uno:DeleteTable", app).as_deref() == Some("enabled")
    });
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(300, 200, image::Rgba([0, 0, 255, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("the png encodes");
    let image = dir.path().join("blue.png");
    std::fs::write(&image, png).expect("the png writes");
    let url = ely_lok::file_url(&image);
    cx.update(|app| props.update(app, |props, cx| props.run("image.insert", &[&url], cx)));
    settle(&cx, |app| {
        props
            .read(app)
            .state(".uno:Size", app)
            .is_some_and(|s| s.contains(" x "))
    });
    settle(&cx, |app| {
        docs.read(app)
            .active_tab()
            .expect("a tab")
            .view
            .read(app)
            .graphic()
            .is_some()
    });
    let before = state(&cx, &props, ".uno:Size").expect("a size");
    let (w0, h0) = super::sheet::size_of(&before).expect("parsed");
    cx.update(|app| props.update(app, |props, cx| props.set_extent(false, 2.0, cx)));
    settle(&cx, |app| {
        let size = props.read(app).state(".uno:Size", app);
        size.as_deref()
            .and_then(super::sheet::size_of)
            .is_some_and(|(w, _)| (w - 2000).abs() <= 3)
    });
    let (w, h) =
        super::sheet::size_of(&state(&cx, &props, ".uno:Size").expect("a size")).expect("parsed");
    let ratio = (f64::from(w) / f64::from(h)) / (f64::from(w0) / f64::from(h0));
    assert!(
        (ratio - 1.0).abs() < 0.01,
        "the ratio holds: {w}x{h} from {w0}x{h0}"
    );
    settled(&cx, &docs);
    let path = dir.path().join("insert.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, path.clone(), None, window, cx)
    });
    written(&cx, &path, None);
    let text = text_of(&mut cx, &docs, &path);
    for mark in [
        "<text:page-number",
        "<table:table ",
        "<draw:frame",
        "style:master-page-name=",
    ] {
        assert!(text.contains(mark), "{mark} lands");
    }
}

#[test]
fn sections_footers_and_tall_images_hold_up() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "First");
    let props = panel(&mut cx, &docs);
    settle(&cx, |app| {
        props.read(app).state(".uno:PageStyleName", app).is_some()
    });
    cx.update(|app| docs.update(app, |docs, cx| docs.insert_section(cx)));
    let name = cx.update(|app| Docs::text(app, "doc.section.name", &[("n", "2")]));
    settle(&cx, |app| {
        props.read(app).state(".uno:PageStyleName", app).as_deref() == Some(&*name)
    });
    type_into(&mut cx, &docs, "Second");
    for (command, values) in [
        ("page.columns", vec!["2"]),
        ("footer.on", vec!["true"]),
        ("footer.layout", vec!["2"]),
    ] {
        cx.update(|app| props.update(app, |props, cx| props.run(command, &values, cx)));
    }
    settle(&cx, |app| {
        props
            .read(app)
            .state(".uno:SwPageFooterLayout", app)
            .as_deref()
            == Some("2")
    });
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(20, 400, image::Rgba([0, 0, 255, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("the png encodes");
    let image = dir.path().join("tall.png");
    std::fs::write(&image, png).expect("the png writes");
    let url = ely_lok::file_url(&image);
    cx.update(|app| props.update(app, |props, cx| props.run("image.insert", &[&url], cx)));
    settle(&cx, |app| {
        docs.read(app)
            .active_tab()
            .expect("a tab")
            .view
            .read(app)
            .graphic()
            .is_some()
    });
    // A person takes this long to reach the panel.
    std::thread::sleep(std::time::Duration::from_millis(500));
    cx.update(|app| {
        props.update(app, |props, cx| {
            props.run("image.size", &["58", "5670"], cx)
        })
    });
    settle(&cx, |app| {
        let size = props.read(app).state(".uno:Size", app);
        size.as_deref()
            .and_then(super::sheet::size_of)
            .is_some_and(|(_, h)| (h - 10001).abs() <= 3)
    });
    cx.update(|app| props.update(app, |props, cx| props.set_extent(true, 10.1, cx)));
    settle(&cx, |app| {
        let size = props.read(app).state(".uno:Size", app);
        size.as_deref()
            .and_then(super::sheet::size_of)
            .is_some_and(|(_, h)| (h - 10100).abs() <= 3)
    });
    settled(&cx, &docs);
    let path = dir.path().join("sections.docx");
    with_window(&mut cx, &docs, |docs, window, cx| {
        docs.write_as(id, path.clone(), None, window, cx)
    });
    written(&cx, &path, None);
    let text = text_of(&mut cx, &docs, &path);
    assert_eq!(
        text.matches("fo:column-count=\"2\"").count(),
        1,
        "only the second section has columns"
    );
    assert!(
        text.matches("style:master-page-name=").count() >= 1,
        "a section starts"
    );
    assert!(
        text.contains("<style:footer-left>"),
        "the footer differs on even pages"
    );
}

#[test]
fn queued_page_edits_footers_and_new_sections_show_up() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "Page");
    let props = panel(&mut cx, &docs);
    settle(&cx, |app| props.read(app).sheet(app).is_some());
    cx.update(|app| {
        props.update(app, |props, cx| {
            props.set_paper("A5", cx);
            props.set_orientation(true, cx);
        })
    });
    settle(&cx, |app| {
        let props = props.read(app);
        let size = props.state(".uno:AttributePageSize", app);
        let size = size.as_deref().and_then(super::sheet::size_of);
        props
            .state(".uno:AttributePage", app)
            .is_some_and(|page| page.starts_with("true,"))
            && size.is_some_and(|(w, h)| (w - 21000).abs() <= 2 && (h - 14800).abs() <= 2)
    });
    cx.update(|app| props.update(app, |props, cx| props.run("footer.on", &["true"], cx)));
    settle(&cx, |app| {
        props
            .read(app)
            .state(".uno:SwPageFooterLayout", app)
            .as_deref()
            == Some("0")
    });
    cx.update(|app| docs.update(app, |docs, cx| docs.insert_section(cx)));
    let name = cx.update(|app| Docs::text(app, "doc.section.name", &[("n", "2")]));
    settle(&cx, |app| {
        let props = props.read(app);
        props
            .shown
            .as_ref()
            .is_some_and(|shown| shown.pages.contains(&name))
    });
}

#[test]
fn a_new_document_starts_on_the_chosen_paper() {
    use ely_platform::settings::Paper;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    for (paper, width, height) in [(Paper::A4, 21000, 29700), (Paper::Letter, 21590, 27940)] {
        cx.update(|app| app.global_mut::<ely_ui::settings::Current>().shared.paper = paper);
        blank(&mut cx, &docs);
        let props = panel(&mut cx, &docs);
        settle(&cx, |app| {
            let props = props.read(app);
            let size = props.state(".uno:AttributePageSize", app);
            size.as_deref()
                .and_then(super::sheet::size_of)
                .is_some_and(|(w, h)| (w - width).abs() <= 2 && (h - height).abs() <= 2)
                && props.state(".uno:PageLRMargin", app).as_deref() == Some("2000,2000")
                && props.state(".uno:PageULMargin", app).as_deref() == Some("2000,2000")
        });
    }
    settled(&cx, &docs);
}
