//! Editing text and objects from the app (C11.3).

use ely_pdf_engine::sample::{Extras, one_page, pdf_with};
use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext};

use super::input::over;
use super::{frames, settle, shown, windowed};
use crate::content::{Aim, Tool};
use crate::view::PdfView;
use crate::viewer::Viewer;

/// Two lines, a stamp, a picture; the edit tool on.
fn editing<'a>(cx: &'a mut TestAppContext, dir: &std::path::Path) -> (Entity<Viewer>, &'a mut VisualTestContext, Entity<PdfView>) {
    let path = dir.join("doc.pdf");
    let extras = Extras { stamp: Some("STAMP"), picture: true, ..Extras::default() };
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["Hello world", "Second line"])], &[], &extras)).expect("writes");
    let (viewer, cx) = windowed(cx, dir, &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > gpui::px(100.0));
    // At actual points the whole page is in view.
    cx.update(|_, app| view.update(app, |view, cx| {
        view.zoom = crate::layout::Zoom::Scale(1.0);
        view.set_tool(Tool::Edit, cx)
    }));
    settle(cx, |app| view.read(app).layers.contains_key(&0));
    (viewer, cx, view)
}

/// Presses at page point `at`, with no drag.
fn click(cx: &mut VisualTestContext, view: &Entity<PdfView>, at: (f32, f32)) {
    let point = over(cx, view, 0, at);
    cx.simulate_mouse_down(point, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(point, MouseButton::Left, Modifiers::none());
}

/// The page's blocks once the layer is read again.
fn blocks(cx: &mut VisualTestContext, view: &Entity<PdfView>) -> Vec<String> {
    settle(cx, |app| view.read(app).settled() && view.read(app).layers.contains_key(&0));
    cx.update(|_, app| view.read(app).layers[&0].blocks.iter().map(|b| b.text.clone()).collect())
}

/// Puts `text` in the panel's field.
fn type_in(cx: &mut VisualTestContext, viewer: &Entity<Viewer>, text: &str) {
    frames(cx);
    cx.update(|_, app| {
        let field = viewer.read(app).edit_panel.read(app).field.clone().expect("drawn");
        field.update(app, |field, cx| field.set_text(text, cx));
    });
}

#[test]
fn a_block_edits_in_its_own_font_without_a_preview() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = editing(&mut cx, dir.path());
    // Hello world's first line sits 72 points down.
    click(cx, &view, (80.0, 66.0));
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::Text { .. })));
    type_in(cx, &viewer, "Hi world\nSecond line");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_text(false, cx)));
    let now = blocks(cx, &view);
    assert!(now.iter().any(|b| b.starts_with("Hi world")) && !now.iter().any(|b| b.contains("Hello")), "{now:?}");
    assert!(cx.update(|_, app| viewer.read(app).preview.is_none()), "same font, same space: no preview");
}

#[test]
fn another_font_shows_the_change_first_and_a_missing_glyph_says_so() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = editing(&mut cx, dir.path());
    click(cx, &view, (80.0, 66.0));
    type_in(cx, &viewer, "你好");
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        viewer.edit_panel.update(cx, |panel, _| panel.font = "helvetica".into());
        viewer.write_text(false, cx)
    }));
    settle(cx, |app| viewer.read(app).edit_panel.read(app).note.is_some());
    assert!(cx.update(|_, app| view.read(app).edited) == 0, "nothing changed");
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        viewer.edit_panel.update(cx, |panel, _| panel.font = "cjk-sc".into());
        viewer.write_text(false, cx)
    }));
    settle(cx, |app| viewer.read(app).preview.is_some());
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        let preview = viewer.preview.take().expect("shown");
        viewer.commit_text((preview.tab, preview.version, preview.page), preview.writing, cx)
    }));
    assert!(blocks(cx, &view).iter().any(|b| b.contains("你好")));
}

