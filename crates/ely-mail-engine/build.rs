use std::{env, fs, io::ErrorKind, path::Path};

const KEYS: [&str; 1] = ["ELY_MICROSOFT_CLIENT_ID"];

/// Client IDs: CI secrets, else the local cred.txt.
fn main() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cred.txt");
    println!("cargo:rerun-if-changed={}", file.display());
    for key in KEYS {
        println!("cargo:rerun-if-env-changed={key}");
    }
    let text = match fs::read_to_string(&file) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => return,
        Err(err) => panic!("cred.txt unreadable: {err}"),
    };
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            panic!("cred.txt line {}: expected KEY=value", number + 1);
        };
        let key = key.trim();
        assert!(
            KEYS.contains(&key),
            "cred.txt line {}: unknown key {key}",
            number + 1
        );
        if env::var_os(key).is_none() {
            println!("cargo:rustc-env={key}={}", value.trim());
        }
    }
}
