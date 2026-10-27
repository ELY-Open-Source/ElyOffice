use super::*;

#[test]
fn formula_callbacks_keep_their_geometry_together() {
    let mut slot = Slot::default();
    record(
        &mut slot,
        CURSOR,
        r#"{"rectangle":"999,888,0,292","relrect":"317,15,0,292","refpoint":"6373,29160"}"#,
    );
    record(
        &mut slot,
        SELECTION,
        "402,0,157,292;600,0,30,292::6373,29160",
    );
    let batch = slot.drain();
    assert!(batch.faults.is_empty(), "{:?}", batch.faults);
    assert_eq!(batch.cursor, Some((317, 15, 0, 292)));
    assert_eq!(batch.selection, [(402, 0, 157, 292), (600, 0, 30, 292)]);
    assert_eq!(batch.text.cursor, Some((6373, 29160)));
    assert_eq!(batch.text.selection, Some((6373, 29160)));
    assert_eq!(
        slot.drain().text,
        batch.text,
        "anchors persist with the snapshot"
    );

    let text = TextAnchors::unpack(&mut crate::ipc::Unpack::new(
        &batch.text.pack(crate::ipc::Pack::new()).bytes(),
    ))
    .unwrap();
    assert_eq!(text, batch.text);

    record(
        &mut slot,
        CURSOR,
        r#"{"rectangle":"10,20,0,30","viewId":"0"}"#,
    );
    record(&mut slot, SELECTION, "10,20,30,40");
    assert_eq!(
        slot.batch.text,
        TextAnchors::default(),
        "legacy text drops the anchors"
    );
    assert_eq!(slot.batch.cursor, Some((10, 20, 0, 30)));
    record(&mut slot, CURSOR, "EMPTY");
    record(&mut slot, SELECTION, "");
    assert!(slot.batch.cursor.is_none());
    assert!(slot.batch.selection.is_empty());
    assert!(slot.batch.faults.is_empty());
}

#[test]
fn malformed_formula_geometry_is_reported_and_cleared() {
    let mut slot = Slot::default();
    record(
        &mut slot,
        CURSOR,
        r#"{"relrect":"1,2,0,4","refpoint":"5,6"}"#,
    );
    record(&mut slot, SELECTION, "1,2,3,4::5,6");
    record(
        &mut slot,
        CURSOR,
        r#"{"relrect":"1,2,0,4","refpoint":"broken"}"#,
    );
    record(&mut slot, SELECTION, "1,2,3,4::broken");
    assert_eq!(slot.batch.faults.len(), 2);
    assert!(slot.batch.cursor.is_none());
    assert!(slot.batch.selection.is_empty());
    assert_eq!(slot.batch.text, TextAnchors::default());
}

#[test]
fn an_unreadable_rectangle_is_reported_never_swallowed() {
    let mut slot = Slot::default();
    record(&mut slot, INVALIDATE_TILES, "EMPTY");
    assert!(slot.batch.faults.is_empty() && slot.batch.all, "EMPTY means all");
    let mut slot = Slot::default();
    record(&mut slot, INVALIDATE_TILES, "1,2,x,4");
    assert!(slot.batch.all, "an unreadable invalidation still redraws");
    record(&mut slot, CELL_CURSOR, "1,2,3");
    assert!(slot.batch.cell.cursor.is_none());
    let mark = r#"{"marks":[{"rectangle":"1,2,y,4","color":"ff0000","part":"0"}]}"#;
    record(&mut slot, REFERENCE_MARKS, mark);
    assert_eq!(slot.batch.faults.len(), 3, "{:?}", slot.batch.faults);
}