#[test]
fn an_image_drags_and_scales_from_its_corner() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = editing(&mut cx, dir.path());
    let rect = |cx: &mut VisualTestContext| cx.update(|_, app| view.read(app).layers[&0].drawn[1].rect);
    let (from, to) = (over(cx, &view, 0, (122.0, 267.0)), over(cx, &view, 0, (172.0, 287.0)));
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    settle(cx, |app| view.read(app).settled() && view.read(app).layers.contains_key(&0));
    let moved = rect(cx);
    assert!((moved[0] - 122.0).abs() < 0.5 && (moved[1] - 262.0).abs() < 0.5, "{moved:?}");
    // Edits drop the aim; aim, then pull the corner.
    click(cx, &view, (150.0, 280.0));
    let (corner, out) = (over(cx, &view, 0, (222.0, 312.0)), over(cx, &view, 0, (322.0, 362.0)));
    cx.simulate_mouse_down(corner, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(out, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(out, MouseButton::Left, Modifiers::none());
    settle(cx, |app| view.read(app).settled() && view.read(app).layers.contains_key(&0));
    let scaled = rect(cx);
    assert!((scaled[2] - 322.0).abs() < 0.5 && (scaled[3] - 362.0).abs() < 0.5, "{scaled:?}");
}

#[test]
fn new_text_lands_where_pressed_and_objects_delete() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = editing(&mut cx, dir.path());
    cx.update(|_, app| view.update(app, |view, cx| view.set_tool(Tool::AddText, cx)));
    click(cx, &view, (100.0, 150.0));
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::New { page: 0, .. })));
    type_in(cx, &viewer, "Added");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_text(false, cx)));
    settle(cx, |app| view.read(app).edited == 1);
    cx.update(|_, app| view.update(app, |view, cx| view.set_tool(Tool::Edit, cx)));
    assert!(blocks(cx, &view).iter().any(|b| b == "Added"));
    click(cx, &view, (310.0, 370.0));
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::Object { index: 0, .. })), "the stamp");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.delete_object(cx)));
    assert!(!blocks(cx, &view).iter().any(|b| b == "STAMP"));
}

#[test]
fn a_text_block_drags_to_a_new_place_in_one_step() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = editing(&mut cx, dir.path());
    let hello = |cx: &mut VisualTestContext| {
        cx.update(|_, app| view.read(app).layers[&0].blocks.iter().find(|b| b.text.starts_with("Hello")).map(|b| b.rect))
    };
    let was = hello(cx).expect("hello");
    let (from, to) = (over(cx, &view, 0, (80.0, 66.0)), over(cx, &view, 0, (120.0, 166.0)));
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    blocks(cx, &view);
    let now = hello(cx).expect("hello, moved");
    assert!((now[0] - was[0] - 40.0).abs() < 0.5 && (now[1] - was[1] - 100.0).abs() < 0.5, "{now:?} from {was:?}");
    assert_eq!(cx.update(|_, app| view.read(app).edited), 1, "one edit");
    // A press with no drag only aims.
    click(cx, &view, (120.0, 166.0));
    assert_eq!(cx.update(|_, app| view.read(app).edited), 1);
}

#[test]
fn delete_removes_the_aimed_object_and_nothing_else() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = editing(&mut cx, dir.path());
    cx.update(|window, app| {
        let focus = view.read(app).focus.clone();
        window.focus(&focus, app)
    });
    click(cx, &view, (80.0, 66.0));
    cx.simulate_keystrokes("delete");
    assert_eq!(cx.update(|_, app| view.read(app).edited), 0, "text is not an object");
    click(cx, &view, (310.0, 370.0));
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::Object { index: 0, .. })), "the stamp");
    cx.simulate_keystrokes("delete");
    assert!(!blocks(cx, &view).iter().any(|b| b == "STAMP"), "the stamp is gone");
    assert!(blocks(cx, &view).iter().any(|b| b.starts_with("Hello")), "the text stays");
}

#[test]
fn escape_leaves_the_edit_tool() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = editing(&mut cx, dir.path());
    cx.update(|window, app| {
        let focus = view.read(app).focus.clone();
        window.focus(&focus, app)
    });
    cx.simulate_keystrokes("escape");
    assert_eq!(cx.update(|_, app| view.read(app).tool), Tool::Read);
}

