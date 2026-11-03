//! Mail merge work: the CSV, filled copies, previews and files.

use std::{
    collections::hash_map::RandomState,
    hash::BuildHasher,
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use ely_gpui_component::primitives::Severity;
use ely_lok::{Doc, Worker};
use ely_ipc::Worded;
use gpui::{AppContext as _, Context, SharedString, TaskExt as _};

use ely_office::files::{self, file_name};
use ely_office::prelude::*;
use ely_office::{Source, in_window};

use crate::docs::Docs;

/// A CSV: its name, column names and rows.
#[derive(Clone, Debug)]
pub struct Table {
    pub name: SharedString,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Reads a UTF-8 CSV whose first row names the columns.
pub fn read_csv(path: &Path) -> io::Result<Table> {
    let mut reader = csv::Reader::from_path(path).map_err(io::Error::other)?;
    let headers: Vec<String> = reader
        .headers()
        .map_err(io::Error::other)?
        .iter()
        .map(|header| header.trim().to_string())
        .collect();
    if let Some(at) = headers.iter().position(String::is_empty) {
        return Err(Worded::error(io::ErrorKind::InvalidData, "doc.merge.unnamed", &[("n", &(at + 1).to_string())]));
    }
    // A name with a guillemet would blur the marker bounds.
    if let Some(header) = headers.iter().find(|header| header.contains(['«', '»'])) {
        return Err(Worded::error(io::ErrorKind::InvalidData, "doc.merge.guillemet", &[("name", header)]));
    }
    for (at, header) in headers.iter().enumerate() {
        if headers[..at].contains(header) {
            return Err(Worded::error(io::ErrorKind::InvalidData, "doc.merge.twice", &[("name", header)]));
        }
    }
    let rows = reader
        .records()
        .map(|record| record.map(|record| record.iter().map(str::to_string).collect()))
        .collect::<Result<Vec<Vec<String>>, _>>()
        .map_err(io::Error::other)?;
    if rows.is_empty() {
        return Err(Worded::error(io::ErrorKind::InvalidData, "doc.merge.no_rows", &[]));
    }
    Ok(Table {
        name: file_name(path),
        headers,
        rows,
    })
}

/// The marker a field leaves in the text.
pub fn marker(header: &str) -> String {
    format!("«{header}»")
}

/// Replaces every field marker in `doc` with `row`'s values.
fn fill(worker: &mut Worker, doc: &Doc, headers: &[String], row: &[String]) -> io::Result<()> {
    // Markers become tokens first, so values are never rescanned.
    let nonce = BuildHasher::hash_one(&RandomState::new(), 0);
    let token = |at: usize| format!("\u{e000}{nonce:x}-{at}\u{e001}");
    let mut replace = |from: &str, to: &str| {
        worker.run(
            doc,
            "find.replace",
            &[from, "false", "3", "0", "0", "1", to],
        )
    };
    for (at, header) in headers.iter().enumerate() {
        replace(&marker(header), &token(at))?;
    }
    for (at, value) in row.iter().enumerate() {
        replace(&token(at), value)?;
    }
    Ok(())
}

/// Copies `from` to `to`, never over an existing file.
fn publish(from: &Path, to: &Path) -> io::Result<()> {
    let mut target = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to)?;
    io::copy(&mut std::fs::File::open(from)?, &mut target)?;
    target.sync_all()
}

impl Docs {
    /// Saves tab `source` as the template copy, in the background.
    fn template(
        &mut self,
        source: u64,
        into: u64,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, SharedString, PathBuf, &mut Context<Self>) + 'static,
    ) {
        let Some(name) = self.tab(source).map(|tab| tab.name.clone()) else {
            return self.merge_failed(&Worded::error(io::ErrorKind::NotFound, "error.closed", &[]), cx);
        };
        let copy = match files::work_dir(&self.core.work, into) {
            Ok(dir) => dir.join("merge.odt"),
            Err(error) => return self.merge_failed(&error, cx),
        };
        let url = ely_lok::file_url(&copy);
        self.read_in(
            source,
            move |worker, doc| doc.save_as(worker, &url, "odt"),
            move |docs, saved, _, cx| match saved {
                Ok(()) => then(docs, name, copy, cx),
                Err(error) => docs.merge_failed(&error, cx),
            },
            cx,
        );
    }

    /// Logs `error` itself; only the toast reads its words.
    fn merge_failed(&self, error: &io::Error, cx: &mut Context<Self>) {
        log::warn!("doc: the merge failed: {}", Worded::logged(error));
        let text = Docs::text(cx, "office.error.engine", &[("error", &ely_ui::i18n::error(cx, error))]);
        self.toast_unlogged(text, Severity::Danger, cx);
    }

    /// Opens record `at` filled in, as a new tab.
    pub(crate) fn preview_record(
        &mut self,
        source: u64,
        table: &Table,
        at: usize,
        cx: &mut Context<Self>,
    ) {
        if !self.settled(source, cx) {
            let table = table.clone();
            return self.when_settled(source, move |docs, _, cx| {
                docs.preview_record(source, &table, at, cx)
            });
        }
        let (headers, row) = (table.headers.clone(), table.rows[at].clone());
        // The preview's tab and folder, reserved now.
        let into = self.core.next;
        self.core.next += 1;
        self.template(source, into, cx, move |docs, name, copy, cx| {
            docs.preview_copy((into, at), name, copy, (headers, row), cx)
        });
    }

    /// Opens the template copy and fills it with one row.
    fn preview_copy(
        &mut self,
        (into, at): (u64, usize),
        name: SharedString,
        copy: PathBuf,
        (headers, row): (Vec<String>, Vec<String>),
        cx: &mut Context<Self>,
    ) {
        log::info!("doc: previewing merge record {}", at + 1);
        self.merge.update(cx, |merge, _| merge.previews.insert(into));
        self.load_into(into, Source::Copy(copy), cx);
        self.core.waiting.push((
            into,
            Box::new(move |docs, _, cx| {
                let Some(tab) = docs.tab_mut(into) else {
                    return;
                };
                let n = (at + 1).to_string();
                tab.name = Docs::text(cx, "doc.merge.preview_name", &[("name", &name), ("n", &n)]);
                cx.notify();
                docs.read_in(
                    into,
                    move |worker, doc| fill(worker, doc, &headers, &row),
                    |docs, filled, _, cx| {
                        if let Err(error) = filled {
                            docs.merge_failed(&error, cx);
                        }
                    },
                    cx,
                );
            }),
        ));
    }

    /// Writes one file per row into `folder`; refuses to overwrite.
    pub(crate) fn generate(
        &mut self,
        source: u64,
        table: Table,
        pdf: bool,
        folder: PathBuf,
        cx: &mut Context<Self>,
    ) {
        if !self.settled(source, cx) {
            return self.when_settled(source, move |docs, _, cx| {
                docs.generate(source, table, pdf, folder, cx)
            });
        }
        let Some(name) = self.tab(source).map(|tab| tab.name.clone()) else {
            return self.merge_failed(&Worded::error(io::ErrorKind::NotFound, "error.closed", &[]), cx);
        };
        let stem = Path::new(name.as_ref()).file_stem().map_or_else(
            || name.to_string(),
            |stem| stem.to_string_lossy().into_owned(),
        );
        let extension = if pdf { "pdf" } else { "docx" };
        let targets: Vec<PathBuf> = (1..=table.rows.len())
            .map(|n| folder.join(format!("{stem} {n}.{extension}")))
            .collect();
        let taken = targets
            .iter()
            .find(|target| target.exists())
            .map(|target| file_name(target));
        if let Some(taken) = taken {
            let text = Docs::text(cx, "doc.merge.exists", &[("name", &taken)]);
            return self.toast(text, Severity::Danger, cx);
        }
        // Each run keeps its own template and staging folder.
        let into = self.core.next;
        self.core.next += 1;
        self.template(source, into, cx, move |docs, _, template, cx| {
            docs.write_records(template, (table, targets, extension, folder), cx)
        });
    }

    /// Writes each row's file from the template copy.
    fn write_records(
        &mut self,
        template: PathBuf,
        (table, targets, extension, folder): (Table, Vec<PathBuf>, &'static str, PathBuf),
        cx: &mut Context<Self>,
    ) {
        log::info!("doc: merging {} records", targets.len());
        let worker = Arc::clone(&self.core.worker);
        cx.spawn(async move |this, cx| {
            let written = cx
                .background_spawn(async move {
                    for (n, (row, target)) in table.rows.iter().zip(&targets).enumerate() {
                        // One record at a time: open documents keep their turns.
                        let mut worker = worker.lock().expect("the worker lock");
                        let staged = template.with_file_name(format!("record.{extension}"));
                        let made = worker.load(&ely_lok::file_url(&template)).and_then(|doc| {
                            let saved = fill(&mut worker, &doc, &table.headers, row)
                                .and_then(|()| doc.save_as(&mut worker, &ely_lok::file_url(&staged), extension));
                            // Closed even when this record failed.
                            let closed = worker.close_doc(doc);
                            saved.and(closed)?;
                            publish(&staged, target)
                        });
                        made.map_err(|error| (n + 1, error))?;
                    }
                    Ok(targets.len())
                })
                .await;
            in_window(&this, cx, |docs, _, cx| {
                let folder = folder.display().to_string();
                let text = match written {
                    Ok(count) => {
                        let n = count.to_string();
                        let text = ely_ui::i18n::counted(cx, "doc.merge.done", count as u64, &[("n", &n), ("folder", &folder)]);
                        return docs.toast(text, Severity::Success, cx);
                    }
                    Err((n, error)) => Docs::text(
                        cx,
                        "doc.merge.failed",
                        &[("n", &n.to_string()), ("error", &error.to_string())],
                    ),
                };
                docs.toast(text, Severity::Danger, cx);
            })
        })
        .detach_and_log_err(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_csv_reads_or_says_why_not() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let path = dir.path().join("people.csv");
        std::fs::write(&path, "\u{feff}Name,City\nAda,\"London, UK\"\nLin,上海\n").expect("writes");
        let table = read_csv(&path).expect("reads");
        assert_eq!(table.headers, ["Name", "City"]);
        assert_eq!(table.rows[0], ["Ada", "London, UK"]);
        assert_eq!(table.rows[1][1], "上海");
        for (body, why) in [
            ("Name,\nAda,x\n", "doc.merge.unnamed"),
            ("Name,Name\na,b\n", "doc.merge.twice"),
            ("Name,«Name»\na,b\n", "doc.merge.guillemet"),
            ("Name,City\n", "doc.merge.no_rows"),
            ("Name,City\nAda\n", "found record"),
        ] {
            std::fs::write(&path, body).expect("writes");
            let error = read_csv(&path).expect_err(body).to_string();
            assert!(error.contains(why), "{body}: {error}");
        }
        std::fs::write(&path, b"Name\n\xff\xfe\n").expect("writes");
        assert!(read_csv(&path).is_err(), "bytes that are not UTF-8 fail");
    }
}
