use super::*;

const DECK: &str = r##"<office:document-content xmlns:office="o" xmlns:draw="d" xmlns:svg="s" xmlns:text="t" xmlns:xlink="x" xmlns:presentation="p" xmlns:script="c"><office:body><office:presentation>
<draw:page draw:name="One">
 <draw:frame svg:x="2cm" svg:y="2cm" svg:width="6cm" svg:height="4cm"><draw:text-box><text:p><text:a xlink:href="#Two">next</text:a></text:p></draw:text-box></draw:frame>
 <draw:frame svg:x="10cm" svg:y="2cm" svg:width="6cm" svg:height="4cm"><draw:text-box><text:p><text:a xlink:href="https://example.com/a?b=1&amp;c=2">web</text:a></text:p></draw:text-box></draw:frame>
 <draw:custom-shape svg:x="1in" svg:y="1in" svg:width="20mm" svg:height="20mm"><office:event-listeners><presentation:event-listener script:event-name="dom:click" presentation:action="next-page"/></office:event-listeners></draw:custom-shape>
 <draw:frame svg:x="0cm" svg:y="10cm" svg:width="2cm" svg:height="2cm"><draw:text-box><text:p><text:a xlink:href="#Nowhere">gone</text:a></text:p></draw:text-box></draw:frame>
 <presentation:notes><draw:page-thumbnail/><draw:frame svg:x="0cm" svg:y="0cm" svg:width="30cm" svg:height="30cm"><draw:text-box><text:p><text:a xlink:href="https://notes.example">n</text:a></text:p></draw:text-box></draw:frame></presentation:notes>
</draw:page>
<draw:page draw:name="Two">
 <draw:g><draw:rect svg:x="1cm" svg:y="1cm" svg:width="2cm" svg:height="2cm"/><draw:rect svg:x="4cm" svg:y="1cm" svg:width="2cm" svg:height="2cm"/><office:event-listeners><presentation:event-listener script:event-name="dom:click" presentation:action="show" xlink:href="#One"/></office:event-listeners></draw:g>
 <draw:rect svg:x="4cm" svg:y="1cm" svg:width="1cm" svg:height="1cm"><office:event-listeners><presentation:event-listener script:event-name="dom:click" presentation:action="stop"/><presentation:event-listener script:event-name="dom:mouse-over" presentation:action="last-page"/></office:event-listeners></draw:rect>
 <draw:rect svg:x="8cm" svg:y="1cm" svg:width="1cm" svg:height="1cm"><office:event-listeners><presentation:event-listener script:event-name="dom:click" presentation:action="execute" xlink:href="evil.sh"/></office:event-listeners></draw:rect>
 <draw:rect svg:x="10cm" svg:y="1cm" svg:width="1cm" svg:height="1cm"><office:event-listeners><script:event-listener script:language="ooo:script" script:event-name="dom:click" xlink:href="vnd.sun.star.script:Standard.M.Run?language=Basic&amp;location=document"/></office:event-listeners></draw:rect>
 <draw:rect svg:x="12cm" svg:y="1cm" svg:width="1cm" svg:height="1cm"><office:event-listeners><presentation:event-listener script:event-name="dom:click" presentation:action="none"/></office:event-listeners></draw:rect>
 <draw:frame svg:x="0cm" svg:y="0cm" svg:width="1cm" svg:height="1cm"><draw:image xlink:href="pic.png"/></draw:frame>
</draw:page>
</office:presentation></office:body></office:document-content>"##;

fn deck() -> Vec<Vec<Link>> {
    parse(DECK.as_bytes()).expect("a deck parses")
}

