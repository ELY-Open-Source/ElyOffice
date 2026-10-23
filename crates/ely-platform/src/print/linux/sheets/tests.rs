use lopdf::{Document, Object, Stream, content::Content, dictionary};

use super::{Choices, Set, Shown, compose};
use crate::print::linux::choices::Order;

/// Pages with these boxes, each with these extra entries.
fn pages(boxes: &[([f32; 4], lopdf::Dictionary)]) -> (tempfile::TempDir, std::path::PathBuf) {
    let mut document = Document::with_version("1.7");
    let tree = document.new_object_id();
    let kids: Vec<Object> = boxes
        .iter()
        .map(|(corners, extra)| {
            let content = document.add_object(Stream::new(dictionary! {}, b"0 g 0 0 10 10 re f".to_vec()));
            let media: Vec<Object> = corners.iter().map(|&n| n.into()).collect();
            let mut page = dictionary! { "Type" => "Page", "Parent" => tree, "MediaBox" => media, "Contents" => content };
            page.extend(extra);
            document.add_object(page).into()
        })
        .collect();
    let count = kids.len() as i64;
    document.objects.insert(tree, dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }.into());
    let catalog = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => tree });
    document.trailer.set("Root", catalog);
    let folder = tempfile::tempdir().expect("a folder");
    let pdf = folder.path().join("pages.pdf");
    document.save(&pdf).expect("saves");
    (folder, pdf)
}

fn plain(box_: [f32; 4]) -> ([f32; 4], lopdf::Dictionary) {
    (box_, dictionary! {})
}

fn choices(sheet: (f64, f64)) -> Choices {
    Choices {
        ranges: None,
        set: Set::All,
        reverse: false,
        copies: 1,
        collate: true,
        duplex: false,
        scale: 1.0,
        up: 1,
        order: Order { across_first: true, leftward: false, upward: false },
        sheet,
        upside_down: false,
    }
}

/// A sheet's size; each form's width and placed corners.
type Sheet = ((i64, i64), Vec<(i64, [i64; 4])>);

fn sheets(bytes: &[u8]) -> Vec<Sheet> {
    let document = Document::load_mem(bytes).expect("a PDF");
    let round = |n: f64| n.round() as i64;
    document
        .get_pages()
        .into_values()
        .map(|page| {
            let dict = document.get_dictionary(page).expect("a page");
            let media: Vec<f64> = dict.get(b"MediaBox").and_then(Object::as_array).expect("a box").iter().map(|n| f64::from(n.as_float().expect("a number"))).collect();
            let content = Content::decode(&document.get_page_content(page)).expect("content");
            let objects = dict.get(b"Resources").and_then(Object::as_dict).and_then(|r| r.get(b"XObject")).and_then(Object::as_dict).expect("forms");
            let mut matrix = [1.0; 6];
            let mut placed = Vec::new();
            for op in content.operations {
                match op.operator.as_str() {
                    "cm" => matrix = op.operands.iter().map(|n| f64::from(n.as_float().expect("a number"))).collect::<Vec<_>>().try_into().expect("six"),
                    "Do" => {
                        let form = objects.get(op.operands[0].as_name().expect("a name")).and_then(Object::as_reference).expect("a form");
                        let bounds: Vec<f64> = document.get_object(form).and_then(Object::as_stream).expect("a stream").dict.get(b"BBox").and_then(Object::as_array).expect("a box").iter().map(|n| f64::from(n.as_float().expect("a number"))).collect();
                        let m = super::Matrix(matrix);
                        let corners = [(bounds[0], bounds[1]), (bounds[2], bounds[3])].map(|corner| m.apply(corner));
                        let (x0, x1) = (corners[0].0.min(corners[1].0), corners[0].0.max(corners[1].0));
                        let (y0, y1) = (corners[0].1.min(corners[1].1), corners[0].1.max(corners[1].1));
                        placed.push((round(bounds[2] - bounds[0]), [x0, y0, x1, y1].map(round)));
                    }
                    _ => {}
                }
            }
            ((round(media[2]), round(media[3])), placed)
        })
        .collect()
}

