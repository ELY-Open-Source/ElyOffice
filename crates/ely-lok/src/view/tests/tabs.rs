//! Tab stops are removed at the revision that showed them.

use super::*;
use crate::client::tests::{HANG, poll_until};
use gpui::TestAppContext;

const TABS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:automatic-styles>
  <style:style style:name="P1" style:family="paragraph"><style:paragraph-properties><style:tab-stops><style:tab-stop style:position="0.5in"/><style:tab-stop style:position="1in"/></style:tab-stops></style:paragraph-properties></style:style>
  <style:style style:name="P2" style:family="paragraph"><style:paragraph-properties><style:tab-stops><style:tab-stop style:position="1.5in"/><style:tab-stop style:position="2in"/></style:tab-stops></style:paragraph-properties></style:style>
 </office:automatic-styles>
 <office:body><office:text><text:p text:style-name="P1">one</text:p><text:p text:style-name="P2">two</text:p></office:text></office:body>
</office:document>"#;

/// The batch once the cursor's paragraph has `stops`.
fn stops_after(view: &LokView, key: i32, stops: [i32; 2]) -> Events {
    let mut worker = view.worker.lock().expect("the worker lock");
    worker.post_key(&view.doc, 0, key).expect("the key posts");
    let seen = poll_until(&mut worker, &view.doc, &Events::default(), HANG, |seen| seen.tabs == stops);
    assert_eq!(seen.tabs, stops, "the paragraph's stops");
    seen
}

#[test]
fn a_late_batch_of_tab_stops_removes_none_in_the_next_paragraph() {
    let Some(install) = crate::client::tests::install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view_of(app, &install, "tabs.fodt", TABS));
    let errors = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    cx.update(|app| {
        let errors = errors.clone();
        app.subscribe(&view, move |_, error: &LokError, _| errors.borrow_mut().push(error.0.clone())).detach();
        view.update(app, |view, cx| {
            let old = stops_after(view, 1027, [1270, 2540]);
            stops_after(view, 1024, [3810, 5080]);
            let now = view.worker.lock().expect("the worker lock").now(&view.doc).expect("the revision");
            // The landing of Down is seen before P1's batch.
            view.seen_revision(now.rev);
            view.apply_batch(old, None, cx);
            let (shown, read) = view.tabs();
            assert_eq!(shown, [1270, 2540], "the view shows P1's stops");
            view.run_at(&read, "paragraph.tab", &["1", "0", "true"]).expect("it sends");
        })
    });
    let deadline = std::time::Instant::now() + HANG;
    while std::time::Instant::now() < deadline && errors.borrow().is_empty() {
        for _ in 0..20 {
            cx.executor().tick();
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(errors.borrow().iter().any(|error| error.contains("stale")), "{:?}", errors.borrow());
    cx.update(|app| {
        view.update(app, |view, _| {
            stops_after(view, 1027, [3810, 5080]);
            stops_after(view, 1025, [1270, 2540]);
        })
    });
}