#[test]
fn a_replaced_image_stays_with_its_own_tab() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, a) = editing(&mut cx, dir.path());
    click(cx, &a, (120.0, 267.0));
    let other = super::file(dir.path(), "b.pdf", &[(612.0, 792.0, &["b"])]);
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.replace_image(cx)));
    cx.update(|_, app| viewer.update(app, |viewer, cx| ely_office::Opening::open_file(viewer, other.clone(), cx)));
    settle(cx, |app| viewer.read(app).core.tabs.len() == 2);
    let b = shown(cx, &viewer);
    assert_ne!(a, b, "B is in front when the picker answers");
    let square = dir.path().join("square.pnm");
    std::fs::write(&square, b"P6\n2 2\n255\n\xff\xff\xff\x00\x00\x00\x00\x00\x00\xff\xff\xff").expect("writes");
    cx.cx.simulate_path_prompt_response(move |_| Some(vec![square]));
    settle(cx, |app| a.read(app).edited == 1);
    assert_eq!(cx.update(|_, app| b.read(app).edited), 0);
}

#[test]
fn a_layer_read_from_before_an_edit_is_read_again() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = editing(&mut cx, dir.path());
    // The read starts before the edit lands, returns after.
    cx.update(|_, app| view.update(app, |view, cx| {
        view.edit("pdf.doing.turn", |worker, doc| worker.rotate_pages(doc, &[0], 90), cx);
        view.layers.clear();
        view.ask_layers(&[0], cx);
        assert!(view.reading_layer);
    }));
    settle(cx, |app| view.read(app).settled() && view.read(app).layers.contains_key(&0) && !view.read(app).reading_layer);
}

#[test]
fn escape_from_the_text_field_leaves_the_tool() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = editing(&mut cx, dir.path());
    click(cx, &view, (80.0, 66.0));
    frames(cx);
    cx.update(|window, app| {
        let field = viewer.read(app).edit_panel.read(app).field.clone().expect("drawn");
        let focus = gpui::Focusable::focus_handle(field.read(app), app);
        window.focus(&focus, app)
    });
    cx.simulate_keystrokes("escape");
    assert_eq!(cx.update(|_, app| view.read(app).tool), Tool::Read);
}

#[test]
fn a_preview_goes_once_anything_else_lands() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    // Twin pages: a stale aim would hit the wrong one.
    let same: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["Hello world", "Second line"]), (612.0, 792.0, &["Hello world", "Second line"])];
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[super::file(dir.path(), "twins.pdf", same)], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > gpui::px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| {
        view.zoom = crate::layout::Zoom::Scale(1.0);
        view.set_tool(Tool::Edit, cx)
    }));
    settle(cx, |app| view.read(app).layers.contains_key(&0));
    click(cx, &view, (80.0, 66.0));
    type_in(cx, &viewer, "Hi world\nSecond line");
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        viewer.edit_panel.update(cx, |panel, _| panel.font = "times".into());
        viewer.write_text(false, cx)
    }));
    settle(cx, |app| viewer.read(app).preview.is_some());
    let stale = cx.update(|_, app| {
        let preview = viewer.read(app).preview.as_ref().expect("shown");
        ((preview.tab, preview.version, preview.page), preview.writing.clone())
    });
    cx.update(|_, app| view.update(app, |view, cx| view.edit("pdf.doing.delete_pages", |worker, doc| worker.delete_pages(doc, &[0]), cx)));
    settle(cx, |app| view.read(app).edited == 1 && viewer.read(app).preview.is_none());
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.commit_text(stale.0, stale.1, cx)));
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).edited), 1, "the old preview writes nothing");
}

