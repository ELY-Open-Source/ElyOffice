//! Visible signatures placed as ink, text or a picture (C11.7).

use std::path::Path;

use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{settle, shown, windowed};
use crate::content::Tool;
use crate::view::PdfView;
use crate::visible::{Stamp, fitted, placed};

fn opened<'a>(cx: &'a mut TestAppContext, dir: &Path) -> (&'a mut VisualTestContext, Entity<PdfView>) {
    let path = super::file(dir, "doc.pdf", &[(612.0, 792.0, &["Hello"])]);
    let (viewer, cx) = windowed(cx, dir, &[path], true);
    let view = shown(cx, &viewer);
    (cx, view)
}

fn place(cx: &mut VisualTestContext, view: &Entity<PdfView>, stamp: Stamp) {
    cx.update(|_, app| view.update(app, |view, cx| {
        view.set_tool(Tool::PlaceSignature, cx);
        view.stamp = Some(stamp);
        view.place_stamp(0, [100.0, 500.0, 100.0, 500.0], true, cx);
    }));
    settle(cx, |app| view.read(app).edited == 1);
    assert_eq!(cx.update(|_, app| view.read(app).tool), Tool::Read, "one place, then reading");
}

#[test]
fn strokes_fit_by_their_longer_side_and_a_click_places_a_set_width() {
    let (strokes, aspect) = fitted(&[vec![(10.0, 10.0), (110.0, 60.0)]]).expect("a stroke");
    assert_eq!((strokes[0][1], aspect), ((1.0, 0.5), 0.5));
    assert!(fitted(&[vec![(1.0, 1.0)]]).is_none(), "a dot is not a signature");
    assert_eq!(placed(0.5, [10.0, 20.0, 10.0, 20.0], true), [10.0, 20.0, 170.0, 100.0]);
    assert_eq!(placed(0.5, [0.0, 0.0, 300.0, 50.0], false), [0.0, 0.0, 100.0, 50.0], "fits the drag");
}

#[test]
fn a_drawn_signature_lands_as_ink() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (cx, view) = opened(&mut cx, dir.path());
    place(cx, &view, Stamp::Ink { strokes: vec![vec![(0.0, 0.0), (1.0, 0.3)]], aspect: 0.3 });
    cx.update(|_, app| view.update(app, |view, cx| view.ask_notes([0], cx)));
    settle(cx, |app| view.read(app).notes.contains_key(&0));
    let kinds: Vec<String> = cx.update(|_, app| view.read(app).notes[&0].annots.iter().map(|a| a.kind.clone()).collect());
    assert_eq!(kinds, vec!["Ink".to_string()]);
}

#[test]
fn a_typed_signature_lands_as_text() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (cx, view) = opened(&mut cx, dir.path());
    place(cx, &view, Stamp::Typed { text: "Ada Lovelace".into(), font: "script".into() });
    cx.update(|_, app| view.update(app, |view, cx| view.page_text(0, cx)));
    settle(cx, |app| view.read(app).text.contains_key(&0));
    let text: String = cx.update(|_, app| view.read(app).text[&0].iter().map(|c| c.c).collect());
    assert!(text.contains("Ada Lovelace"), "{text}");
}

#[test]
fn a_picture_lands_in_its_own_proportions() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (cx, view) = opened(&mut cx, dir.path());
    let picture = dir.path().join("sig.png");
    image::RgbImage::from_pixel(40, 10, image::Rgb([0, 0, 0])).save(&picture).expect("writes");
    place(cx, &view, Stamp::Picture { path: picture, aspect: 0.25 });
    cx.update(|_, app| view.update(app, |view, cx| {
        view.set_tool(Tool::Edit, cx);
        view.ask_layers(&[0], cx);
    }));
    settle(cx, |app| view.read(app).layers.contains_key(&0));
    let drawn = cx.update(|_, app| view.read(app).layers[&0].drawn.clone());
    assert_eq!(drawn.len(), 1);
    let r = drawn[0].rect;
    assert!(((r[2] - r[0]) - 160.0).abs() < 0.5 && ((r[3] - r[1]) - 40.0).abs() < 0.5, "{r:?}");
}

#[test]
fn the_dialog_hands_a_drawing_to_the_place_tool() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = super::file(dir.path(), "doc.pdf", &[(612.0, 792.0, &["Hello"])]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.ask_visible(window, cx)));
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        let dot = vec![gpui::point(gpui::px(5.0), gpui::px(5.0))];
        viewer.making.as_mut().expect("open").strokes = vec![dot];
        viewer.ready_stamp(cx);
    }));
    assert!(cx.update(|_, app| viewer.read(app).making.is_some()), "a dot is not ready");
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        viewer.making.as_mut().expect("open").strokes = vec![vec![gpui::point(gpui::px(0.0), gpui::px(0.0)), gpui::point(gpui::px(80.0), gpui::px(20.0))]];
        viewer.ready_stamp(cx);
    }));
    assert!(cx.update(|_, app| viewer.read(app).making.is_none()));
    let (tool, stamp) = cx.update(|_, app| (view.read(app).tool, view.read(app).stamp.clone()));
    assert_eq!(tool, Tool::PlaceSignature);
    assert!(matches!(stamp, Some(Stamp::Ink { aspect, .. }) if (aspect - 0.25).abs() < 1e-6));
}

#[test]
fn a_typed_signature_the_font_cannot_draw_says_so_in_chinese_and_spanish() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = super::file(dir.path(), "doc.pdf", &[(612.0, 792.0, &["Hello"])]);
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    // The window's toasts live on its root.
    let root = cx.update(|window, app| {
        let root = ely_ui::document_root(ely_platform::AppId::Pdf, viewer.clone().into(), &ely_ui::commands::FILE, Vec::new(), window, app);
        viewer.update(app, |viewer, _| viewer.core.root = root.downgrade());
        root
    });
    for (locale, said) in [("zh-CN", "这种字体画不出“中”"), ("es-ES", "Esta fuente no puede dibujar «中»")] {
        cx.update(|_, app| ely_ui::i18n::install_as(locale, app));
        cx.update(|_, app| view.update(app, |view, cx| {
            view.set_tool(Tool::PlaceSignature, cx);
            view.stamp = Some(Stamp::Typed { text: "中".into(), font: "script".into() });
            view.place_stamp(0, [100.0, 500.0, 100.0, 500.0], true, cx);
        }));
        settle(cx, |app| root.read(app).toasts(app).iter().any(|(text, _)| text.contains(said)));
    }
    assert_eq!(cx.update(|_, app| view.read(app).edited), 0, "nothing written");
}