#[test]
fn text_links_and_click_actions_land_on_their_slides() {
    let slides = deck();
    assert_eq!(slides.len(), 2, "one list per slide");
    let first: Vec<_> = slides[0].iter().map(|link| (link.bounds, link.to.clone())).collect();
    assert!(first.contains(&((2000, 2000, 6000, 4000), Goto::Slide(1))), "a link to Two: {first:?}");
    let web = Goto::Address("https://example.com/a?b=1&c=2".into());
    assert!(first.contains(&((10000, 2000, 6000, 4000), web)), "the address is unescaped");
    assert!(first.contains(&((2540, 2540, 2000, 2000), Goto::Next)), "inches and millimetres");
    assert!(first.contains(&((0, 10000, 2000, 2000), Goto::Broken("Nowhere".into()))));
    assert_eq!(first.len(), 4, "the notes page adds none: {first:?}");
}

#[test]
fn a_group_covers_its_children_and_the_inner_shape_wins() {
    let slides = deck();
    let group = (1000, 1000, 5000, 2000);
    assert!(slides[1].iter().any(|link| link.bounds == group && link.to == Goto::Slide(0)));
    assert_eq!(slides[1].len(), 4, "a program and a macro stay, refused: {:?}", slides[1]);
    let refused: Vec<_> = slides[1].iter().filter(|link| link.to == Goto::Refused).map(|link| link.bounds.0).collect();
    assert_eq!(refused, [8000, 10000], "the action none adds nothing");
    let slide = (28_000.0, 15_750.0);
    let point = |x: f32, y: f32| at(&slides[1], slide, (x / slide.0, y / slide.1), 1400.0);
    // Inside both the group and the small shape.
    assert_eq!(point(4500.0, 1500.0), Some(&Goto::End));
    // 2 cm across: the group only.
    assert_eq!(point(2000.0, 1500.0), Some(&Goto::Slide(0)));
    assert_eq!(point(20_000.0, 1500.0), None, "off every link");
    assert_eq!(point(500.0, 500.0), None, "a picture without a link");
}

#[test]
fn a_bad_length_is_an_error() {
    for bad in ["<draw:page><draw:frame svg:x=\"2\" svg:y=\"0cm\" svg:width=\"1cm\" svg:height=\"1cm\"/></draw:page>",
        "<draw:page><draw:frame svg:x=\"2em\" svg:y=\"0cm\" svg:width=\"1cm\" svg:height=\"1cm\"/></draw:page>"] {
        assert!(parse(bad.as_bytes()).is_err(), "{bad}");
    }
}

/// Home links to Two, a file, and attaching mail.
const PLAY: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
 xmlns:xlink="http://www.w3.org/1999/xlink"
 xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:body><office:presentation>
 <draw:page draw:name="Home">
  <draw:frame svg:x="2cm" svg:y="2cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p><text:a xlink:href="#Two" xlink:type="simple">Two</text:a></text:p></draw:text-box></draw:frame>
  <draw:frame svg:x="2cm" svg:y="6cm" svg:width="10cm" svg:height="2cm"><draw:text-box><text:p><text:a xlink:href="file:///etc/passwd" xlink:type="simple">File</text:a></text:p></draw:text-box></draw:frame>
  <draw:frame svg:x="2cm" svg:y="9cm" svg:width="10cm" svg:height="2cm"><draw:text-box><text:p><text:a xlink:href="mailto:x@a.example?attach=/etc/passwd" xlink:type="simple">Mail</text:a></text:p></draw:text-box></draw:frame>
 </draw:page>
 <draw:page draw:name="Two">
  <draw:frame svg:x="2cm" svg:y="2cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p><text:a xlink:href="https://example.com/" xlink:type="simple">Web</text:a></text:p></draw:text-box></draw:frame>
 </draw:page>
 </office:presentation></office:body>
</office:document>
"##;

/// Clicks the audience screen at a point in 1/100 mm.
fn click(cx: &mut gpui::TestAppContext, show: &gpui::Entity<Show>, (x, y): (f32, f32)) {
    use gpui::{Modifiers, MouseButton, point};
    let window = cx.update(|app| show.read(app).windows[0]);
    let cx = &mut gpui::VisualTestContext::from_window(window, cx);
    let (room, slide) = cx.update(|window, app| (window.viewport_size(), show.read(app).slide));
    let size = super::super::stage::fit(slide, (room.width, room.height));
    let at = point(
        (room.width - size.0) / 2.0 + size.0 * (x / slide.0),
        (room.height - size.1) / 2.0 + size.1 * (y / slide.1),
    );
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
}

