//! Bookmarks, links, properties, attachments against the real worker (PDF-07).

use std::path::Path;

use ely_pdf_engine::{
    BookmarkChange, Doc, LinkChange, Target, Worker,
    sample::{Extras, pdf_with},
};

fn worker() -> Worker {
    Worker::spawn_from(Path::new(env!("CARGO_BIN_EXE_ely-pdf-worker"))).expect("the worker")
}

/// Two pages, bookmarks One (with Sub) and Two, a link.
fn opened(path: &Path) -> (Worker, Doc) {
    let pages: [(f32, f32, &[&str]); 2] = [(612.0, 792.0, &["Page one"]), (612.0, 792.0, &["Page two"])];
    let marks = [("One", 0, &[("Sub", 1)][..]), ("Two", 1, &[][..])];
    std::fs::write(path, pdf_with(&pages, &marks, &Extras { links: &[(0, 1)], ..Extras::default() })).expect("writes");
    let mut worker = worker();
    let doc = worker.open(path).expect("opens").doc;
    (worker, doc)
}

/// The outline as indented titles with their pages.
fn tree(worker: &mut Worker, doc: Doc) -> Vec<String> {
    fn walk(marks: &[ely_pdf_engine::Bookmark], depth: usize, out: &mut Vec<String>) {
        for mark in marks {
            out.push(format!("{}{} {}", "-".repeat(depth), mark.title, mark.page.map_or(-1, |p| p as i64)));
            walk(&mark.down, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    walk(&worker.outline(doc).expect("outline"), 0, &mut out);
    out
}

#[test]
fn bookmarks_add_rename_retarget_and_go() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    assert_eq!(tree(&mut worker, doc), ["One 0", "-Sub 1", "Two 1"]);
    worker.change_bookmark(doc, &BookmarkChange::Add { path: vec![2], title: "Three".into(), page: 1, y: 100.0 }).expect("adds");
    doc = worker.now(doc).expect("the state");
    worker.change_bookmark(doc, &BookmarkChange::Add { path: vec![0, 1], title: "Under".into(), page: 0, y: 0.0 }).expect("adds below");
    doc = worker.now(doc).expect("the state");
    let rename = BookmarkChange::Edit { path: vec![1], expect: "Two".into(), title: Some("Deux".into()), to: Some((0, 50.0)) };
    worker.change_bookmark(doc, &rename).expect("edits");
    doc = worker.now(doc).expect("the state");
    assert_eq!(tree(&mut worker, doc), ["One 0", "-Sub 1", "-Under 0", "Deux 0", "Three 1"]);
    assert!(worker.change_bookmark(doc, &BookmarkChange::Delete { path: vec![1], expect: "Two".into() }).is_err(), "renamed since");
    assert!(worker.change_bookmark(doc, &BookmarkChange::Delete { path: vec![9], expect: "One".into() }).is_err(), "no such place");
    worker.change_bookmark(doc, &BookmarkChange::Delete { path: vec![0], expect: "One".into() }).expect("deletes with its children");
    doc = worker.now(doc).expect("the state");
    assert_eq!(tree(&mut worker, doc), ["Deux 0", "Three 1"]);
    worker.undo(doc, false).expect("undoes");
    doc = worker.now(doc).expect("the state");
    assert_eq!(tree(&mut worker, doc).len(), 5, "one step");
    let saved = dir.path().join("saved.pdf");
    worker.save(doc, &saved).expect("saves");
    let mut other = self::worker();
    let reopened = other.open(&saved).expect("opens").doc;
    assert_eq!(tree(&mut other, reopened), ["One 0", "-Sub 1", "-Under 0", "Deux 0", "Three 1"]);
}

#[test]
fn links_list_add_retarget_and_go() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    let links = worker.links(doc, 0).expect("links");
    assert_eq!((links.len(), links[0].page), (1, Some(1)), "the sample links to page two");
    let web = Target::Web("https://example.com/".into());
    worker.change_link(doc, 0, &LinkChange::Add { rect: [72.0, 300.0, 200.0, 320.0], to: web.clone() }).expect("adds");
    doc = worker.now(doc).expect("the state");
    let links = worker.links(doc, 0).expect("links");
    assert_eq!((links[1].uri.as_str(), links[1].page), ("https://example.com/", None));
    let first = links[0].uri.clone();
    worker.change_link(doc, 0, &LinkChange::Edit { index: 0, expect: first.clone(), to: Target::Page(0, 200.0) }).expect("retargets");
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.links(doc, 0).expect("links")[0].page, Some(0));
    assert!(worker.change_link(doc, 0, &LinkChange::Delete { index: 0, expect: first }).is_err(), "changed since");
    for unsafe_uri in ["javascript:alert(1)", "file:///etc/passwd"] {
        let add = LinkChange::Add { rect: [72.0, 400.0, 200.0, 420.0], to: Target::Web(unsafe_uri.into()) };
        assert!(worker.change_link(doc, 0, &add).is_err(), "{unsafe_uri}");
    }
    let now = worker.links(doc, 0).expect("links")[1].uri.clone();
    worker.change_link(doc, 0, &LinkChange::Delete { index: 1, expect: now }).expect("deletes");
    doc = worker.now(doc).expect("the state");
    assert_eq!(worker.links(doc, 0).expect("links").len(), 1);
}

