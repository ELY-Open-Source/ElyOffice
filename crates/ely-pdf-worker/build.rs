//! Builds the MuPDF shim into the worker.

use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("set by cargo")).join("../..");
    let mupdf = std::env::var_os("ELY_MUPDF_DIR").map_or_else(|| root.join("engines/mupdf-build"), PathBuf::from);
    println!("cargo:rerun-if-env-changed=ELY_MUPDF_DIR");
    // cc's rebuild lines are off; shim edits recompile it.
    println!("cargo:rerun-if-changed=shim");
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    // MSVC's libmupdf.lib holds its third parties and PKCS#7.
    let libs: &[&str] =
        if windows { &["libmupdf.lib"] } else { &["libmupdf-pkcs7.a", "libmupdf.a", "libmupdf-third.a"] };
    for lib in libs {
        println!("cargo:rerun-if-changed={}", mupdf.join(lib).display());
        if !mupdf.join(lib).is_file() {
            panic!("no {lib} in {} (run scripts/build-mupdf.sh or set ELY_MUPDF_DIR)", mupdf.display());
        }
    }
    cc::Build::new()
        .file("shim/ely_mupdf.c")
        .file("shim/ely_pages.c")
        .file("shim/ely_insert.c")
        .file("shim/ely_text.c")
        .file("shim/ely_write.c")
        .file("shim/ely_pen.c")
        .file("shim/ely_place.c")
        .file("shim/ely_annot.c")
        .file("shim/ely_form.c")
        .file("shim/ely_ocr.c")
        .file("shim/ely_outline.c")
        .file("shim/ely_meta.c")
        .file("shim/ely_redact.c")
        .file("shim/ely_alt.c")
        .file("shim/ely_clean.c")
        .file("shim/ely_secure.c")
        .file("shim/ely_sign.c")
        .file("shim/ely_output.c")
        .file("shim/ely_story.c")
        .include(root.join("reference/mupdf/include"))
        .include(openssl(&root, "includedir"))
        .warnings(true)
        .cargo_metadata(false)
        .compile("ely_mupdf");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("set by cargo"));
    let bin = "ely-pdf-worker";
    let shim = if windows { "ely_mupdf.lib" } else { "libely_mupdf.a" };
    println!("cargo:rustc-link-arg-bin={bin}={}", out.join(shim).display());
    for lib in libs {
        println!("cargo:rustc-link-arg-bin={bin}={}", mupdf.join(lib).display());
    }
    // Signing and checking need OpenSSL, linked in whole.
    let crypto = if windows { "libcrypto.lib" } else { "libcrypto.a" };
    println!("cargo:rustc-link-arg-bin={bin}={}", PathBuf::from(openssl(&root, "libdir")).join(crypto).display());
    if windows {
        for lib in ["crypt32.lib", "ws2_32.lib", "advapi32.lib", "user32.lib"] {
            println!("cargo:rustc-link-arg-bin={bin}={lib}");
        }
        return;
    }
    // What a static libcrypto needs, as its pkg-config says.
    let needs = std::process::Command::new("pkg-config")
        .args(["--static", "--libs-only-l", "libcrypto"])
        .output()
        .expect("pkg-config, to find OpenSSL");
    assert!(needs.status.success(), "pkg-config knows no static libcrypto");
    for lib in String::from_utf8(needs.stdout).expect("UTF-8 flags").split_whitespace().filter(|lib| *lib != "-lcrypto") {
        println!("cargo:rustc-link-arg-bin={bin}={lib}");
    }
    println!("cargo:rustc-link-arg-bin={bin}=-lm");
    // Tesseract inside MuPDF is C++.
    let cxx = if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") { "-lc++" } else { "-lstdc++" };
    println!("cargo:rustc-link-arg-bin={bin}={cxx}");
}

/// An OpenSSL directory: vcpkg's static build on Windows, else pkg-config's.
fn openssl(root: &std::path::Path, variable: &str) -> String {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let dir = root.join("engines/vcpkg/installed/x64-windows-static");
        let sub = if variable == "includedir" { "include" } else { "lib" };
        assert!(dir.join("lib/libcrypto.lib").is_file(), "no static OpenSSL in {}", dir.display());
        return dir.join(sub).display().to_string();
    }
    let out = std::process::Command::new("pkg-config")
        .args([&format!("--variable={variable}"), "libcrypto"])
        .output()
        .expect("pkg-config, to find OpenSSL");
    assert!(out.status.success(), "pkg-config knows no libcrypto");
    String::from_utf8(out.stdout).expect("a UTF-8 path").trim().to_string()
}