#[test]
fn nothing_aimed_before_a_queued_edit_is_done() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = editing(&mut cx, dir.path());
    let version = cx.update(|_, app| view.read(app).version);
    // An edit is queued; old numbers may not hold.
    cx.update(|_, app| view.update(app, |view, _| view.editing = true));
    let ran = std::rc::Rc::new(std::cell::Cell::new(false));
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        let ran = ran.clone();
        viewer.on_same(&view, version, cx, move |_, _| ran.set(true))
    }));
    assert!(!ran.get());
}

#[test]
fn a_second_delete_before_the_first_lands_does_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = editing(&mut cx, dir.path());
    click(cx, &view, (310.0, 370.0));
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        viewer.delete_object(cx);
        viewer.delete_object(cx);
    }));
    settle(cx, |app| view.read(app).settled() && view.read(app).layers.contains_key(&0));
    assert_eq!(cx.update(|_, app| view.read(app).layers[&0].drawn.len()), 1, "the picture stays");
}

#[test]
fn a_drag_while_an_edit_is_queued_does_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = editing(&mut cx, dir.path());
    let (from, to) = (over(cx, &view, 0, (122.0, 267.0)), over(cx, &view, 0, (172.0, 287.0)));
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.update(|_, app| view.update(app, |view, _| view.editing = true));
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    assert!(cx.update(|_, app| view.read(app).queue.is_empty()), "nothing queued with old numbers");
}

#[test]
fn the_panel_waits_for_the_drag_so_the_page_stays_put() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = editing(&mut cx, dir.path());
    let wide = cx.update(|_, app| view.read(app).room.size.width);
    let (from, to) = (over(cx, &view, 0, (122.0, 267.0)), over(cx, &view, 0, (140.0, 267.0)));
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).room.size.width), wide, "no panel mid-drag");
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    settle(cx, |app| view.read(app).settled());
    frames(cx);
    cx.update(|_, app| view.update(app, |view, cx| view.press(from, cx)));
    cx.update(|_, app| view.update(app, |view, cx| view.release(cx)));
    frames(cx);
    assert!(cx.update(|_, app| view.read(app).room.size.width) < wide, "the panel opens after");
}

#[test]
fn an_open_panel_stays_open_through_a_second_drag() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = editing(&mut cx, dir.path());
    let wide = cx.update(|_, app| view.read(app).room.size.width);
    click(cx, &view, (122.0, 267.0));
    // The panel slides open; wait until the view stops narrowing.
    let mut narrow = wide;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while std::time::Instant::now() < deadline {
        cx.executor().advance_clock(std::time::Duration::from_millis(50));
        frames(cx);
        let now = cx.update(|_, app| view.read(app).room.size.width);
        if now < wide && now == narrow {
            break;
        }
        narrow = now;
    }
    assert!(narrow < wide, "the panel opened");
    // Pull the picture's corner 50 points right.
    let (from, to) = (over(cx, &view, 0, (172.0, 292.0)), over(cx, &view, 0, (222.0, 292.0)));
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    cx.executor().advance_clock(std::time::Duration::from_millis(500));
    frames(cx);
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.executor().advance_clock(std::time::Duration::from_millis(500));
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).room.size.width), narrow, "the panel stays mid-drag");
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    settle(cx, |app| view.read(app).settled() && view.read(app).layers.contains_key(&0));
    let rect = cx.update(|_, app| view.read(app).layers[&0].drawn[1].rect);
    assert!((rect[2] - 222.0).abs() < 1.0 && (rect[3] - 317.0).abs() < 1.0, "{rect:?}");
}

#[test]
fn a_drag_follows_the_hand_when_the_page_shifts_under_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = editing(&mut cx, dir.path());
    let from = over(cx, &view, 0, (122.0, 267.0));
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    // The view shifts right, as a panel pushes it.
    cx.update(|_, app| view.update(app, |view, _| view.room.origin.x += gpui::px(100.0)));
    let to = gpui::point(from.x + gpui::px(50.0), from.y);
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    settle(cx, |app| view.read(app).settled() && view.read(app).layers.contains_key(&0));
    let rect = cx.update(|_, app| view.read(app).layers[&0].drawn[1].rect);
    assert!((rect[0] - 122.0).abs() < 0.5, "moved as far as the hand: {rect:?}");
}

