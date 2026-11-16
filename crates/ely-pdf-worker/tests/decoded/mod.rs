//! A saved file's bytes with its streams decompressed.

use std::path::Path;

/// Saves compress, so byte checks read this.
pub fn decoded(path: &Path) -> Vec<u8> {
    let mut pdf = lopdf::Document::load(path).expect("lopdf opens it");
    pdf.decompress();
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("writes");
    bytes
}