#[test]
fn properties_read_set_and_keep() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    assert_eq!(worker.properties(doc).expect("reads").title, None, "the sample has none");
    worker.set_properties(doc, &[("Title", "Report"), ("Author", "Zoe")]).expect("sets");
    doc = worker.now(doc).expect("the state");
    let read = worker.properties(doc).expect("reads");
    assert_eq!((read.title.as_deref(), read.author.as_deref()), (Some("Report"), Some("Zoe")));
    assert!(worker.set_properties(doc, &[("Title", "x"), ("Producer", "Ely")]).is_err(), "the producer is the producer's");
    assert_eq!(worker.properties(doc).expect("reads").title.as_deref(), Some("Report"), "nothing changed");
    let saved = dir.path().join("saved.pdf");
    worker.save(doc, &saved).expect("saves");
    let mut other = self::worker();
    let reopened = other.open(&saved).expect("opens").doc;
    assert_eq!(other.properties(reopened).expect("reads").author.as_deref(), Some("Zoe"));
}

#[test]
fn attachments_embed_read_and_go() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    assert!(worker.attachments(doc).expect("lists").is_empty());
    let file = dir.path().join("notes.txt");
    std::fs::write(&file, "seven words of notes for the file").expect("writes");
    worker.attach(doc, "notes.txt", &file).expect("attaches");
    doc = worker.now(doc).expect("the state");
    worker.attach(doc, "b.txt", &file).expect("attaches another");
    doc = worker.now(doc).expect("the state");
    let list = worker.attachments(doc).expect("lists");
    let names: Vec<(&str, u64)> = list.iter().map(|a| (a.name.as_str(), a.size)).collect();
    assert_eq!(names, [("b.txt", 33), ("notes.txt", 33)], "by name");
    assert_eq!(worker.attachment(doc, "n:notes.txt").expect("reads"), b"seven words of notes for the file");
    assert!(worker.attach(doc, "notes.txt", &file).is_err(), "the name is taken");
    worker.detach(doc, "n:b.txt").expect("removes");
    doc = worker.now(doc).expect("the state");
    assert!(worker.attachment(doc, "n:b.txt").is_err());
    let saved = dir.path().join("saved.pdf");
    worker.save(doc, &saved).expect("saves");
    let mut other = self::worker();
    let reopened = other.open(&saved).expect("opens").doc;
    let kept: Vec<String> = other.attachments(reopened).expect("lists").into_iter().map(|a| a.name).collect();
    assert_eq!(kept, ["notes.txt"]);
}

#[test]
fn a_file_on_a_page_lists_reads_and_goes_by_its_key() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("doc.pdf");
    let extras = Extras { attached: Some(("note.txt", b"notes")), ..Extras::default() };
    std::fs::write(&path, pdf_with(&[(612.0, 792.0, &["Hello"])], &[], &extras)).expect("writes");
    let mut worker = worker();
    let mut doc = worker.open(&path).expect("opens").doc;
    let list = worker.attachments(doc).expect("lists");
    assert_eq!(list.len(), 1, "the page's file is an attachment");
    assert_eq!((list[0].name.as_str(), list[0].page, list[0].size), ("note.txt", Some(0), 5), "its size from its bytes");
    assert!(list[0].key.starts_with("a:0:"));
    assert_eq!(worker.attachment(doc, &list[0].key).expect("reads"), b"notes");
    worker.detach(doc, &list[0].key).expect("removes");
    doc = worker.now(doc).expect("the state");
    assert!(worker.attachments(doc).expect("lists").is_empty());
    assert!(worker.marks(doc, 0).expect("marks").annots.is_empty(), "with its annotation");
}

#[test]
fn reading_an_attachment_leaves_its_bytes_in_the_document() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, mut doc) = opened(&dir.path().join("doc.pdf"));
    let file = dir.path().join("note.txt");
    std::fs::write(&file, "notes").expect("writes");
    worker.attach(doc, "note.txt", &file).expect("attaches");
    doc = worker.now(doc).expect("the state");
    let mark = worker.history(doc).expect("a mark");
    for _ in 0..2 {
        assert_eq!(worker.attachment(doc, "n:note.txt").expect("reads"), b"notes");
    }
    assert_eq!(worker.history(doc).expect("a mark"), mark, "reading changes nothing");
    let saved = dir.path().join("saved.pdf");
    worker.save(doc, &saved).expect("saves");
    let mut other = self::worker();
    let reopened = other.open(&saved).expect("opens").doc;
    assert_eq!(other.attachment(reopened, "n:note.txt").expect("reads"), b"notes", "whole in the file");
}

#[test]
fn an_attachment_read_before_a_change_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (mut worker, doc) = opened(&dir.path().join("doc.pdf"));
    let (old, new) = (dir.path().join("old.txt"), dir.path().join("new.txt"));
    std::fs::write(&old, "old report").expect("writes");
    std::fs::write(&new, "new report").expect("writes");
    let first = doc.at(&worker.attach(doc, "report.txt", &old).expect("attaches")).expect("the state");
    let gone = first.at(&worker.detach(first, "n:report.txt").expect("removes")).expect("the state");
    let now = gone.at(&worker.attach(gone, "report.txt", &new).expect("attaches again")).expect("the state");
    // The same key names another file now: refused.
    let read = worker.attachment(first, "n:report.txt").expect_err("an older state");
    assert!(ely_pdf_engine::is_stale(&read), "{read}");
    let removed = worker.detach(first, "n:report.txt").expect_err("an older state");
    assert!(ely_pdf_engine::is_stale(&removed), "{removed}");
    assert_eq!(worker.attachment(now, "n:report.txt").expect("reads"), b"new report", "the new file stays");
}