#[test]
fn a_show_click_follows_slides_and_the_web_never_files() {
    use ely_office::prelude::*;

    use super::super::steps::Place;
    use crate::tests::{settle, update};
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = gpui::TestAppContext::single();
    let Some(deck) = crate::tests::deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let from = dir.path().join("play.fodp");
    std::fs::write(&from, PLAY).expect("writes");
    update(&mut cx, &deck, |deck, cx| deck.open_file(from.clone(), cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.list.len() == 2));
    update(&mut cx, &deck, |deck, cx| deck.start_show(false, cx));
    settle(&cx, |app| deck.read(app).show.is_some());
    let show = cx.update(|app| deck.read(app).show.clone().expect("playing"));
    settle(&cx, |app| show.read(app).stack(0).is_some());
    let refused = cx.update(|app| Deck::text(app, "office.link.refused", &[]));
    for (y, what) in [(7000.0, "a file"), (10_000.0, "mail attaching a file")] {
        click(&mut cx, &show, (7000.0, y));
        assert_eq!(cx.update(|app| show.read(app).place), Place::Slide { part: 0, clicks: 0 }, "{what}");
        assert_eq!(cx.opened_url(), None, "{what} never reaches the opener");
        assert_eq!(cx.update(|app| show.read(app).notice.clone()), Some(refused.clone()), "{what}");
    }
    click(&mut cx, &show, (7000.0, 3500.0));
    settle(&cx, |app| show.read(app).place == Place::Slide { part: 1, clicks: 0 });
    click(&mut cx, &show, (7000.0, 3500.0));
    assert_eq!(cx.opened_url().as_deref(), Some("https://example.com/"));
    assert_eq!(cx.update(|app| show.read(app).place), Place::Slide { part: 1, clicks: 0 }, "a link does not advance");
}

#[test]
fn a_next_page_shape_passes_pending_clicks_and_a_program_stays_put() {
    use super::super::steps::Place;
    use crate::tests::{objects::blank, play::three_slides, settle, update};
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = gpui::TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    update(&mut cx, &deck, |deck, cx| deck.start_show(false, cx));
    settle(&cx, |app| deck.read(app).show.is_some());
    let show = cx.update(|app| deck.read(app).show.clone().expect("playing"));
    settle(&cx, |app| show.read(app).stack(1).is_some());
    let width = cx.update(|app| show.read(app).slide.0);
    cx.update(|app| {
        show.update(app, |show, cx| {
            // Slide two: a program top left, next-page top right.
            let right = width as i64 - 2000;
            show.links[1] = vec![
                Link { bounds: (0, 0, 2000, 2000), to: Goto::Refused, line: false },
                Link { bounds: (right, 0, 2000, 2000), to: Goto::Next, line: false },
            ];
            show.next(cx);
        })
    });
    let on_two = Place::Slide { part: 1, clicks: 0 };
    assert_eq!(cx.update(|app| show.read(app).place), on_two, "its fade-in waits for a click");
    let refused = cx.update(|app| Deck::text(app, "office.link.refused", &[]));
    click(&mut cx, &show, (1000.0, 1000.0));
    assert_eq!(cx.update(|app| show.read(app).place), on_two, "a program neither runs nor steps");
    assert_eq!(cx.update(|app| show.read(app).notice.clone()), Some(refused));
    assert_eq!(cx.opened_url(), None);
    click(&mut cx, &show, (width - 1000.0, 1000.0));
    settle(&cx, |app| show.read(app).place == Place::End);
}

