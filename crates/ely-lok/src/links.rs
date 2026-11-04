//! Linked files: listed at open, read on request (LOK-11).

use std::io;

/// The files and servers a document's links read (0022, 0086).
pub const LIST: &str = ".uno:ExternalLinks";
/// Reads every link's source again; answers the failed ones (0086).
pub const UPDATE: &str = ".uno:ElyUpdateLinks";

/// Parses `LIST`.
pub fn sources(json: &str) -> io::Result<Vec<String>> {
    crate::sheet::strings(json, "links")
}

/// Parses a text's `LIST`: links kept as pictures (0088).
pub fn pictured(json: &str) -> io::Result<Vec<String>> {
    crate::sheet::strings(json, "pictured")
}

/// Parses `UPDATE`: the sources not read.
pub fn failed(json: &str) -> io::Result<Vec<String>> {
    crate::sheet::strings(json, "failed")
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_answer_names_its_failed_sources() {
        let json = r#"{"commandName":".uno:ElyUpdateLinks","failed":["file:///a/b.xlsx"]}"#;
        assert_eq!(super::failed(json).expect("parses"), ["file:///a/b.xlsx"]);
        assert!(super::failed(r#"{"links":[]}"#).is_err(), "no failed list is an error");
        assert_eq!(super::pictured(r#"{"links":[],"pictured":["a.pdf"]}"#).expect("parses"), ["a.pdf"]);
    }
}
