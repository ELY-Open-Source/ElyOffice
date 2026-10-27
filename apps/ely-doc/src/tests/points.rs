//! An unsaved tab's recovery point (X-07).

use gpui::TestAppContext;

use ely_office::Recovering as _;
use ely_office::prelude::*;

use super::{blank, docs, settle, type_into};

fn points(dir: &std::path::Path) -> usize {
    let Ok(slots) = std::fs::read_dir(dir.join("recovery")) else { return 0 };
    let slots = slots.filter_map(Result::ok).filter(|slot| slot.path().is_dir());
    let names = slots.filter_map(|slot| std::fs::read_dir(slot.path().join("points")).ok()).flatten();
    names.filter_map(Result::ok).filter(|name| name.path().extension().is_some_and(|ext| ext == "bin")).count()
}

#[test]
fn an_unsaved_tab_gets_a_point() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(docs) = docs(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let kept = dir.path().join("kept");
    let keeping = ely_office::Keeping::in_folder(kept.clone()).expect("recovery starts");
    cx.update(|app| docs.update(app, |docs, _| docs.core.keeping = Some(keeping)));
    let id = blank(&mut cx, &docs);
    type_into(&mut cx, &docs, "kept");
    settle(&cx, |app| docs.read(app).tab(id).is_some_and(|tab| tab.dirty(app)));
    cx.update(|app| docs.update(app, |docs, cx| docs.keep_points(cx)));
    settle(&cx, |_| points(&kept) == 1);
}