/// Linked: a frame to turn, a sheared shape, a line.
const TURNED: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
 xmlns:xlink="http://www.w3.org/1999/xlink"
 xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
 xmlns:presentation="urn:oasis:names:tc:opendocument:xmlns:presentation:1.0"
 xmlns:script="urn:oasis:names:tc:opendocument:xmlns:script:1.0"
 xmlns:dom="http://www.w3.org/2001/xml-events"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:body><office:presentation>
 <draw:page draw:name="One">
  <draw:frame svg:x="2cm" svg:y="2cm" svg:width="8cm" svg:height="2cm"><draw:text-box><text:p><text:a xlink:href="#Two">Two</text:a></text:p></draw:text-box></draw:frame>
  <draw:custom-shape svg:width="6cm" svg:height="3cm" draw:transform="skewX (-0.4) translate (14cm 2cm)"><text:p><text:a xlink:href="https://example.com/">Web</text:a></text:p><draw:enhanced-geometry svg:viewBox="0 0 21600 21600" draw:type="rectangle" draw:enhanced-path="M 0 0 L 21600 0 21600 21600 0 21600 Z N"/></draw:custom-shape>
  <draw:line svg:x1="2cm" svg:y1="9cm" svg:x2="12cm" svg:y2="12cm"><office:event-listeners><presentation:event-listener script:event-name="dom:click" presentation:action="next-page"/></office:event-listeners></draw:line>
 </draw:page>
 <draw:page draw:name="Two"/>
 </office:presentation></office:body>
</office:document>
"##;

#[test]
fn turned_sheared_and_line_links_cover_what_the_kernel_draws() {
    use ely_office::prelude::*;
    use crate::tests::{settle, update};
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = gpui::TestAppContext::single();
    let Some(deck) = crate::tests::deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let from = dir.path().join("turned.fodp");
    std::fs::write(&from, TURNED).expect("writes");
    update(&mut cx, &deck, |deck, cx| deck.open_file(from.clone(), cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.slides.list.len() == 2));
    let copy = dir.path().join("copy.odp");
    let (drawn, links, transforms) = cx.update(|app| {
        let deck = deck.read(app);
        let doc = deck.active_tab().expect("a tab").doc.clone();
        let mut worker = deck.core.worker.lock().expect("the worker lock");
        let objects = |worker: &mut ely_lok::Worker| {
            ely_lok::show::objects(&worker.values(&doc, ely_lok::show::OBJECTS).expect("reads")).expect("parses").objects
        };
        // The kernel turns the first frame 30° about its middle.
        let (x, y, w, h) = objects(&mut worker)[0].rect;
        let twips = crate::object::twips;
        let state = worker.now(&doc).expect("the state");
        worker.run(&state, "object.select", &["0", "false"]).expect("selects");
        let (cx_, cy_) = (twips(x + w / 2).to_string(), twips(y + h / 2).to_string());
        worker.run(&doc, "object.rotate", &["3000", &cx_, &cy_]).expect("rotates");
        let drawn: Vec<_> = objects(&mut worker).into_iter().map(|object| object.rect).collect();
        doc.save_as(&mut worker, &ely_lok::file_url(&copy), "odp").expect("saves");
        let xml = ely_office::files::part(&copy, "content.xml").expect("reads").expect("content.xml");
        let transforms = String::from_utf8_lossy(&xml).matches("draw:transform=").count();
        (drawn, super::read(&copy).expect("links"), transforms)
    });
    assert_eq!(transforms, 2, "the kernel wrote the turn and the shear as transforms");
    let page = &links[0];
    let near = |a: (i64, i64, i64, i64), b: (i64, i64, i64, i64)| {
        [(a.0, b.0), (a.1, b.1), (a.2, b.2), (a.3, b.3)].iter().all(|(m, n)| (m - n).abs() <= 3)
    };
    for (at, to) in [(0, Goto::Slide(1)), (1, Goto::Address("https://example.com/".into())), (2, Goto::Next)] {
        let link = page.iter().find(|link| link.to == to).unwrap_or_else(|| panic!("no link to {to:?}: {page:?}"));
        assert!(near(link.bounds, drawn[at]), "{to:?}: {:?}, drawn {:?}", link.bounds, drawn[at]);
    }
    let (x, y, w, h) = drawn[0];
    let slide = (28_000.0, 15_750.0);
    let middle = ((x + w / 2) as f32 / slide.0, (y + h / 2) as f32 / slide.1);
    assert_eq!(at(page, slide, middle, 1400.0), Some(&Goto::Slide(1)), "a click on the turned frame follows it");
}

