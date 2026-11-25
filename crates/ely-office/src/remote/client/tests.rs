//! The command line's words, input and refusals.

use std::path::Path;

use ely_platform::AppId;

use super::{Cli, Fail, Plan, plan, with_stdin, write_new};

const CLI: Cli = Cli {
    app: AppId::Agent,
    nouns: &[("text", |words| match *words {
        ["get", file] => Ok(vec!["get".into(), file.into()]),
        _ => Err("unknown command".into()),
    })],
    usage: "usage",
    exact: &["get"],
    blanks: &[("docx", b"blank")],
    exports: &["docx", "pdf"],
};

fn planned(words: &[&str]) -> Result<Plan, Fail> {
    plan(&CLI, &words.iter().map(|word| word.to_string()).collect::<Vec<_>>())
}

fn code(words: &[&str]) -> i32 {
    planned(words).expect_err("refused").code
}

fn absolute(file: &str) -> String {
    super::super::absolute(file).expect("an absolute path")
}

#[test]
fn verbs_stand_before_the_noun_and_files_become_absolute() {
    assert_eq!(planned(&["docs"]), Ok(Plan::Ask(vec!["docs".into()], false)));
    assert_eq!(planned(&["save", "a.docx"]), Ok(Plan::Ask(vec!["save".into(), absolute("a.docx")], false)));
    assert_eq!(planned(&["open", "a.docx"]), Ok(Plan::Open(absolute("a.docx"))));
    assert_eq!(planned(&["new", "b.DOCX"]), Ok(Plan::New(absolute("b.DOCX"), b"blank")));
    let export = planned(&["export", "a.docx", "a.pdf"]);
    assert_eq!(export, Ok(Plan::Ask(vec!["export".into(), absolute("a.docx"), absolute("a.pdf")], false)));
    assert_eq!(planned(&["text", "get", "x"]), Ok(Plan::Ask(vec!["get".into(), "x".into()], true)), "the noun's own");
    assert_eq!(planned(&["--help"]), Ok(Plan::Usage));
    assert!(Path::new(&absolute("a.docx")).is_absolute());
}

#[test]
fn wrong_words_are_usage_errors() {
    assert_eq!(code(&["open"]), 2, "no file");
    assert_eq!(code(&["export", "a.docx"]), 2, "no target");
    assert_eq!(code(&["text", "drop", "x"]), 2, "no such noun verb");
    let new = planned(&["new", "a.odt"]).expect_err("an unknown type");
    assert_eq!(new, Fail { code: 2, said: "a.odt: cannot create this type (supported: .docx)".into() });
    let to = planned(&["export", "a.docx", "a.xlsx"]).expect_err("an unknown target");
    assert_eq!(to.said, "a.xlsx: cannot export to this type (supported: .docx, .pdf)");
    let same = planned(&["export", "a.docx", "./a.docx"]).expect_err("onto itself");
    assert_eq!(same.said, "export target cannot be the same as the open file");
}

#[test]
fn a_dash_reads_standard_input_once() {
    let words = |words: &[&str]| words.iter().map(|word| word.to_string()).collect::<Vec<_>>();
    let read = with_stdin(words(&["text", "replace", "a", "-", "dog"]), || Ok("cat\nkitten".into()));
    assert_eq!(read, Ok(words(&["text", "replace", "a", "cat\nkitten", "dog"])));
    let untouched = with_stdin(words(&["docs"]), || panic!("no dash, no read"));
    assert_eq!(untouched, Ok(words(&["docs"])));
    let twice = with_stdin(words(&["text", "replace", "a", "-", "-"]), || Ok(Vec::new()));
    assert_eq!(twice.expect_err("two dashes").code, 2);
    let binary = with_stdin(words(&["-"]), || Ok(vec![0xff, 0xfe]));
    assert_eq!(binary.expect_err("not UTF-8").said, "standard input is not valid UTF-8 text");
    let big = with_stdin(words(&["-"]), || Ok(vec![b'a'; super::LIMIT + 1]));
    assert_eq!(big.expect_err("too big").said, "standard input exceeds the 64 MiB limit");
}

#[test]
fn new_writes_the_blank_and_never_over_a_file() {
    let dir = tempfile::tempdir().expect("a folder");
    let file = dir.path().join("a.docx").display().to_string();
    assert_eq!(write_new(&file, b"blank"), Ok(()));
    assert_eq!(std::fs::read(&file).expect("written"), b"blank");
    std::fs::write(&file, b"mine").expect("the user's own");
    assert_eq!(write_new(&file, b"blank"), Err(Fail { code: 1, said: format!("{file}: file already exists") }));
    assert_eq!(std::fs::read(&file).expect("kept"), b"mine");
}

#[test]
fn an_app_not_running_exits_three() {
    ely_platform::test_root();
    let asked = super::ask(AppId::Agent, &["docs".into()]).expect_err("nothing runs");
    assert_eq!(asked, Fail { code: 3, said: "ElyAgent is not running; use open FILE to start it".into() });
}

#[test]
fn answered_paths_parse_back_as_json() {
    let files = vec![r"C:\a b\x.docx".to_string(), "/tmp/two\nlines \"q\".docx".to_string()];
    let listed: Vec<String> = serde_json::from_str(&crate::remote::files_json(&files)).expect("a JSON list");
    assert_eq!(listed, files, "a newline stays inside its path");
    let named: String = serde_json::from_str(&crate::remote::path_json(&files[1])).expect("a JSON path");
    assert_eq!(named, files[1]);
    assert_eq!(crate::remote::files_json(&[]), "[]", "no file open");
}
