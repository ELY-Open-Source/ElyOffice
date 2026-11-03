//! Typed snapshots and validated batch instructions.

use serde_json::{Value, json};
use std::io;

pub const LIMIT: usize = 10_000;
pub const BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub json: Value,
    pub rows: usize,
    pub columns: usize,
}

/// A failure ElyTable words itself; holds its message key.
#[derive(Debug)]
pub struct Refusal(pub &'static str);

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for Refusal {}

pub fn refused(key: &'static str) -> io::Error {
    io::Error::other(Refusal(key))
}

/// The key `error` carries, if ElyTable words it.
pub fn refusal(error: &io::Error) -> Option<&'static str> {
    error.get_ref()?.downcast_ref::<Refusal>().map(|refusal| refusal.0)
}

/// The kernel's reason the selection cannot be read.
#[derive(Debug)]
pub struct Unreadable(pub String);

impl std::fmt::Display for Unreadable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Unreadable {}

/// R's output cannot be applied.
fn bad() -> io::Error {
    refused("table.r.invalid")
}

/// The kernel's copy of the selection cannot be read.
fn unread() -> io::Error {
    refused("table.r.unreadable")
}

/// The message key for the kernel's refusal code (0081).
fn kernel_key(code: &str) -> Option<&'static str> {
    Some(match code {
        "range" => "table.r.range",
        "cells" => "table.r.cells",
        "too_big" => "table.r.too_big",
        "unreadable" => "table.r.unreadable",
        "batch_too_big" => "table.r.batch_too_big",
        "locked" => "table.r.locked",
        "rollback" => "table.r.rollback",
        "undo_off" => "table.r.undo_off",
        "busy" => "table.r.busy",
        "changed" => "table.r.stale",
        "protected" => "table.r.protected",
        "merged" => "table.r.merged",
        "invalid" => "table.r.invalid",
        _ => return None,
    })
}

/// `failed` with the reason the kernel kept for it.
pub fn kernel_refusal(reason: &str, failed: io::Error) -> io::Error {
    let reason: Value = match serde_json::from_str(reason) {
        Ok(reason) => reason,
        Err(error) => return unsure(&failed, &format!("its reason is unreadable: {error}")),
    };
    match (reason["refused"].as_str(), reason["message"].as_str()) {
        // Rolled back in the kernel: nothing changed.
        (Some("failed"), Some(message)) => io::Error::other(format!("{failed}: {message}")),
        (Some(""), _) => unsure(&failed, "the kernel gave no reason"),
        (Some(code), _) => match kernel_key(code) {
            Some(key) => refused(key),
            None => unsure(&failed, &format!("unknown reason {code}")),
        },
        _ => unsure(&failed, &format!("no reason in {reason}")),
    }
}

/// An apply whose outcome nobody knows; changes may have landed.
pub fn unsure(failed: &io::Error, why: &str) -> io::Error {
    log::warn!("table: R macro outcome unknown: {failed}; {why}");
    refused("table.r.unsure")
}

impl Snapshot {
    pub fn parse(text: &str) -> io::Result<Self> {
        if text.len() as u64 > BYTES {
            return Err(refused("table.r.too_big"));
        }
        let json: Value = serde_json::from_str(text).map_err(|_| unread())?;
        if let Some(error) = json["error"].as_str() {
            return Err(io::Error::other(Unreadable(error.to_string())));
        }
        if let Some(code) = json["refused"].as_str() {
            return Err(kernel_key(code).map_or_else(
                || io::Error::other(format!("the selection was refused: {code}")),
                refused,
            ));
        }
        let rows = usize::try_from(json["rows"].as_u64().ok_or_else(unread)?).map_err(|_| unread())?;
        let columns =
            usize::try_from(json["columns"].as_u64().ok_or_else(unread)?).map_err(|_| unread())?;
        let count = rows
            .checked_mul(columns)
            .filter(|n| (1..=LIMIT).contains(n))
            .ok_or_else(unread)?;
        let cells = json["cells"].as_array().ok_or_else(unread)?;
        if cells.len() != count || json["range"].as_str().is_none() {
            return Err(unread());
        }
        for cell in cells {
            if !valid(cell) {
                return Err(unread());
            }
            if cell.get("formula").is_some_and(|f| f.as_str().is_none()) {
                return Err(unread());
            }
        }
        Ok(Self {
            json,
            rows,
            columns,
        })
    }