#[test]
fn a_user_unit_scales_the_page() {
    let (_folder, pdf) = pages(&[([0.0, 0.0, 421.0, 297.5], dictionary! { "UserUnit" => 2 })]);
    let document = Document::load(&pdf).expect("loads");
    let page = document.get_pages()[&1];
    assert_eq!(Shown::of(&document, page).expect("reads").size(), (842.0, 595.0));
    // On A4 landscape it fills the sheet, from its corner.
    let laid = sheets(&compose(&pdf, &choices((842.0, 595.0))).expect("composes"));
    assert_eq!(laid, [((842, 595), vec![(421, [0, 0, 842, 595])])]);
}

#[test]
fn chosen_pages_print_scaled_from_the_top_left() {
    let (_folder, pdf) = pages(&[plain([0.0, 0.0, 595.0, 842.0]), plain([0.0, 0.0, 420.0, 595.0]), plain([0.0, 0.0, 300.0, 300.0])]);
    let only_second = Choices { ranges: Some(vec![(1, 1)]), scale: 0.5, ..choices((595.0, 842.0)) };
    let laid = sheets(&compose(&pdf, &only_second).expect("composes"));
    assert_eq!(laid, [((595, 842), vec![(420, [0, 545, 210, 842])])]);
}

#[test]
fn a_page_that_fits_only_turned_turns_left() {
    let (_folder, pdf) = pages(&[plain([0.0, 0.0, 842.0, 595.0])]);
    let bytes = compose(&pdf, &choices((595.0, 842.0))).expect("composes");
    assert_eq!(sheets(&bytes), [((595, 842), vec![(842, [0, 0, 595, 842])])]);
    // Its top left lands at the sheet's bottom left.
    let document = Document::load_mem(&bytes).expect("a PDF");
    let content = Content::decode(&document.get_page_content(document.get_pages()[&1])).expect("content");
    let cm = content.operations.iter().find(|op| op.operator == "cm").expect("a matrix");
    let m: Vec<f64> = cm.operands.iter().map(|n| f64::from(n.as_float().expect("a number"))).collect();
    let corner = super::Matrix(m.try_into().expect("six")).apply((0.0, 595.0));
    assert_eq!((corner.0.round(), corner.1.round()), (0.0, 0.0));
}

#[test]
fn a_crop_box_past_the_media_box_is_cut_to_it() {
    let crop = |corners: [i64; 4]| -> Object { corners.map(Object::from).to_vec().into() };
    let (_folder, pdf) = pages(&[([0.0, 0.0, 612.0, 792.0], dictionary! { "CropBox" => crop([0, 0, 700, 792]) })]);
    // The form's box clips at 612: x 650 stays hidden.
    let laid = sheets(&compose(&pdf, &choices((612.0, 792.0))).expect("composes"));
    assert_eq!(laid, [((612, 792), vec![(612, [0, 0, 612, 792])])]);
    let (_folder, pdf) = pages(&[([0.0, 0.0, 612.0, 792.0], dictionary! { "CropBox" => crop([700, 0, 800, 792]) })]);
    assert!(compose(&pdf, &choices((612.0, 792.0))).is_err(), "nothing left to show");
}

#[test]
fn a_rotated_page_shows_turned() {
    let (_folder, pdf) = pages(&[([0.0, 0.0, 595.0, 842.0], dictionary! { "Rotate" => 90 })]);
    let document = Document::load(&pdf).expect("loads");
    let shown = Shown::of(&document, document.get_pages()[&1]).expect("reads");
    assert_eq!(shown.size(), (842.0, 595.0));
    // Turned clockwise, the page's top left becomes its top right.
    assert_eq!(shown.matrix().apply((0.0, 842.0)), (842.0, 595.0));
}

