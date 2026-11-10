use std::rc::Rc;

use ely_gpui_component::git::{Commit, CommitList, GraphRow};
use gpui::{
    Context, IntoElement, Modifiers, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div, px,
};

struct History {
    width: f32,
    picked: Option<String>,
}

impl Render for History {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let commits = (0..50)
            .map(|n| Commit {
                id: format!("c{n}").into(),
                parents: vec![],
                subject: "历史提交仍然可见".into(),
                author: "Author".into(),
                when: "2026-11-05".into(),
                refs: if n == 0 {
                    vec![
                        "feature/a-very-long-branch".into(),
                        "origin/feature/a-very-long-branch".into(),
                    ]
                } else {
                    vec![]
                },
            })
            .collect();
        let graph = (0..50)
            .map(|n| GraphRow {
                lane: n % 15,
                width: 15,
                strokes: vec![],
            })
            .collect();
        let this = cx.entity();
        div().w(px(self.width)).h(px(400.)).overflow_hidden().child(
            CommitList::new("history", Rc::new(commits))
                .graph(Rc::new(graph))
                .on_pick(move |id, _, cx| this.update(cx, |s, _| s.picked = Some(id.to_string()))),
        )
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

#[test]
fn many_graph_lanes_leave_commit_titles_visible_and_clickable() {
    for width in [220., 280., 800.] {
        let mut cx = TestAppContext::single();
        cx.update(ely_gpui_component::init_for_tests);
        let (view, cx) = cx.add_window_view(move |_, _| History {
            width,
            picked: None,
        });
        cx.update(|window, _| window.activate_window());
        settle(cx);
        for key in ["commit-title-c0", "commit-title-c1"] {
            let bounds = cx.debug_bounds(key).expect("commit title");
            assert!(
                bounds.left() >= px(0.) && bounds.right() <= px(width),
                "{key} outside {width}: {bounds:?}"
            );
            assert!(bounds.size.width >= px(80.), "{key} unreadable: {bounds:?}");
            cx.simulate_click(bounds.center(), Modifiers::default());
            settle(cx);
        }
        assert_eq!(
            view.read_with(cx, |v, _| v.picked.clone()),
            Some("c1".into())
        );
    }
}

#[test]
fn graph_toggle_reclaims_space_without_changing_the_commit() {
    use super::{committed, window};
    use crate::tree::Side;
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
    let title: &'static str = Box::leak(format!("commit-title-{id}").into_boxed_str());
    let shown = cx.debug_bounds(title).expect("commit title");
    cx.simulate_click(shown.center(), Modifiers::default());
    settle(cx);
    for visible in [false, true] {
        let toggle = cx
            .debug_bounds("toggle history-graph")
            .expect("graph toggle");
        cx.simulate_click(toggle.center(), Modifiers::default());
        settle(cx);
        let bounds = cx.debug_bounds(title).expect("title stays visible");
        if visible {
            assert_eq!(bounds.left(), shown.left());
        } else {
            assert!(bounds.left() < shown.left(), "hidden graph frees its width");
        }
        cx.update(|_, app| {
            let tree = tree.read(app);
            assert_eq!(tree.changes.read(app).show_graph, visible);
            assert_eq!(
                tree.active().unwrap().hist.pick.commit.as_deref(),
                Some(id.as_ref())
            );
        });
    }
}