    pub fn input(&self) -> io::Result<String> {
        let mut text = format!("{}\t{}\n", self.rows, self.columns);
        for cell in self.json["cells"].as_array().ok_or_else(unread)? {
            let value = match cell["kind"].as_str() {
                Some("text" | "number") => cell["value"].as_str().ok_or_else(unread)?.to_string(),
                Some("empty") => String::new(),
                _ => cell["value"].to_string(),
            };
            text.push_str(&format!(
                "{}\t{}",
                cell["kind"].as_str().ok_or_else(unread)?,
                hex(&value)
            ));
            if let Some(formula) = cell.get("formula") {
                text.push('\t');
                text.push_str(&hex(formula.as_str().ok_or_else(unread)?));
            }
            text.push('\n');
        }
        Ok(text)
    }

    pub fn instructions(&self, text: &str) -> io::Result<String> {
        if text.len() as u64 > BYTES {
            return Err(bad());
        }
        let mut lines = text.lines();
        if lines.next() != Some("ELY_R_1") {
            return Err(bad());
        }
        let mut writes = Vec::new();
        let mut indices = std::collections::HashSet::new();
        let mut bold = None;
        let mut autofit = false;
        let mut done = false;
        for line in lines {
            if done {
                return Err(bad());
            }
            let fields: Vec<_> = line.split('\t').collect();
            match fields.as_slice() {
                ["W", index, kind, encoded] => {
                    let index: usize = index.parse().map_err(|_| bad())?;
                    if index >= self.rows * self.columns
                        || !indices.insert(index)
                        || self.json["cells"][index].get("formula").is_some()
                    {
                        return Err(bad());
                    }
                    let text = unhex(encoded)?;
                    // Calc holds no empty text: "" clears, as in Excel.
                    let kind = if *kind == "text" && text.is_empty() { "empty" } else { *kind };
                    let value = match kind {
                        "text" => Value::String(text),
                        "empty" if text.is_empty() => Value::Null,
                        "number" => {
                            let n: f64 = text.parse().map_err(|_| bad())?;
                            if !n.is_finite() {
                                return Err(bad());
                            }
                            Value::String(text)
                        }
                        "boolean" if text == "true" || text == "false" => json!(text == "true"),
                        _ => return Err(bad()),
                    };
                    let cell = json!({"index": index, "kind": kind, "value": value});
                    if !valid(&cell) {
                        return Err(bad());
                    }
                    writes.push(cell);
                }
                ["H", flag] if bold.is_none() && matches!(*flag, "true" | "false") => {
                    bold = Some(*flag == "true")
                }
                ["A"] if !autofit => autofit = true,
                ["DONE"] => done = true,
                _ => return Err(bad()),
            }
        }
        if !done {
            return Err(bad());
        }
        let batch =
            json!({"snapshot": self.json, "writes": writes, "bold": bold, "autofit": autofit})
                .to_string();
        if batch.len() as u64 > BYTES {
            return Err(refused("table.r.batch_too_big"));
        }
        Ok(batch)
    }
}

fn valid(cell: &Value) -> bool {
    let value = &cell["value"];
    match cell["kind"].as_str() {
        Some("text") => value
            .as_str()
            .is_some_and(|s| !s.contains('\0') && s.len() <= 32767),
        Some("number") => value
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .is_some_and(f64::is_finite),
        Some("boolean") => value.is_boolean(),
        Some("empty") => value.is_null() || value.as_str() == Some(""),
        _ => false,
    }
}