#[test]
fn too_large_a_page_fails() {
    let (_folder, pdf) = pages(&[plain([0.0, 0.0, 1000.0, 600.0])]);
    let error = compose(&pdf, &choices((595.0, 842.0))).expect_err("does not fit");
    assert!(error.to_string().contains("page 1 at 100% is larger than the 210 × 297 mm paper"), "{error}");
    let (_folder, pdf) = pages(&[plain([0.0, 0.0, 595.0, 842.0])]);
    assert!(compose(&pdf, &Choices { scale: 2.0, ..choices((595.0, 842.0)) }).is_err(), "scaled past the paper");
    // A4 is 595.28 points wide: 600 would lose its edge.
    let a4 = (210.0 * 72.0 / 25.4, 297.0 * 72.0 / 25.4);
    let (_folder, pdf) = pages(&[plain([0.0, 0.0, 600.0, 842.0])]);
    assert!(compose(&pdf, &choices(a4)).is_err(), "a page 1.6 mm wider than A4");
    // A4 in whole points still takes an A4 page.
    let (_folder, pdf) = pages(&[plain([0.0, 0.0, a4.0 as f32, a4.1 as f32])]);
    assert!(compose(&pdf, &choices((595.0, 842.0))).is_ok(), "rounding is no overflow");
}

#[test]
fn an_open_gtk_range_runs_to_the_last_page() {
    use zbus::zvariant::{OwnedValue, Value};
    let widths = [100.0, 110.0, 120.0];
    let boxes: Vec<_> = widths.iter().map(|&w| plain([0.0, 0.0, w, 100.0])).collect();
    let (_folder, pdf) = pages(&boxes);
    let owned = |value: Value<'static>| OwnedValue::try_from(value).expect("owned");
    let settings = [("print-pages", "ranges"), ("page-ranges", "1--1")].map(|(key, value)| (key.to_string(), owned(Value::from(value))));
    let setup = [("Width", Value::from(210.0)), ("Height", Value::from(297.0)), ("Orientation", Value::from("portrait"))];
    let setup = setup.map(|(key, value)| (key.to_string(), owned(value)));
    let chosen = super::super::choices::chosen(&settings.into(), &setup.into()).expect("reads");
    let printed: Vec<i64> = sheets(&compose(&pdf, &chosen).expect("composes")).iter().map(|sheet| sheet.1[0].0).collect();
    assert_eq!(printed, [110, 120], "pages two and three");
}

#[test]
fn four_up_fills_cells_in_the_chosen_order() {
    let sizes = [100.0, 110.0, 120.0, 130.0, 140.0];
    let boxes: Vec<_> = sizes.iter().map(|&w| plain([0.0, 0.0, w, w * 1.5])).collect();
    let (_folder, pdf) = pages(&boxes);
    let up = Choices { up: 4, ..choices((400.0, 600.0)) };
    let laid = sheets(&compose(&pdf, &up).expect("composes"));
    assert_eq!(laid.len(), 2, "five pages, four a sheet");
    let corner = |(width, [x0, _, _, y1]): &(i64, [i64; 4])| (*width, x0 / 200, (600 - y1) / 300);
    let cells: Vec<_> = laid[0].1.iter().map(corner).collect();
    assert_eq!(cells, [(100, 0, 0), (110, 1, 0), (120, 0, 1), (130, 1, 1)]);
    // Every page fills its 200 × 300 cell, centred.
    assert_eq!(laid[1].1, [(140, [0, 300, 200, 600])]);
    let upward = Choices { order: Order { across_first: false, leftward: true, upward: true }, ..up };
    let laid = sheets(&compose(&pdf, &upward).expect("composes"));
    let cells: Vec<_> = laid[0].1.iter().map(corner).collect();
    assert_eq!(cells, [(100, 1, 1), (110, 1, 0), (120, 0, 1), (130, 0, 0)]);
}

