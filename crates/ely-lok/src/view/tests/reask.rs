//! A dropped shown tile returns with the next frame.

use super::reveal::{counted, landed, long_view};
use super::*;

/// No batch re-asks it: the frame's want() does.
#[test]
fn a_dropped_shown_tile_is_asked_again_by_the_next_frame() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let cx = gpui::TestAppContext::single();
    let (view, bounds) = long_view(&cx, &install);
    // A real change: the kernel's count passes zero.
    counted(&cx, &view, "a dropped tile");
    let (worker, doc) = cx.update(|app| (Arc::clone(&view.read(app).worker), view.read(app).doc.clone()));
    let kernel = || worker.lock().expect("the worker lock").state(&doc).expect("the state reads").1;
    let deadline = std::time::Instant::now() + crate::client::tests::HANG;
    while cx.update(|app| view.read(app).generation) != kernel() {
        assert!(std::time::Instant::now() < deadline, "the view never took the kernel's batch");
        cx.update(|app| view.update(app, |view, _| _ = view.prepare(bounds)));
        cx.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    landed(&cx, &view);
    let notified = std::rc::Rc::new(std::cell::Cell::new(0));
    let frames = std::rc::Rc::clone(&notified);
    cx.update(|app| app.observe(&view, move |_, _| frames.set(frames.get() + 1)).detach());
    let (key, generation) = cx.update(|app| view.update(app, |view, cx| {
        let key = view.on_screen[0];
        // In flight when the change came: it paints too old.
        view.tiles.get_mut(&key).expect("the shown tile").stale = true;
        view.fetching.insert(key);
        let side = view.geom.tile_pixels() as usize;
        let old = crate::Tile { generation: view.generation - 1, pixels: vec![0; side * side * 4] };
        view.finish_tile(key, view.geom, Ok(old), cx);
        (key, view.generation)
    }));
    let stranded = |app: &gpui::App| {
        let view = view.read(app);
        view.tiles.get(&key).is_some_and(|tile| tile.stale) && !view.fetching.contains(&key)
    };
    assert!(cx.update(|app| stranded(app)), "dropped, and nothing asks for it yet");
    assert!(notified.get() > 0, "the drop asks for a frame");
    cx.run_until_parked();
    assert!(cx.update(|app| stranded(app)), "without a frame it stays dropped");
    // The frames gpui draws: prepare, then the lane.
    let deadline = std::time::Instant::now() + crate::client::tests::HANG;
    while cx.update(|app| view.read(app).tiles.get(&key).is_some_and(|tile| tile.stale)) {
        assert!(std::time::Instant::now() < deadline, "the dropped tile never came back");
        cx.update(|app| view.update(app, |view, _| _ = view.prepare(bounds)));
        cx.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(cx.update(|app| view.read(app).generation), generation, "no batch came between");
    assert_eq!(kernel(), generation, "nor did the kernel change");
}
