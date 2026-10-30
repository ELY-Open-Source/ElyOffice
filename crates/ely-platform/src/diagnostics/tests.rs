use std::time::{Duration, SystemTime};

use super::{Part, Redact, bundle, crashes_in, log_part};
use crate::AppId;

#[test]
fn only_this_apps_recent_crashes_are_picked() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["ely-doc-2026-10-08-1.ips", "ely-lok-worker-2026-10-08-2.ips", "ely-docs-x.ips", "Safari-1.ips"] {
        std::fs::write(dir.path().join(name), "report").unwrap();
    }
    let names = vec!["ely-doc".to_string(), "ely-lok-worker".to_string()];
    let picked = crashes_in(dir.path(), &names, SystemTime::now()).unwrap();
    let files: Vec<_> = picked.iter().map(|part| part.path.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(files, ["ely-doc-2026-10-08-1.ips", "ely-lok-worker-2026-10-08-2.ips"]);
    let later = SystemTime::now() + Duration::from_secs(8 * 24 * 3600);
    assert!(crashes_in(dir.path(), &names, later).unwrap().is_empty(), "a week old is left out");
}

#[test]
fn redaction_hides_every_path_url_and_name() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("ely.log");
    let lines = [
        "opened /Users/ana/Documents/plan.docx for Ana Lee",
        "opened /Volumes/ClientSecret/payroll.pdf",
        r"saved C:\Users\ana\pay.xlsx",
        "linked 'https://intra.example/q3' and (~/notes.md)",
        "signed by Ana",
        "opened '/Volumes/ClientSecret/Tax Salary Report.pdf' fine",
        r"saved C:\Clients\Tax Salary Report.pdf",
        "author: 李",
        r#"{"usedImages":[{"path":"/Volumes/ClientSecret/Tax \"Wage Sheet\".pdf"}]}"#,
    ];
    std::fs::write(&log, lines.join("\n")).unwrap();
    let parts = [Part { path: log.clone(), bytes: 4096 }];
    // The short account name comes before the full author name.
    let redact = Redact { names: vec!["Ana".into(), "Ana Lee".into(), "李".into()] };
    let text = bundle(AppId::Doc, &parts, Some(&redact)).unwrap();
    for secret in ["/Users/ana", "ClientSecret", "payroll", r"C:\Users", "intra.example", "notes.md", "Lee", "Ana", "Salary", "Report", "李", "Wage", "Sheet"] {
        assert!(!text.contains(secret), "{secret} stays in:\n{text}");
    }
    assert!(text.contains("opened '<path>' fine") && text.contains("linked '<path>' and (<path>"), "{text}");
    assert!(!text.contains(&dir.path().display().to_string()), "the header names only the file");
    let kept = bundle(AppId::Doc, &parts, None).unwrap();
    assert!(kept.contains("/Volumes/ClientSecret/payroll.pdf"), "unredacted when not asked");
}

#[cfg(unix)]
#[test]
fn an_unreadable_log_is_an_error_not_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("ely.log");
    assert_eq!(log_part(log.clone()).unwrap(), None, "no log yet is nothing");
    std::os::unix::fs::symlink(&log, &log).unwrap();
    assert!(log_part(log).is_err(), "a log that cannot be read shows");
}

#[cfg(windows)]
#[test]
fn an_unreadable_log_is_an_error_not_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("ely.log");
    assert_eq!(log_part(log.clone()).unwrap(), None, "no log yet is nothing");
    // A junction to itself: no privilege resolves it.
    let done = std::process::Command::new("cmd").args(["/c", "mklink", "/J"]).arg(&log).arg(&log).output().unwrap();
    assert!(done.status.success(), "mklink: {}", String::from_utf8_lossy(&done.stdout));
    let read = log_part(log);
    assert!(read.is_err(), "a log that cannot be read shows: {read:?}");
}

#[test]
fn a_cut_log_starts_at_a_whole_line() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("ely.log");
    let first = "opened '/Volumes/ClientSecret/Tax Salary Report.pdf'\n";
    std::fs::write(&log, format!("{first}later line\n")).unwrap();
    // The cut falls inside the path, on its file name.
    let bytes = (first.len() - "Tax Salary Report.pdf'\n".len() + "later line\n".len()) as u64;
    let part = Part { path: log, bytes: bytes + "Tax Salary Report.pdf'\n".len() as u64 - 10 };
    let text = bundle(AppId::Doc, &[part], Some(&Redact::default())).unwrap();
    assert!(!text.contains("Report") && text.contains("later line"), "{text}");
}

#[test]
fn a_long_log_sends_only_its_newest_part() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("ely.log");
    std::fs::write(&log, "old line\nnew line\n").unwrap();
    let text = bundle(AppId::Doc, &[Part { path: log, bytes: 9 }], None).unwrap();
    assert!(text.ends_with("new line\n") && !text.contains("old line"), "{text}");
}

#[test]
fn unreadable_names_stop_the_redaction() {
    let broken = || Err(std::io::Error::other("settings: author cannot be 3"));
    assert!(Redact::of(Ok("Ana".into()), broken()).is_err(), "no names, no export");
    assert!(Redact::of(Err(std::io::Error::other("no account")), Ok(Default::default())).is_err());
    let names = Redact::of(Ok("Ana".into()), Ok(Default::default())).unwrap().names;
    assert!(names.contains(&"Ana".to_string()));
}
