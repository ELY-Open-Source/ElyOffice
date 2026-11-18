//! SHOW-03: a shape the menu inserts leaves its tool.

use super::*;
use crate::show;

/// One empty rectangle, left of the slide's middle.
const ONE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
 xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:body><office:presentation>
 <draw:page draw:name="One"><draw:rect svg:x="1cm" svg:y="9cm" svg:width="4cm" svg:height="4cm"/></draw:page>
 </office:presentation></office:body>
</office:document>"#;

/// 1/100 mm in twips.
fn twips(mm100: i64) -> i32 {
    (mm100 * 72 / 127) as i32
}

fn read(worker: &mut Worker, doc: &Doc) -> show::Objects {
    show::objects(&worker.values(doc, show::OBJECTS).expect("reads")).expect("parses")
}

#[test]
fn a_click_after_an_insert_picks_and_draws_nothing() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    // Insert alone, twice, or then moved.
    for (kind, times, moved) in [("rectangle", 1, false), ("ellipse", 1, false), ("rectangle", 2, false), ("rectangle", 1, true)] {
        let dir = tempfile::tempdir().expect("a temp dir");
        let mut worker = Worker::spawn(&install().expect("LOK"), &dir.path().join("p")).expect("up");
        let path = dir.path().join("one.fodp");
        std::fs::write(&path, ONE).expect("writes");
        let doc = worker.load(&crate::file_url(&path)).expect("loads");
        for _ in 0..times {
            worker.run(&doc, "draw.shape", &[kind, "true"]).expect("inserts");
        }
        if moved {
            worker.run(&doc, "object.move", &[&twips(15000).to_string(), &twips(2000).to_string()]).expect("moves");
        }
        let inserted = read(&mut worker, &doc);
        assert_eq!(inserted.objects.len(), 1 + times, "{kind} x{times}: inserted");
        let (x, y, w, h) = inserted.objects[0].rect;
        let at = (x + w / 2, y + h / 2);
        let clear = |&(l, t, w, h): &(i64, i64, i64, i64)| at.0 < l || at.0 > l + w || at.1 < t || at.1 > t + h;
        assert!(inserted.objects[1..].iter().map(|o| o.rect).all(|r| clear(&r)), "{kind}: the click lands on the old shape only");
        let framed = poll_until(&mut worker, &doc, &Events::default(), HANG, |seen| seen.graphic.is_some());
        for press in [0, 1] {
            let click = Mouse { kind: press, x: twips(at.0), y: twips(at.1), count: 1, buttons: 1, modifiers: 0 };
            worker.post_mouse(&doc, click).expect("the click posts");
        }
        let left = twips(x);
        poll_until(&mut worker, &doc, &framed, HANG, |seen| seen.graphic.is_some_and(|r| (r.0 - left).abs() < 60));
        let after = read(&mut worker, &doc);
        assert_eq!(after.objects.len(), 1 + times, "{kind} x{times}: the click drew no shape");
        let picked: Vec<usize> = (after.objects.iter().enumerate()).filter(|(_, o)| o.selected).map(|(i, _)| i).collect();
        assert_eq!(picked, [0], "{kind} x{times}: the clicked shape is selected");
    }
}