/// One screen pixel of the shown slide, in 1/100 mm.
fn pixel(cx: &mut gpui::TestAppContext, show: &gpui::Entity<Show>) -> f32 {
    let window = cx.update(|app| show.read(app).windows[0]);
    let cx = &mut gpui::VisualTestContext::from_window(window, cx);
    let (room, slide) = cx.update(|window, app| (window.viewport_size(), show.read(app).slide));
    let size = super::super::stage::fit(slide, (room.width, room.height));
    slide.0 / f32::from(size.0)
}

#[test]
fn a_click_two_pixels_from_a_line_follows_it() {
    use super::super::steps::Place;
    use crate::tests::{objects::blank, play::three_slides, settle, update};
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = gpui::TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    update(&mut cx, &deck, |deck, cx| deck.start_show(false, cx));
    settle(&cx, |app| deck.read(app).show.is_some());
    let show = cx.update(|app| deck.read(app).show.clone().expect("playing"));
    settle(&cx, |app| show.read(app).stack(1).is_some());
    // A hairline across slides one and two, linked to three.
    let line = Link { bounds: (2000, 5000, 10_000, 0), to: Goto::Slide(2), line: true };
    cx.update(|app| show.update(app, |show, _| show.links[..2].iter_mut().for_each(|page| *page = vec![line.clone()])));
    let px = pixel(&mut cx, &show);
    click(&mut cx, &show, (7000.0, 5000.0 - 3.0 * px));
    settle(&cx, |app| show.read(app).place == Place::Slide { part: 1, clicks: 0 });
    click(&mut cx, &show, (7000.0, 5000.0 - px));
    settle(&cx, |app| show.read(app).place == Place::Slide { part: 2, clicks: 0 });
}

#[test]
fn a_raw_address_opens_encoded_and_a_broken_one_says_so() {
    use crate::tests::{objects::blank, play::three_slides, settle, update};
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = gpui::TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    three_slides(&mut cx, &deck);
    update(&mut cx, &deck, |deck, cx| deck.start_show(false, cx));
    settle(&cx, |app| deck.read(app).show.is_some());
    let show = cx.update(|app| deck.read(app).show.clone().expect("playing"));
    settle(&cx, |app| show.read(app).stack(0).is_some());
    let width = cx.update(|app| show.read(app).slide.0);
    let right = width as i64 - 2000;
    cx.update(|app| {
        show.update(app, |show, _| {
            // As a PPTX keeps a typed address.
            show.links[0] = vec![
                Link { bounds: (0, 0, 2000, 2000), to: Goto::Address("https://a.example/x y\"z'w".into()), line: false },
                Link { bounds: (right, 0, 2000, 2000), to: Goto::Address("https://a b/".into()), line: false },
            ];
        })
    });
    click(&mut cx, &show, (1000.0, 1000.0));
    let sent = Some("https://a.example/x%20y%22z'w");
    assert_eq!(cx.opened_url().as_deref(), sent, "the opener gets it encoded");
    click(&mut cx, &show, (width - 1000.0, 1000.0));
    let malformed = cx.update(|app| Deck::text(app, "office.link.malformed", &[]));
    assert_eq!(cx.update(|app| show.read(app).notice.clone()), Some(malformed), "a broken address says so");
    assert_eq!(cx.opened_url().as_deref(), sent, "and never reaches the opener");
}
