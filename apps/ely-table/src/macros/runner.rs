//! R runs against a copy, without holding the LOK worker.

use super::{
    contain::Run,
    protocol::{BYTES, Snapshot, refused, refusal},
};
use std::{
    fs,
    io::{self, Read},
    path::Path,
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(30);

pub struct Outcome {
    pub log: String,
    pub batch: io::Result<String>,
}

/// Waits for R, a stop, or 30 seconds.
pub fn run(rscript: &Path, snapshot: &Snapshot, script: &str, stop: &AtomicBool) -> Outcome {
    let mut log = String::new();
    let batch = execute(rscript, snapshot, script, TIMEOUT, stop, &mut log);
    Outcome { log, batch }
}

/// At most `BYTES`, however the file grows.
pub(super) fn read_limited(path: &Path) -> io::Result<String> {
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > BYTES {
        return Err(refused("table.r.too_big"));
    }
    let mut text = String::new();
    file.take(BYTES + 1).read_to_string(&mut text)?;
    if text.len() as u64 > BYTES {
        return Err(refused("table.r.too_big"));
    }
    Ok(text)
}

fn execute(
    executable: &Path,
    snapshot: &Snapshot,
    script: &str,
    timeout: Duration,
    stop: &AtomicBool,
    log: &mut String,
) -> io::Result<String> {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("input.tsv");
    let source = dir.path().join("macro.R");
    let api = dir.path().join("api.R");
    let output = dir.path().join("instructions.tsv");
    let stdout = dir.path().join("stdout.txt");
    let stderr = dir.path().join("stderr.txt");
    let data = snapshot.input()?;
    if data.len() as u64 > BYTES || script.len() as u64 > BYTES {
        return Err(refused("table.r.too_big"));
    }
    fs::write(&input, data)?;
    fs::write(&source, script)?;
    fs::write(&api, include_str!("api.R"))?;
    let mut command = Command::new(executable);
    command
        .arg("--vanilla")
        .arg(&api)
        .arg(&input)
        .arg(&source)
        .arg(&output)
        .current_dir(dir.path())
        .stdin(Stdio::null())
        .stdout(fs::File::create(&stdout)?)
        .stderr(fs::File::create(&stderr)?);
    // A stop before R starts runs nothing.
    if stop.load(Ordering::Relaxed) {
        return Err(refused("table.r.stopped"));
    }
    let mut run = Run::start(&mut command).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => refused("table.r.missing"),
        _ => error,
    })?;
    let started = Instant::now();
    let ended = loop {
        match run.exited() {
            Ok(true) => break Ok(()),
            Ok(false) => {}
            Err(error) => break Err(error),
        }
        let oversized = [&stdout, &stderr, &output]
            .iter()
            .any(|p| fs::metadata(p).is_ok_and(|m| m.len() > BYTES));
        if stop.load(Ordering::Relaxed) {
            break Err(refused("table.r.stopped"));
        }
        if oversized {
            break Err(refused("table.r.too_big"));
        }
        if started.elapsed() >= timeout {
            break Err(refused("table.r.timeout"));
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    // Helpers end on every path, an exit too.
    let status = run.finish()?;
    let elapsed = started.elapsed().as_millis();
    match &ended {
        Ok(()) => log::info!("table: Rscript ended {status} after {elapsed} ms"),
        Err(error) => log::info!("table: Rscript ended by {:?} after {elapsed} ms", refusal(error)),
    }
    let mut output_error = None;
    for path in [&stdout, &stderr] {
        match read_limited(path) {
            Ok(text) => log.push_str(&text),
            Err(error) => output_error = Some(error),
        }
    }
    ended?;
    // A stop while R exited still drops its batch.
    if stop.load(Ordering::Relaxed) {
        return Err(refused("table.r.stopped"));
    }
    if let Some(error) = output_error {
        return Err(error);
    }
    if !status.success() {
        return Err(io::Error::other(Exited(status)));
    }
    snapshot.instructions(&read_limited(&output)?)
}

/// R's own failure: its output says why.
#[derive(Debug)]
pub struct Exited(pub std::process::ExitStatus);

impl std::fmt::Display for Exited {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Exited {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::rscript;

    const EXAMPLE: &str = include_str!("example.R");

    fn snapshot() -> Snapshot {
        Snapshot::parse(r#"{"range":"S!A1:B1","rows":1,"columns":2,"cells":[{"kind":"text","value":"  á \"00123\"  "},{"kind":"number","value":"2","formula":"=1+1"}]}"#).expect("snapshot")
    }

    fn go(executable: &Path, script: &str, timeout: Duration, log: &mut String) -> io::Result<String> {
        execute(executable, &snapshot(), script, timeout, &AtomicBool::new(false), log)
    }

    #[test]
    fn an_absent_rscript_says_how_to_install_r() {
        let dir = tempfile::tempdir().expect("directory");
        let error = go(&dir.path().join("absent-Rscript"), "", TIMEOUT, &mut String::new())
            .expect_err("missing");
        assert_eq!(refusal(&error), Some("table.r.missing"));
    }

    #[test]
    fn a_stop_before_r_starts_runs_nothing() {
        let dir = tempfile::tempdir().expect("directory");
        let absent = dir.path().join("absent-Rscript");
        let stop = AtomicBool::new(true);
        let error = execute(&absent, &snapshot(), "", TIMEOUT, &stop, &mut String::new())
            .expect_err("stopped");
        assert_eq!(refusal(&error), Some("table.r.stopped"), "R was never started");
    }

    #[cfg(unix)]
    fn fixture(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/table")
            .join(name)
    }

    #[cfg(unix)]
    #[test]
    fn process_failures_invalid_batches_and_timeout_return_no_changes() {
        let mut log = String::new();
        let error = go(&fixture("rscript-fails"), "", TIMEOUT, &mut log).expect_err("script failed");
        let exited = error.get_ref().and_then(|e| e.downcast_ref::<Exited>()).expect("R's exit");
        assert_eq!(exited.0.code(), Some(7));
        assert!(log.contains("áé") && log.contains("falló"));
        let error = go(&fixture("rscript-invalid"), "", TIMEOUT, &mut log).expect_err("invalid");
        assert_eq!(refusal(&error), Some("table.r.invalid"));
        let error = go(&fixture("rscript-held"), "", Duration::from_millis(100), &mut log)
            .expect_err("timeout");
        assert_eq!(refusal(&error), Some("table.r.timeout"));
    }

    #[cfg(unix)]
    #[test]
    fn a_stop_ends_r_before_its_time() {
        let stop = AtomicBool::new(false);
        let started = Instant::now();
        let error = std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(200));
                stop.store(true, Ordering::Relaxed);
            });
            execute(&fixture("rscript-held"), &snapshot(), "", TIMEOUT, &stop, &mut String::new())
        })
        .expect_err("stopped");
        assert_eq!(refusal(&error), Some("table.r.stopped"));
        assert!(started.elapsed() < Duration::from_secs(3), "{:?}", started.elapsed());
    }

    #[test]
    fn real_r_keeps_mixed_types_and_writes_values() {
        let Ok(r) = rscript::locate() else {
            eprintln!("SKIP: Rscript is not installed");
            return;
        };
        let snapshot = Snapshot::parse(&serde_json::json!({
            "range":"Demo!A1:D2", "rows":2, "columns":4, "cells":[
                {"kind":"text","value":"  Café \"01\"  "}, {"kind":"text","value":"00123"},
                {"kind":"number","value":"-1.25"}, {"kind":"boolean","value":true},
                {"kind":"empty","value":null}, {"kind":"text","value":""},
                {"kind":"number","value":"7","formula":"=SUM(3,4)"}, {"kind":"boolean","value":false}
            ]
        }).to_string()).expect("mixed snapshot");
        let stop = AtomicBool::new(false);
        let text = run(&r, &snapshot, EXAMPLE, &stop).batch.expect("clean mixed cells");
        let batch: serde_json::Value = serde_json::from_str(&text).expect("batch");
        assert_eq!(batch["writes"].as_array().expect("writes").len(), 1);
        assert_eq!(batch["writes"][0]["value"], "Café \"01\"");
        assert_eq!(batch["snapshot"], snapshot.json);
        let script = r#"s <- ely$selection(); d <- ely$read_range(s)
            d[[3]][1] <- list(42); d[[4]][2] <- list(TRUE)
            d[[1]][2] <- list("vacía"); d[[2]][1] <- list(NULL)
            ely$write_range(s, d)"#;
        let text = run(&r, &snapshot, script, &stop).batch.expect("write mixed cells");
        let batch: serde_json::Value = serde_json::from_str(&text).expect("batch");
        let writes = &batch["writes"];
        assert!(writes[0]["value"].is_null());
        assert_eq!(writes[1]["value"], "42");
        assert_eq!(writes[2]["value"], "vacía");
        assert_eq!(writes[3]["value"], true);
    }

    #[test]
    fn real_r_reads_precise_numbers_and_returns_precise_writes() {
        let Ok(r) = rscript::locate() else {
            eprintln!("SKIP: Rscript is not installed");
            return;
        };
        let snapshot = Snapshot::parse(
            &serde_json::json!({"range":"S!A1:D1","rows":1,"columns":4,"cells":[
                {"kind":"number","value":"1.0000000000000002"},
                {"kind":"number","value":"0.12345678901234566"},
                {"kind":"number","value":"1E-007"},
                {"kind":"number","value":"1E+020"}
            ]})
            .to_string(),
        )
        .expect("snapshot");
        let script = r#"s <- ely$selection(); d <- ely$read_range(s)
            stopifnot(identical(d[[1]][[1]], 1.0000000000000002))
            stopifnot(identical(d[[2]][[1]], 0.12345678901234566))
            stopifnot(identical(d[[3]][[1]], 1e-7), identical(d[[4]][[1]], 1e20))
            d[[3]][1] <- list(d[[1]][[1]]); ely$write_range(s, d)"#;
        let batch = run(&r, &snapshot, script, &AtomicBool::new(false))
            .batch
            .expect("precise R execution");
        let batch: serde_json::Value = serde_json::from_str(&batch).expect("batch");
        assert_eq!(batch["writes"][0]["value"], "1.0000000000000002");
        assert_eq!(batch["snapshot"], snapshot.json);
    }

    #[test]
    fn real_r_success_errors_and_timeout() {
        let Ok(r) = rscript::locate() else {
            eprintln!("SKIP: Rscript is not installed");
            return;
        };
        let mut log = String::new();
        let batch = go(&r, EXAMPLE, TIMEOUT, &mut log).expect("R macro");
        let batch: serde_json::Value = serde_json::from_str(&batch).expect("JSON batch");
        assert_eq!(batch["writes"][0]["value"], "á \"00123\"");
        assert!(log.contains("formulas kept"));
        let failing = "ely$autofit_columns(ely$selection()); stop('falló')";
        assert!(go(&r, failing, TIMEOUT, &mut log).is_err());
        assert!(log.contains("falló"));
        let error = go(&r, "Sys.sleep(5)", Duration::from_millis(100), &mut log).expect_err("timeout");
        assert_eq!(refusal(&error), Some("table.r.timeout"));
    }
}