fn hex(text: &str) -> String {
    if text.is_empty() {
        return "-".into();
    }
    text.bytes().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> io::Result<String> {
    if text == "-" {
        return Ok(String::new());
    }
    if text.is_empty() || !text.len().is_multiple_of(2) || !text.is_ascii() {
        return Err(bad());
    }
    let bytes = (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).map_err(|_| bad()))
        .collect::<io::Result<Vec<_>>>()?;
    String::from_utf8(bytes).map_err(|_| bad())
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn snapshot() -> Snapshot {
        Snapshot::parse(
            &json!({"range":"$Sheet1.$A$1:$F$1", "rows":1,"columns":6,"cells":[
                {"kind":"text","value":"  áé \"hola\"  "}, {"kind":"text","value":"00123"},
                {"kind":"number","value":"1.25"}, {"kind":"boolean","value":true},
                {"kind":"empty","value":null}, {"kind":"number","value":"2","formula":"=1+1"}
            ]})
            .to_string(),
        )
        .expect("snapshot")
    }

    #[test]
    fn typed_values_and_utf8_round_trip() {
        let s = snapshot();
        assert!(s.input().expect("input").contains("boolean\t74727565"));
        for text in ["", "  áé \"hola\"  ", "00123", "\t\n", "=1+1"] {
            assert_eq!(unhex(&hex(text)).expect("UTF-8"), text);
        }
        let output = format!(
            "ELY_R_1\nW\t0\ttext\t{}\nH\ttrue\nA\nDONE\n",
            hex("áé \"hola\"")
        );
        let batch: Value =
            serde_json::from_str(&s.instructions(&output).expect("batch")).expect("json");
        assert_eq!(batch["writes"][0]["value"], "áé \"hola\"");
        assert_eq!(batch["snapshot"]["cells"][5]["formula"], "=1+1");
    }

    #[test]
    fn writes_keep_each_cell_type() {
        let output = "ELY_R_1\nW\t0\ttext\t3030313233\nW\t1\ttext\t-\nW\t2\tnumber\t2d312e3235\nW\t3\tboolean\t66616c7365\nW\t4\tempty\t-\nDONE\n";
        let batch: Value =
            serde_json::from_str(&snapshot().instructions(output).expect("typed writes"))
                .expect("batch");
        let writes = &batch["writes"];
        assert_eq!(writes[0]["value"], "00123");
        // Calc holds no empty text: "" clears the cell.
        assert_eq!(writes[1]["kind"], "empty");
        assert!(writes[1]["value"].is_null());
        assert_eq!(writes[2]["value"], "-1.25");
        assert_eq!(writes[3]["value"], false);
        assert!(writes[4]["value"].is_null());
        assert_eq!(writes.as_array().expect("writes").len(), 5);
    }

    #[test]
    fn decimal_strings_preserve_precision_and_scientific_notation() {
        for number in [
            "1.0000000000000002",
            "0.12345678901234566",
            "1E-007",
            "1E+020",
            "-0",
            "4.9406564584124654e-324",
        ] {
            let source = json!({"range":"S!A1","rows":1,"columns":1,"cells":[{"kind":"number","value":number}]}).to_string();
            let snapshot = Snapshot::parse(&source).expect("number");
            assert!(
                snapshot
                    .input()
                    .expect("input")
                    .contains(&format!("number\t{}", hex(number)))
            );
            let output = format!("ELY_R_1\nW\t0\tnumber\t{}\nDONE\n", hex(number));
            let batch: Value =
                serde_json::from_str(&snapshot.instructions(&output).expect("instructions"))
                    .expect("JSON");
            assert_eq!(batch["snapshot"]["cells"][0]["value"], number);
            assert_eq!(batch["writes"][0]["value"], number);
        }
    }

    #[test]
    fn all_instructions_validate_before_applying() {
        for bad in [
            "W\t6\ttext\t61",
            "W\t5\ttext\t61",
            "W\t0\tnumber\t4e614e",
            "W\t0\ttext\t00",
            "W\t0\ttext\tff",
            "H\tmaybe",
            "A\nA",
            "W\t0\ttext\t61\nW\t0\ttext\t62",
            "DELETE",
        ] {
            assert!(
                snapshot()
                    .instructions(&format!("ELY_R_1\n{bad}\nDONE\n"))
                    .is_err(),
                "{bad}"
            );
        }
        assert!(snapshot().instructions("ELY_R_1\nA\n").is_err());
        assert!(snapshot().instructions("ELY_R_1\nDONE\nA").is_err());
    }

    #[test]
    fn selection_failures_are_not_blamed_on_r() {
        // Each cell is valid; together they pass 4 MiB.
        let big = "a".repeat(32767);
        let cells: Vec<_> = (0..130).map(|_| json!({"kind": "text", "value": big})).collect();
        let source = json!({"range": "S!A1:A130", "rows": 130, "columns": 1, "cells": cells}).to_string();
        assert_eq!(refusal(&Snapshot::parse(&source).expect_err("too big")), Some("table.r.too_big"));
        let odd = r#"{"range":"S!A1","rows":1,"columns":1,"cells":[{"kind":"date","value":"1"}]}"#;
        for broken in ["{", odd] {
            assert_eq!(refusal(&Snapshot::parse(broken).expect_err(broken)), Some("table.r.unreadable"));
        }
    }

    #[test]
    fn kernel_refusals_keep_their_own_words() {
        let codes = [
            "range", "cells", "too_big", "batch_too_big", "unreadable", "undo_off", "busy", "changed",
            "protected", "locked", "merged", "invalid", "rollback",
        ];
        let keys: std::collections::HashSet<_> = codes
            .iter()
            .map(|code| {
                let reason = json!({"refused": code, "message": ""}).to_string();
                let error = kernel_refusal(&reason, io::Error::other("macro.apply"));
                refusal(&error).unwrap_or_else(|| panic!("{code} is worded"))
            })
            .collect();
        assert_eq!(keys.len(), codes.len(), "no two causes share words");
        assert!(!keys.contains("table.r.refused"), "none is the generic failure");
        let failed = json!({"refused": "failed", "message": "kernel said"}).to_string();
        let error = kernel_refusal(&failed, io::Error::other("macro.apply"));
        assert_eq!(refusal(&error), None);
        assert!(error.to_string().ends_with("kernel said"), "{error}");
        // No known reason: the batch may have landed.
        for unknown in [json!({"refused": "", "message": ""}).to_string(), json!({"refused": "odd"}).to_string(), json!({}).to_string(), "{".into()] {
            let error = kernel_refusal(&unknown, io::Error::other("macro.apply"));
            assert_eq!(refusal(&error), Some("table.r.unsure"), "{unknown}");
        }
        let selection = json!({"refused": "cells"}).to_string();
        assert_eq!(refusal(&Snapshot::parse(&selection).expect_err("refused")), Some("table.r.cells"));
    }

    #[test]
    fn a_batch_over_the_limit_is_not_blamed_on_r() {
        let sizes: Vec<usize> = (0..64).map(|i| if i == 63 { 32477 } else { 32767 }).collect();
        let cells: Vec<_> = sizes.iter().map(|n| json!({"kind": "text", "value": "a".repeat(*n)})).collect();
        let source = json!({"range": "S!A1:A64", "rows": 64, "columns": 1, "cells": cells}).to_string();
        let snapshot = Snapshot::parse(&source).expect("the selection fits");
        assert!(snapshot.input().expect("input").len() as u64 <= BYTES, "R's input fits");
        let mut output = String::from("ELY_R_1
");
        for (index, n) in sizes.iter().enumerate() {
            output.push_str(&format!("W	{index}	text	{}
", hex(&"b".repeat(*n))));
        }
        output.push_str("DONE
");
        assert!(output.len() as u64 <= BYTES, "R's output fits");
        let error = snapshot.instructions(&output).expect_err("the batch does not");
        assert_eq!(refusal(&error), Some("table.r.batch_too_big"));
    }
}