#[test]
fn text_on_a_full_page_picture_aims_at_the_text_and_a_shake_stays_a_click() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = dir.path().join("scan.pdf");
    let extras = Extras { picture: true, backdrop: true, ..Extras::default() };
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["Hello world"])], &[], &extras)).expect("writes");
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > gpui::px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| {
        view.zoom = crate::layout::Zoom::Scale(1.0);
        view.set_tool(Tool::Edit, cx)
    }));
    settle(cx, |app| view.read(app).layers.contains_key(&0));
    click(cx, &view, (80.0, 66.0));
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::Text { .. })), "the text, not the picture under it");
    // A two-pixel wobble on the picture moves nothing.
    let from = over(cx, &view, 0, (300.0, 400.0));
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    let to = from + gpui::point(gpui::px(2.0), gpui::px(0.0));
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    frames(cx);
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::Object { .. })), "the picture is aimed");
    assert_eq!(cx.update(|_, app| view.read(app).edited), 0, "and not moved");
}

#[test]
fn a_small_picture_on_text_under_a_page_form_still_aims() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = dir.path().join("layered.pdf");
    // The page-size form comes last, on top of both.
    let content = "q 10 0 0 10 80 722 cm /Im1 Do Q BT /F1 18 Tf 72 720 Td (Hello world) Tj ET q /Fm1 Do Q";
    let form = "0 0 1 rg 500 50 20 20 re f";
    let objects = [
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        format!("<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] /Length {} >>\nstream\n{form}\nendstream", form.len()),
        "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8 /Length 1 >>\nstream\nA\nendstream".to_string(),
    ];
    let page = " /Resources << /Font << /F1 5 0 R >> /XObject << /Fm1 6 0 R /Im1 7 0 R >> >>";
    std::fs::write(&path, one_page("", page, content, &objects)).expect("writes");
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| view.read(app).room.size.width > gpui::px(100.0));
    cx.update(|_, app| view.update(app, |view, cx| {
        view.zoom = crate::layout::Zoom::Scale(1.0);
        view.set_tool(Tool::Edit, cx)
    }));
    settle(cx, |app| view.read(app).layers.contains_key(&0));
    // The picture sits at 80..90 across, 60..70 down.
    click(cx, &view, (85.0, 65.0));
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::Object { index: 0, .. })), "the small picture");
    click(cx, &view, (140.0, 66.0));
    assert!(matches!(cx.update(|_, app| view.read(app).aim), Some(Aim::Text { .. })), "the text, not the form around it");
}

#[test]
fn a_preview_overtaken_by_an_edit_says_so() {
    use ely_gpui_component::primitives::Severity;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = editing(&mut cx, dir.path());
    // The window's toasts live on its root.
    let root = cx.update(|window, app| {
        let root = ely_ui::document_root(ely_platform::AppId::Pdf, viewer.clone().into(), &ely_ui::commands::FILE, Vec::new(), window, app);
        viewer.update(app, |viewer, _| viewer.core.root = root.downgrade());
        root
    });
    click(cx, &view, (80.0, 66.0));
    type_in(cx, &viewer, "Hi world\nSecond line");
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        // A new font asks for a preview first.
        viewer.edit_panel.update(cx, |panel, _| panel.font = "times".into());
        viewer.write_text(false, cx);
        // An edit lands before the preview answers.
        if let Some(view) = viewer.active_view() {
            view.update(cx, |view, _| view.version += 1);
        }
    }));
    let changed = cx.update(|_, app| Viewer::text(app, "pdf.object.changed", &[]));
    settle(cx, |app| root.read(app).toasts(app).iter().any(|(text, severity)| *severity == Some(Severity::Warning) && *text == changed));
    frames(cx);
    assert!(cx.update(|_, app| viewer.read(app).preview.is_none()), "no stale preview");
    assert_eq!(cx.update(|_, app| view.read(app).edited), 0, "nothing written");
}
