//! R runs against a copy, without holding the LOK worker.

use super::protocol::{BYTES, Snapshot};
use std::{
    fs,
    io::{self, Read},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub const MISSING: &str = "Rscript was not found in PATH. Install R from https://cran.r-project.org (Linux: your distribution's r-base/R package), add R's bin directory to PATH if needed, restart ElyTable, then run the macro again. R is never installed automatically.";
const TIMEOUT: Duration = Duration::from_secs(30);

pub struct Outcome {
    pub log: String,
    pub batch: io::Result<String>,
}

pub fn available() -> bool {
    let name = if cfg!(windows) {
        "Rscript.exe"
    } else {
        "Rscript"
    };
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
}

pub fn run(snapshot: Snapshot, script: String) -> Outcome {
    let mut log = String::new();
    let batch = execute(Path::new("Rscript"), &snapshot, &script, TIMEOUT, &mut log);
    Outcome { log, batch }
}

fn read_limited(path: &Path) -> io::Result<String> {
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > BYTES {
        return Err(io::Error::other("R output exceeds 4 MiB"));
    }
    let mut text = String::new();
    file.take(BYTES + 1).read_to_string(&mut text)?;
    if text.len() as u64 > BYTES {
        return Err(io::Error::other("R output exceeds 4 MiB"));
    }
    Ok(text)
}

fn execute(
    executable: &Path,
    snapshot: &Snapshot,
    script: &str,
    timeout: Duration,
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
        return Err(io::Error::other("R input exceeds 4 MiB"));
    }
    fs::write(&input, data)?;
    fs::write(&source, script)?;
    fs::write(&api, include_str!("api.R"))?;
    let mut child = Command::new(executable)
        .arg("--vanilla")
        .arg(&api)
        .arg(&input)
        .arg(&source)
        .arg(&output)
        .current_dir(dir.path())
        .stdin(Stdio::null())
        .stdout(fs::File::create(&stdout)?)
        .stderr(fs::File::create(&stderr)?)
        .spawn()
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                io::Error::new(e.kind(), MISSING)
            } else {
                e
            }
        })?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {}
            Err(error) => break Err(error),
        }
        let oversized = [&stdout, &stderr, &output]
            .iter()
            .any(|p| fs::metadata(p).is_ok_and(|m| m.len() > BYTES));
        if started.elapsed() >= timeout || oversized {
            break Err(io::Error::other(if oversized {
                "R output exceeds 4 MiB"
            } else {
                "R macro timed out (30 seconds)"
            }));
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    if status.is_err() {
        let killed = child.kill();
        let waited = child.wait();
        killed.and(waited.map(|_| ()))?;
    }
    let mut output_error = None;
    for path in [&stdout, &stderr] {
        match read_limited(path) {
            Ok(text) => log.push_str(&text),
            Err(error) => {
                log.push_str(&format!("\n{error}\n"));
                output_error = Some(error);
            }
        }
    }
    let status = status?;
    if let Some(error) = output_error {
        return Err(error);
    }
    if !status.success() {
        return Err(io::Error::other(format!("Rscript exited with {status}")));
    }
    snapshot.instructions(&read_limited(&output)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        Snapshot::parse(r#"{"range":"S!A1:B1","rows":1,"columns":2,"cells":[{"kind":"text","value":"  á \"00123\"  "},{"kind":"number","value":"2","formula":"=1+1"}]}"#).expect("snapshot")
    }

    #[test]
    fn absent_r_has_installation_instructions() {
        let mut log = String::new();
        let dir = tempfile::tempdir().expect("directory");
        let error = execute(
            &dir.path().join("absent-Rscript"),
            &snapshot(),
            "",
            TIMEOUT,
            &mut log,
        )
        .expect_err("missing");
        assert!(error.to_string().contains("cran.r-project.org"));
    }

    #[cfg(unix)]
    #[test]
    fn process_failures_invalid_batches_and_timeout_return_no_changes() {
        let fixtures = Path::new(file!())
            .parent()
            .expect("module directory")
            .join("../../../../tests/fixtures/table")
            .canonicalize()
            .expect("fixture directory");
        let mut log = String::new();
        let error = execute(
            &fixtures.join("rscript-fails"),
            &snapshot(),
            "",
            TIMEOUT,
            &mut log,
        )
        .expect_err("script failed");
        assert!(error.to_string().contains("7"));
        assert!(log.contains("áé") && log.contains("falló"));
        assert!(
            execute(
                &fixtures.join("rscript-invalid"),
                &snapshot(),
                "",
                TIMEOUT,
                &mut log
            )
            .is_err()
        );
        let error = execute(
            &fixtures.join("rscript-held"),
            &snapshot(),
            "",
            Duration::from_millis(100),
            &mut log,
        )
        .expect_err("timeout");
        assert!(error.to_string().contains("timed out"));
    }

    #[test]
    fn real_r_keeps_mixed_types_and_writes_values() {
        if !available() {
            eprintln!("SKIP: Rscript is not installed");
            return;
        }
        let snapshot = Snapshot::parse(&serde_json::json!({
            "range":"Demo!A1:D2", "rows":2, "columns":4, "cells":[
                {"kind":"text","value":"  Café \"01\"  "}, {"kind":"text","value":"00123"},
                {"kind":"number","value":"-1.25"}, {"kind":"boolean","value":true},
                {"kind":"empty","value":null}, {"kind":"text","value":""},
                {"kind":"number","value":"7","formula":"=SUM(3,4)"}, {"kind":"boolean","value":false}
            ]
        }).to_string()).expect("mixed snapshot");
        let mut log = String::new();
        let text = execute(
            Path::new("Rscript"),
            &snapshot,
            include_str!("../../../../examples/macros/limpiar-textos.R"),
            TIMEOUT,
            &mut log,
        )
        .expect("clean mixed cells");
        let batch: serde_json::Value = serde_json::from_str(&text).expect("batch");
        assert_eq!(batch["writes"].as_array().expect("writes").len(), 1);
        assert_eq!(batch["writes"][0]["value"], "Café \"01\"");
        assert_eq!(batch["snapshot"], snapshot.json);
        let script = r#"s <- ely$selection(); d <- ely$read_range(s)
            d[[3]][1] <- list(42); d[[4]][2] <- list(TRUE)
            d[[1]][2] <- list("vacía"); d[[2]][1] <- list(NULL)
            ely$write_range(s, d)"#;
        let text = execute(Path::new("Rscript"), &snapshot, script, TIMEOUT, &mut log)
            .expect("write mixed cells");
        let batch: serde_json::Value = serde_json::from_str(&text).expect("batch");
        let writes = &batch["writes"];
        assert!(writes[0]["value"].is_null());
        assert_eq!(writes[1]["value"], "42");
        assert_eq!(writes[2]["value"], "vacía");
        assert_eq!(writes[3]["value"], true);
    }

    #[test]
    fn real_r_reads_precise_numbers_and_returns_precise_writes() {
        if !available() {
            eprintln!("SKIP: Rscript is not installed");
            return;
        }
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
        let mut log = String::new();
        let batch = execute(Path::new("Rscript"), &snapshot, script, TIMEOUT, &mut log)
            .expect("precise R execution");
        let batch: serde_json::Value = serde_json::from_str(&batch).expect("batch");
        assert_eq!(batch["writes"][0]["value"], "1.0000000000000002");
        assert_eq!(batch["snapshot"], snapshot.json);
    }

    #[test]
    fn real_r_success_errors_and_timeout() {
        if Command::new("Rscript").arg("--version").output().is_err() {
            eprintln!("SKIP: Rscript is not installed");
            return;
        }
        let r = Path::new("Rscript");
        let mut log = String::new();
        let batch = execute(
            r,
            &snapshot(),
            include_str!("../../../../examples/macros/limpiar-textos.R"),
            TIMEOUT,
            &mut log,
        )
        .expect("R macro");
        let batch: serde_json::Value = serde_json::from_str(&batch).expect("JSON batch");
        assert_eq!(batch["writes"][0]["value"], "á \"00123\"");
        assert!(log.contains("fórmulas"));
        assert!(
            execute(
                r,
                &snapshot(),
                "ely$autofit_columns(ely$selection()); stop('falló')",
                TIMEOUT,
                &mut log
            )
            .is_err()
        );
        assert!(log.contains("falló"));
        assert!(
            execute(
                r,
                &snapshot(),
                "Sys.sleep(5)",
                Duration::from_millis(100),
                &mut log
            )
            .is_err()
        );
    }
}
