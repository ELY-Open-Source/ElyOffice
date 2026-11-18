use super::*;

const DECK: &str = r##"<office:document-content xmlns:office="o" xmlns:draw="d" xmlns:svg="s" xmlns:text="t" xmlns:xlink="x" xmlns:presentation="p" xmlns:script="c"><office:body><office:presentation>
<draw:page draw:name="One">
 <draw:frame svg:x="2cm" svg:y="2cm" svg:width="6cm" svg:height="4cm"><draw:text-box><text:p><text:a xlink:href="#Two">next</text:a></text:p></draw:text-box></draw:frame>
 <draw:frame svg:x="10cm" svg:y="2cm" svg:width="6cm" svg:height="4cm"><draw:text-box><text:p><text:a xlink:href="https://example.com/a?b=1&amp;c=2">web</text:a></text:p></draw:text-box></draw:frame>
 <draw:custom-shape svg:x="1in" svg:y="1in" svg:width="20mm" svg:height="20mm"><presentation:event-listeners><presentation:event-listener script:event-name="dom:click" presentation:action="next-page"/></presentation:event-listeners></draw:custom-shape>
 <draw:frame svg:x="0cm" svg:y="10cm" svg:width="2cm" svg:height="2cm"><draw:text-box><text:p><text:a xlink:href="#Nowhere">gone</text:a></text:p></draw:text-box></draw:frame>
 <presentation:notes><draw:page-thumbnail/><draw:frame svg:x="0cm" svg:y="0cm" svg:width="30cm" svg:height="30cm"><draw:text-box><text:p><text:a xlink:href="https://notes.example">n</text:a></text:p></draw:text-box></draw:frame></presentation:notes>
</draw:page>
<draw:page draw:name="Two">
 <draw:g><draw:rect svg:x="1cm" svg:y="1cm" svg:width="2cm" svg:height="2cm"/><draw:rect svg:x="4cm" svg:y="1cm" svg:width="2cm" svg:height="2cm"/><presentation:event-listeners><presentation:event-listener script:event-name="dom:click" presentation:action="show" xlink:href="#One"/></presentation:event-listeners></draw:g>
 <draw:rect svg:x="4cm" svg:y="1cm" svg:width="1cm" svg:height="1cm"><presentation:event-listeners><presentation:event-listener script:event-name="dom:click" presentation:action="stop"/><presentation:event-listener script:event-name="dom:click" presentation:action="execute" xlink:href="evil.sh"/><presentation:event-listener script:event-name="dom:mouse-over" presentation:action="last-page"/></presentation:event-listeners></draw:rect>
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
    assert_eq!(slides[1].len(), 2, "only clicks that move about are kept, and no program");
    let slide = (28_000.0, 15_750.0);
    let point = |x: f32, y: f32| at(&slides[1], slide, (x / slide.0, y / slide.1));
    // 4.5 cm across, 1.5 down: inside the group and the small shape.
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
