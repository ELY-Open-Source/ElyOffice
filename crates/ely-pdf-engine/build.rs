//! One stamp for client and worker; stale workers refused.

fn main() {
    let mut stamp: u64 = 0xcbf2_9ce4_8422_2325;
    // Every shim file: a new one can't be missed.
    let mut sources: Vec<String> = std::fs::read_dir("../ely-pdf-worker/shim")
        .expect("the shim folder")
        .map(|entry| entry.expect("a shim entry").path().display().to_string())
        .collect();
    sources.sort();
    println!("cargo:rerun-if-changed=../ely-pdf-worker/shim");
    sources.extend(
        [
            "src/proto.rs",
            "../ely-pdf-worker/src/main.rs",
            "../ely-pdf-worker/src/ffi.rs",
            "../ely-pdf-worker/src/edit.rs",
            "../ely-pdf-worker/src/content.rs",
            "src/edit.rs",
            "src/content.rs",
            "src/marks.rs",
            "../ely-pdf-worker/src/marks.rs",
            "../ely-pdf-worker/src/ocr.rs",
            "../ely-pdf-worker/src/structure.rs",
            "../ely-pdf-worker/src/redact.rs",
            "src/redact.rs",
            "src/structure.rs",
            "src/ocr.rs",
            "../ely-pdf-worker/src/secure.rs",
            "src/secure.rs",
            "../ely-pdf-worker/src/sign.rs",
            "src/sign.rs",
            "../ely-pdf-worker/src/output.rs",
            "src/output.rs",
            "../ely-ipc/src/lib.rs",
        ]
        .map(String::from),
    );
    for source in sources {
        println!("cargo:rerun-if-changed={source}");
        for byte in std::fs::read(source).expect("a source file") {
            stamp = (stamp ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
        }
    }
    println!("cargo:rustc-env=ELY_PDF_BUILD={stamp:016x}");
}
