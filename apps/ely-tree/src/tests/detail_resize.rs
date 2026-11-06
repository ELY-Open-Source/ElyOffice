use gpui::{Modifiers, TestAppContext, VisualTestContext, px};

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

#[test]
fn commit_detail_dividers_resize_both_axes() {
    use super::{committed, window};
    use crate::tree::Side;
    use gpui::{MouseButton, Pixels, Point, point};
    let dir = tempfile::tempdir().expect("test folder");
    let root = committed(dir.path());
    let mut app = TestAppContext::single();
    let (tree, cx) = window::window(&mut app, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(root, cx)));
    window::settle(cx, |app| {
        tree.read(app).active().is_some_and(|r| r.status.is_some())
    });
    cx.update(|_, app| tree.update(app, |tree, cx| tree.set_side(Side::History, cx)));
    window::settle(cx, |app| {
        tree.read(app)
            .active()
            .is_some_and(|r| r.hist.read && !r.hist.loading)
    });
    let id = cx.update(|_, app| tree.read(app).active().unwrap().hist.rows[0].id.clone());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.pick_commit(&id, cx)));
    window::settle(cx, |app| {
        tree.read(app).active().unwrap().hist.pick.detail.is_some()
    });
    let drag = |cx: &mut VisualTestContext, from: Point<Pixels>, to: Point<Pixels>| {
        cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
        settle(cx);
        for step in [0.1, 0.2, 1.0] {
            cx.simulate_mouse_move(
                from + (to - from) * step,
                MouseButton::Left,
                Modifiers::default(),
            );
            settle(cx);
        }
        cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
        settle(cx);
    };
    let head = cx.debug_bounds("tree-commit-head").expect("header");
    let files = cx.debug_bounds("tree-touched").expect("files");
    let horizontal = point(head.center().x, (head.bottom() + files.top()) / 2.0);
    drag(cx, horizontal, horizontal + point(px(0.), px(80.)));
    let taller = cx.debug_bounds("tree-commit-head").unwrap();
    assert!(taller.size.height > head.size.height + px(30.));
    let files = cx.debug_bounds("tree-touched").unwrap();
    let shown = cx.debug_bounds("tree-touched-shown").unwrap();
    let vertical = point((files.right() + shown.left()) / 2.0, files.center().y);
    drag(cx, vertical, vertical + point(px(80.), px(0.)));
    let wider = cx.debug_bounds("tree-touched").unwrap();
    assert!(wider.size.width > files.size.width + px(30.));
    assert_eq!(
        cx.debug_bounds("tree-commit-head").unwrap().size.height,
        taller.size.height
    );
    assert_eq!(
        cx.update(|_, app| tree.read(app).active().unwrap().hist.pick.commit.clone()),
        Some(id.to_string())
    );
}