#[test]
fn sets_order_and_copies_choose_the_sheets() {
    let widths = [100.0, 110.0, 120.0];
    let boxes: Vec<_> = widths.iter().map(|&w| plain([0.0, 0.0, w, 100.0])).collect();
    let (_folder, pdf) = pages(&boxes);
    // A blank back has no form: width 0.
    let order = |chosen: Choices| -> Vec<i64> {
        sheets(&compose(&pdf, &chosen).expect("composes")).iter().map(|sheet| sheet.1.first().map_or(0, |form| form.0)).collect()
    };
    let base = choices((595.0, 842.0));
    assert_eq!(order(Choices { set: Set::Odd, ..choices((595.0, 842.0)) }), [100, 120]);
    assert_eq!(order(Choices { set: Set::Even, ..choices((595.0, 842.0)) }), [110]);
    assert_eq!(order(Choices { reverse: true, ..choices((595.0, 842.0)) }), [120, 110, 100]);
    assert_eq!(order(Choices { copies: 2, ..choices((595.0, 842.0)) }), [100, 110, 120, 100, 110, 120]);
    assert_eq!(order(Choices { copies: 2, collate: false, ..base }), [100, 100, 110, 110, 120, 120]);
    // Two-sided: each copy starts a fresh sheet.
    let both = || Choices { duplex: true, copies: 2, ..choices((595.0, 842.0)) };
    assert_eq!(order(both()), [100, 110, 120, 0, 100, 110, 120, 0]);
    assert_eq!(order(Choices { collate: false, ..both() }), [100, 110, 100, 110, 120, 0, 120, 0]);
    assert_eq!(order(Choices { reverse: true, copies: 1, ..both() }), [120, 0, 100, 110]);
    let none = compose(&pdf, &Choices { ranges: Some(vec![(5, 9)]), ..choices((595.0, 842.0)) });
    assert!(none.is_err(), "no chosen page is in the document");
}

#[test]
fn printable_annotations_are_drawn_where_they_sit() {
    let mut document = Document::with_version("1.7");
    let look = document.add_object(Stream::new(
        dictionary! { "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 10.into(), 20.into()] },
        b"0 g 0 0 10 20 re f".to_vec(),
    ));
    let rect = |x: i64| -> Object { vec![x.into(), 100.into(), (x + 30).into(), 160.into()].into() };
    let shown = dictionary! { "Subtype" => "Square", "F" => 4, "Rect" => rect(50), "AP" => dictionary! { "N" => look } };
    let unprinted = dictionary! { "Subtype" => "Square", "F" => 0, "Rect" => rect(200), "AP" => dictionary! { "N" => look } };
    let annots: Vec<Object> = vec![document.add_object(shown).into(), document.add_object(unprinted).into()];
    let tree = document.new_object_id();
    let content = document.add_object(Stream::new(dictionary! {}, Vec::new()));
    let media: Vec<Object> = vec![0.into(), 0.into(), 595.into(), 842.into()];
    let page = document.add_object(dictionary! { "Type" => "Page", "Parent" => tree, "MediaBox" => media, "Contents" => content, "Annots" => annots });
    document.objects.insert(tree, dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }.into());
    let looks = super::appearances(&document, page).expect("reads");
    assert_eq!(looks.len(), 1, "only the printable one");
    let (id, place) = looks[0];
    assert_eq!(id, look);
    assert_eq!((place.apply((0.0, 0.0)), place.apply((10.0, 20.0))), ((50.0, 100.0), (80.0, 160.0)));
}

/// One A4 page, turned `rotate`: its annotations and own forms.
fn annotated(rotate: i64, build: impl FnOnce(&mut Document) -> (Vec<lopdf::Dictionary>, lopdf::Dictionary)) -> (Document, lopdf::ObjectId) {
    let mut document = Document::with_version("1.7");
    let (annots, own) = build(&mut document);
    let annots: Vec<Object> = annots.into_iter().map(|annot| document.add_object(annot).into()).collect();
    let tree = document.new_object_id();
    let content = document.add_object(Stream::new(dictionary! {}, b"/ElyAnnot0 Do".to_vec()));
    let media: Vec<Object> = vec![0.into(), 0.into(), 595.into(), 842.into()];
    let resources = dictionary! { "XObject" => own };
    let page = dictionary! { "Type" => "Page", "Parent" => tree, "MediaBox" => media, "Contents" => content, "Annots" => annots, "Rotate" => rotate, "Resources" => resources };
    let page = document.add_object(page);
    document.objects.insert(tree, dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }.into());
    (document, page)
}

