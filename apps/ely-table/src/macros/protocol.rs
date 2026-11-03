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

fn bad() -> io::Error {
    io::Error::other("Invalid R macro data or instructions")
}

impl Snapshot {
    pub fn parse(text: &str) -> io::Result<Self> {
        if text.len() as u64 > BYTES {
            return Err(bad());
        }
        let json: Value = serde_json::from_str(text)?;
        if let Some(error) = json["error"].as_str() {
            return Err(io::Error::other(format!("R macro selection: {error}")));
        }
        let rows = usize::try_from(json["rows"].as_u64().ok_or_else(bad)?).map_err(|_| bad())?;
        let columns =
            usize::try_from(json["columns"].as_u64().ok_or_else(bad)?).map_err(|_| bad())?;
        let count = rows
            .checked_mul(columns)
            .filter(|n| (1..=LIMIT).contains(n))
            .ok_or_else(bad)?;
        let cells = json["cells"].as_array().ok_or_else(bad)?;
        if cells.len() != count || json["range"].as_str().is_none() {
            return Err(bad());
        }
        for cell in cells {
            validate_value(cell)?;
            if cell.get("formula").is_some_and(|f| f.as_str().is_none()) {
                return Err(bad());
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
        for cell in self.json["cells"].as_array().ok_or_else(bad)? {
            let value = match cell["kind"].as_str() {
                Some("text" | "number") => cell["value"].as_str().ok_or_else(bad)?.to_string(),
                Some("empty") => String::new(),
                _ => cell["value"].to_string(),
            };
            text.push_str(&format!(
                "{}\t{}",
                cell["kind"].as_str().ok_or_else(bad)?,
                hex(&value)
            ));
            if let Some(formula) = cell.get("formula") {
                text.push('\t');
                text.push_str(&hex(formula.as_str().ok_or_else(bad)?));
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
                    let value = match *kind {
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
                    validate_value(&cell)?;
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
            return Err(bad());
        }
        Ok(batch)
    }
}

fn validate_value(cell: &Value) -> io::Result<()> {
    let value = &cell["value"];
    let valid = match cell["kind"].as_str() {
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
    };
    if valid { Ok(()) } else { Err(bad()) }
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
        assert_eq!(writes[1]["value"], "");
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
}
