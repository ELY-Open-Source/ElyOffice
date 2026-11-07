//! SHOW-04: a connector's ends glue to shapes.

use super::*;
use crate::show;

/// Two shapes 10 cm apart on one slide.
const PAIR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
 xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:body><office:presentation>
 <draw:page draw:name="One"><draw:rect svg:x="2cm" svg:y="5cm" svg:width="4cm" svg:height="3cm"/><draw:rect svg:x="16cm" svg:y="5cm" svg:width="4cm" svg:height="3cm"/></draw:page>
 </office:presentation></office:body>
</office:document>"#;

/// Centimetres in twips.
fn cm(value: f64) -> i32 {
    (value * 1440.0 / 2.54).round() as i32
}

/// The saved connector's tag.
fn connector(xml: &str) -> String {
    let at = xml.find("<draw:connector").expect("a connector is saved");
    xml[at..at + xml[at..].find('>').expect("the tag ends")].to_string()
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let start = tag.find(&format!(" {name}=\""))? + name.len() + 3;
    Some(&tag[start..start + tag[start..].find('"')?])
}

/// An end's position in twips.
fn end(tag: &str, which: char) -> (i32, i32) {
    let read = |axis: char| {
        let value = attr(tag, &format!("svg:{axis}{which}")).expect("the end is saved");
        cm(value.strip_suffix("cm").expect("cm").parse().expect("a number"))
    };
    (read('x'), read('y'))
}

#[test]
fn a_connector_s_ends_glue_to_shapes_and_follow_a_move() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install().expect("LOK"), &dir.path().join("p")).expect("up");
    let path = dir.path().join("pair.fodp");
    std::fs::write(&path, PAIR).expect("writes");
    let doc = worker.load(&crate::file_url(&path)).expect("loads");
    let saved = |worker: &mut Worker, name: &str| {
        let out = dir.path().join(name);
        doc.save_as(worker, &crate::file_url(&out), "fodp")
            .expect("saves");
        connector(&std::fs::read_to_string(out).expect("reads"))
    };
    worker.run(&doc, "draw.connector", &[]).expect("the tool");
    let read = show::objects(&worker.values(&doc, show::OBJECTS).expect("reads")).expect("parses");
    let kinds: Vec<&str> = read.objects.iter().map(|object| object.kind.as_str()).collect();
    assert_eq!(kinds, ["shape", "shape", "connector"], "one new connector");
    let drawn = saved(&mut worker, "drawn.fodp");
    assert!(attr(&drawn, "draw:start-shape").is_none(), "it starts loose: {drawn}");
    // Each step waits for the frame.
    let wait = std::time::Duration::from_secs(5);
    let settle = |worker: &mut Worker, done: &dyn Fn(crate::events::Rect) -> bool, what: &str| {
        let seen = poll_until(worker, &doc, &Events::default(), wait, |seen| seen.graphic.is_some_and(done));
        seen.graphic.filter(|&rect| done(rect)).unwrap_or_else(|| panic!("{what}: {:?}", seen.graphic))
    };
    let left = end(&drawn, '1').0.min(end(&drawn, '2').0);
    let first = settle(&mut worker, &|rect| (rect.0 - left).abs() < 30, "the new connector is framed");
    worker.drag_graphic(&doc, end(&drawn, '1'), (cm(4.0), cm(6.5))).expect("start");
    let second = settle(&mut worker, &|rect| (rect.0 - first.0).abs() > 30, "the frame follows the start");
    let half = saved(&mut worker, "half.fodp");
    assert_ne!(end(&half, '1'), end(&drawn, '1'), "the start moved: {half}");
    worker.drag_graphic(&doc, end(&half, '2'), (cm(18.0), cm(6.5))).expect("end");
    settle(&mut worker, &|rect| (rect.2 - second.2).abs() > 30, "the frame follows the end");
    let glued = saved(&mut worker, "glued.fodp");
    let (start, finish) = (attr(&glued, "draw:start-shape"), attr(&glued, "draw:end-shape"));
    assert!(start.is_some() && finish.is_some() && start != finish, "{glued}");
    let state = worker.now(&doc).expect("the state");
    worker
        .run(&state, "object.select", &["1", "false"])
        .expect("selects");
    worker.run(&doc, "object.x", &[&cm(20.0).to_string()]).expect("moves");
    let moved = saved(&mut worker, "moved.fodp");
    assert_eq!(
        (attr(&moved, "draw:start-shape"), attr(&moved, "draw:end-shape")),
        (start, finish),
        "still glued"
    );
    let shift = end(&moved, '2').0 - end(&glued, '2').0;
    assert!((shift - cm(4.0)).abs() < cm(0.1), "the end moved {shift} twips");
}
