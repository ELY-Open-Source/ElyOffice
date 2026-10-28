//! One stamp for client and worker; stale workers refused.

fn main() {
    let mut stamp: u64 = 0xcbf2_9ce4_8422_2325;
    for source in [
        "../ely-pdf-worker/shim/ely_mupdf.c",
        "../ely-pdf-worker/shim/ely_pages.c",
        "../ely-pdf-worker/shim/ely_insert.c",
        "../ely-pdf-worker/shim/ely_text.c",
        "../ely-pdf-worker/shim/ely_write.c",
        "../ely-pdf-worker/shim/ely_pen.c",
        "../ely-pdf-worker/shim/ely_place.c",
        "../ely-pdf-worker/shim/ely_annot.c",
        "../ely-pdf-worker/shim/ely_form.c",
        "../ely-pdf-worker/shim/ely_ocr.c",
        "../ely-pdf-worker/shim/ely_outline.c",
        "../ely-pdf-worker/shim/ely_meta.c",
        "../ely-pdf-worker/shim/ely_redact.c",
        "../ely-pdf-worker/shim/ely_alt.c",
        "../ely-pdf-worker/shim/ely_clean.c",
        "../ely-pdf-worker/shim/ely_secure.c",
        "../ely-pdf-worker/shim/ely_sign.c",
        "../ely-pdf-worker/shim/ely_output.c",
        "../ely-pdf-worker/shim/ely_story.c",
        "../ely-pdf-worker/shim/ely_shim.h",
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
    ] {
        println!("cargo:rerun-if-changed={source}");
        for byte in std::fs::read(source).expect("a source file") {
            stamp = (stamp ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
        }
    }
    println!("cargo:rustc-env=ELY_PDF_BUILD={stamp:016x}");
}