fn square(document: &mut Document, flags: i64) -> lopdf::Dictionary {
    let look = document.add_object(Stream::new(
        dictionary! { "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 100.into(), 80.into()] },
        b"1 0 0 rg 0 0 100 80 re f".to_vec(),
    ));
    let rect: Vec<Object> = vec![50.into(), 100.into(), 150.into(), 180.into()];
    dictionary! { "Subtype" => "Square", "F" => flags, "Rect" => rect, "AP" => dictionary! { "N" => look } }
}

#[test]
fn a_no_rotate_annotation_stays_upright_about_its_top_left() {
    let (document, page) = annotated(90, |document| (vec![square(document, 20)], dictionary! {}));
    let (_, place) = super::appearances(&document, page).expect("reads")[0];
    let shown = Shown::of(&document, page).expect("reads").matrix();
    let at = |corner| {
        let (x, y) = shown.apply(place.apply(corner));
        (x.round(), y.round())
    };
    // Shown: its top left stays put, its top runs right.
    assert_eq!(at((0.0, 80.0)), (180.0, 545.0));
    assert_eq!(at((100.0, 80.0)), (280.0, 545.0));
    assert_eq!(at((0.0, 0.0)), (180.0, 465.0));
    // Without the flag, it turns with the page.
    let (document, page) = annotated(90, |document| (vec![square(document, 4)], dictionary! {}));
    let (_, place) = super::appearances(&document, page).expect("reads")[0];
    let (x, y) = shown.apply(place.apply((0.0, 80.0)));
    assert_eq!((x.round(), y.round()), (180.0, 545.0));
    let (x, y) = shown.apply(place.apply((100.0, 80.0)));
    assert_eq!((x.round(), y.round()), (180.0, 445.0));
}

#[test]
fn an_annotation_name_leaves_the_page_its_own() {
    let mut blue = None;
    let (mut document, page) = annotated(0, |document| {
        let form = dictionary! { "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 1.into(), 1.into()] };
        let own = *blue.insert(document.add_object(Stream::new(form, b"0 0 1 rg".to_vec())));
        (vec![square(document, 4)], dictionary! { "ElyAnnot0" => own })
    });
    let blue = blue.expect("made");
    let form = super::form(&mut document, page).expect("a form");
    let form = document.get_object(form).and_then(Object::as_stream).expect("a stream");
    let objects = form.dict.get(b"Resources").and_then(Object::as_dict).and_then(|r| r.get(b"XObject")).and_then(Object::as_dict).expect("forms");
    assert_eq!(objects.get(b"ElyAnnot0").and_then(Object::as_reference).expect("kept"), blue);
    let drawn = String::from_utf8(form.decompressed_content().expect("content")).expect("text");
    assert!(drawn.contains("/ElyAnnot0 Do") && drawn.contains("/ElyAnnot1 Do"), "{drawn}");
    let added = objects.get(b"ElyAnnot1").and_then(Object::as_reference).expect("the annotation's");
    assert_ne!(added, blue);
}

#[test]
fn a_printable_annotation_without_a_look_fails() {
    let rect: Vec<Object> = vec![50.into(), 100.into(), 150.into(), 180.into()];
    let bare = dictionary! { "Subtype" => "Square", "F" => 4, "Rect" => rect.clone() };
    let (document, page) = annotated(0, |_| (vec![bare], dictionary! {}));
    let error = super::appearances(&document, page).expect_err("cannot be drawn");
    assert!(error.to_string().contains("a printable Square annotation has no appearance"), "{error}");
    // A link draws nothing; an unset state draws nothing.
    let link = dictionary! { "Subtype" => "Link", "F" => 4, "Rect" => rect.clone() };
    let (mut document, page) = annotated(0, |_| (vec![link], dictionary! {}));
    assert!(super::appearances(&document, page).expect("reads").is_empty());
    let on = document.add_object(Stream::new(dictionary! { "BBox" => vec![0.into(), 0.into(), 1.into(), 1.into()] }, Vec::new()));
    let off = dictionary! { "Subtype" => "Widget", "F" => 4, "Rect" => rect, "AS" => "Off", "AP" => dictionary! { "N" => dictionary! { "Yes" => on } } };
    assert_eq!(super::appearance(&document, &off).expect("reads"), None);
}
