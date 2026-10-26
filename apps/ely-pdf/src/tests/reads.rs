//! Reads that fail are kept, not asked each frame (P-08).

use ely_pdf_engine::sample::{Extras, pdf_with};
use gpui::TestAppContext;

use super::{settle, shown, windowed};

/// Two pages; the first's square has an unreadable rectangle.
fn bad_then_good() -> Vec<u8> {
    let nines = "9".repeat(50);
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [5 0 R] >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [6 0 R] >>".to_string(),
        format!("<< /Type /Annot /Subtype /Square /Rect [0 0 {nines} 100] >>"),
        "<< /Type /Annot /Subtype /Text /Rect [100 500 120 520] /Contents (fine) >>".to_string(),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend(format!("{} 0 obj\n{body}\nendobj\n", i + 1).bytes());
    }
    let xref = pdf.len();
    pdf.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
    for offset in offsets {
        pdf.extend(format!("{offset:010} 00000 n \n").bytes());
    }
    pdf.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).bytes());
    pdf
}

#[test]
fn one_bad_page_spares_the_rest_of_its_batch() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = dir.path().join("two.pdf");
    std::fs::write(&path, bad_then_good()).expect("writes");
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    settle(cx, |app| !view.read(app).reading_notes);
    cx.update(|_, app| view.update(app, |view, cx| {
        view.notes.clear();
        view.unread.clear();
        view.ask_notes([0, 1], cx)
    }));
    settle(cx, |app| !view.read(app).reading_notes);
    let (good, bad) = cx.update(|_, app| {
        let view = view.read(app);
        (view.notes.contains_key(&1), view.unread.contains(&crate::view::Unread::Notes(0)))
    });
    assert!(bad, "the bad page is kept as failed");
    assert!(good, "the good page is read all the same");
    assert!(!cx.update(|_, app| view.read(app).unread.contains(&crate::view::Unread::Notes(1))));
}

#[test]
fn a_signature_read_from_before_a_save_counts_for_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = dir.path().join("one.pdf");
    let page: (f32, f32, &[&str]) = (612.0, 792.0, &["Hello"]);
    std::fs::write(&path, pdf_with(&[page], &[], &Extras::default())).expect("writes");
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    let gone = dir.path().join("gone.pdf");
    // The read fails, but a save lands before its answer.
    cx.update(|_, app| view.update(app, |view, cx| {
        view.ask_signatures(gone.clone(), cx);
        let same = view.saved.clone();
        view.saved(&same, cx);
    }));
    settle(cx, |app| !view.read(app).reading_signatures);
    assert!(cx.update(|_, app| view.read(app).unread.is_empty()), "a stale failure blocks nothing");
    cx.update(|_, app| view.update(app, |view, cx| view.ask_signatures(gone.clone(), cx)));
    assert!(cx.update(|_, app| view.read(app).reading_signatures), "asked again after the save");
}

#[test]
fn a_failed_read_is_not_asked_again() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let path = dir.path().join("one.pdf");
    let page: (f32, f32, &[&str]) = (612.0, 792.0, &["Hello"]);
    std::fs::write(&path, pdf_with(&[page], &[], &Extras::default())).expect("writes");
    let (viewer, cx) = windowed(&mut cx, dir.path(), &[path], true);
    let view = shown(cx, &viewer);
    assert!(cx.update(|_, app| view.read(app).form_names.is_none()), "not read yet");
    // A dead worker fails every read.
    cx.update(|_, app| viewer.read(app).core.worker.lock().expect("the lock").get().expect("a worker").kill()).expect("kills");
    cx.update(|_, app| view.update(app, |view, cx| view.ask_names(cx)));
    settle(cx, |app| !view.read(app).reading_names);
    assert!(cx.update(|_, app| view.read(app).unread.contains(&crate::view::Unread::Names)), "the failure is kept");
    cx.update(|_, app| view.update(app, |view, cx| view.ask_names(cx)));
    assert!(!cx.update(|_, app| view.read(app).reading_names), "not asked again");
}
