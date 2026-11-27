//! R23.8a: a read-only view refuses typing until released.

use super::input::{open, paragraphs, saved};
use super::*;

/// The kernel's Delete key code.
const DELETE: i32 = 1286;

#[test]
fn a_read_only_view_takes_no_keys_until_released() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    doc.set_read_only(&mut worker, true).expect("read-only on");
    worker.post_key(&doc, 'a' as u32, 512).expect("a posts");
    worker.post_key(&doc, 0, DELETE).expect("delete posts");
    let locked = paragraphs(&saved(&dir, &mut worker, &doc, "locked.fodt"));
    assert_eq!(locked, ["Hello 世界"], "typing and deleting are refused");
    doc.set_read_only(&mut worker, false).expect("read-only off");
    worker.post_key(&doc, 'b' as u32, 512).expect("b posts");
    let open_again = paragraphs(&saved(&dir, &mut worker, &doc, "open.fodt"));
    assert_eq!(open_again, ["bHello 世界"], "typing lands again");
}
