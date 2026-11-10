use std::{
    fs,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use ely_platform::{
    AppId, logging,
    update::{self, Check, IDLE_TIMEOUT, check, download, platform, verify_file},
};
use semver::Version;

mod served;
use served::*;

#[test]
fn a_newer_release_for_this_platform_is_offered_and_an_equal_or_older_one_is_not() {
    let sha = hex_of(&payload()).to_uppercase();
    let base = serve(|base| {
        let body = manifest("0.2.0", &platform(), base, &sha);
        vec![signature_of("latest.json", &body), ("/latest.json".into(), body)]
    });
    let url = format!("{base}/latest.json");

    let found = check(&Version::new(0, 1, 0), &url, KEY).unwrap();
    assert_eq!(found, Check::Available(release(&base, sha.to_lowercase())));
    assert_eq!(check(&Version::new(0, 2, 0), &url, KEY).unwrap(), Check::Current);
    assert_eq!(check(&Version::new(0, 3, 0), &url, KEY).unwrap(), Check::Current);
}

#[test]
fn a_manifest_without_this_platform_a_bad_manifest_and_a_missing_one_all_fail_loudly() {
    let base = serve(|base| {
        let other = manifest("0.2.0", "plan9-mips", base, "00");
        let bad = Reply::Body(b"{not json".to_vec());
        vec![
            signature_of("other.json", &other),
            ("/other.json".into(), other),
            signature_of("bad.json", &bad),
            ("/bad.json".into(), bad),
        ]
    });
    let current = Version::new(0, 1, 0);

    assert!(
        error_text(check(&current, &format!("{base}/other.json"), KEY)).contains("has no build for")
    );
    assert!(error_text(check(&current, &format!("{base}/bad.json"), KEY)).contains("not valid"));
    assert!(error_text(check(&current, &format!("{base}/none.json"), KEY)).contains("cannot fetch"));
}

#[test]
fn a_manifest_unsigned_or_signed_for_another_file_or_key_is_refused() {
    let sha = hex_of(&payload());
    let base = serve(|base| {
        let body = manifest("0.2.0", &platform(), base, &sha);
        let (_, swapped) = signature_of("ely-tree-windows-x86_64.json", &body);
        vec![
            ("/unsigned.json".into(), body.clone()),
            ("/swapped.json".into(), body.clone()),
            ("/swapped.json.sig".into(), swapped),
            ("/forged.json".into(), body.clone()),
            ("/forged.json.sig".into(), Reply::Body(b"AAAA".to_vec())),
            signature_of("latest.json", &body),
            ("/latest.json".into(), body),
        ]
    });
    let current = Version::new(0, 1, 0);
    let said = |name: &str, key: &str| error_text(check(&current, &format!("{base}/{name}"), key));

    assert!(said("unsigned.json", KEY).contains("unsigned.json.sig"), "no signature");
    assert!(said("swapped.json", KEY).contains("not signed by the release key"), "another file's");
    assert!(said("forged.json", KEY).contains("not signed by the release key"), "a forgery");
    assert!(said("latest.json", update::KEY).contains("not signed by the release key"), "another key");
}

#[test]
fn plain_http_to_other_hosts_and_credentials_in_the_url_are_refused_before_any_request() {
    let current = Version::new(0, 1, 0);

    assert!(error_text(check(&current, "http://example.com/latest.json", KEY)).contains("not https"));
    let sneaky = "http://127.0.0.1:1@192.168.31.221:9/latest.json";
    assert!(error_text(check(&current, sneaky, KEY)).contains("carries credentials"));
}

#[test]
fn a_redirect_is_followed_on_this_machine_and_refused_when_it_leaves_for_plain_http() {
    let sha = hex_of(&payload());
    let base = serve(|base| {
        let body = manifest("0.2.0", &platform(), base, &sha);
        vec![
            signature_of("first", &body),
            ("/latest.json".into(), body),
            ("/first".into(), Reply::Redirect("/second".into())),
            ("/second".into(), Reply::Redirect("/latest.json".into())),
            (
                "/leave".into(),
                Reply::Redirect("http://example.com/latest.json".into()),
            ),
        ]
    });
    let current = Version::new(0, 1, 0);

    let followed = check(&current, &format!("{base}/first"), KEY).unwrap();
    assert!(matches!(followed, Check::Available(_)), "{followed:?}");
    assert!(error_text(check(&current, &format!("{base}/leave"), KEY)).contains("not https"));
}

#[test]
fn a_download_is_verified_reported_and_moved_into_place() {
    let body = payload();
    let base = serve(|_| vec![("/ElyDoc.bin".into(), Reply::Body(body.clone()))]);
    let dir = tempfile::tempdir().unwrap();
    let mut seen = Vec::new();

    let path = download(
        &release(&base, hex_of(&body)),
        dir.path(),
        &AtomicBool::new(false),
        IDLE_TIMEOUT,
        &mut |done, total| {
            seen.push((done, total));
        },
    )
    .unwrap()
    .expect("not cancelled");

    assert_eq!(path, dir.path().join("ElyDoc.bin"));
    assert_eq!(fs::read(&path).unwrap(), body);
    assert!(!dir.path().join("ElyDoc.bin.part").exists());
    assert_eq!(
        seen.last().copied(),
        Some((body.len() as u64, Some(body.len() as u64)))
    );
    assert!(
        seen.windows(2).all(|pair| pair[0].0 < pair[1].0),
        "progress went backwards"
    );
    verify_file(&path, &hex_of(&body)).unwrap();
}

#[test]
fn a_wrong_hash_leaves_no_file_behind() {
    let body = payload();
    let base = serve(|_| vec![("/ElyDoc.bin".into(), Reply::Body(body.clone()))]);
    let dir = tempfile::tempdir().unwrap();

    let result = download(
        &release(&base, hex_of(b"other")),
        dir.path(),
        &AtomicBool::new(false),
        IDLE_TIMEOUT,
        &mut |_, _| {},
    );

    assert!(error_text(result).contains("the release says"));
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn the_download_error_survives_a_failed_cleanup() {
    use std::os::unix::fs::PermissionsExt;

    let body = payload();
    let base = serve(|_| vec![("/ElyDoc.bin".into(), Reply::Body(body.clone()))]);
    let dir = tempfile::tempdir().unwrap();
    let lock = |mode| fs::set_permissions(dir.path(), fs::Permissions::from_mode(mode)).unwrap();
    let mut locked = false;

    let result = download(
        &release(&base, hex_of(b"other")),
        dir.path(),
        &AtomicBool::new(false),
        IDLE_TIMEOUT,
        &mut |_, _| {
            if !locked {
                lock(0o555);
                locked = true;
            }
        },
    );

    let still_writable = fs::File::create(dir.path().join("probe")).is_ok();
    lock(0o755);
    if still_writable {
        println!("SKIP: permissions do not bind this user");
        return;
    }
    assert!(error_text(result).contains("the release says"));
}

fn closed_within(flag: &AtomicBool, limit: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < limit {
        if flag.load(Ordering::Relaxed) {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn a_stalled_download_fails_and_frees_its_socket() {
    let closed = Arc::new(AtomicBool::new(false));
    let base = serve({
        let closed = closed.clone();
        move |_| vec![("/ElyDoc.bin".into(), Reply::Stall(closed))]
    });
    let dir = tempfile::tempdir().unwrap();

    let stalled = download(
        &release(&base, hex_of(b"x")),
        dir.path(),
        &AtomicBool::new(false),
        Duration::from_millis(600),
        &mut |_, _| {},
    );

    let text = error_text(stalled);
    assert!(text.contains("stalled"), "{text}");
    assert!(
        closed_within(&closed, Duration::from_secs(5)),
        "the client kept the socket open"
    );
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn a_cancel_is_answered_at_once_and_the_socket_is_freed_within_the_idle_time() {
    let closed = Arc::new(AtomicBool::new(false));
    let base = serve({
        let closed = closed.clone();
        move |_| vec![("/ElyDoc.bin".into(), Reply::Stall(closed))]
    });
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);

    let started = Instant::now();
    let outcome = thread::scope(|scope| {
        scope.spawn(|| {
            // The download stalls before it is cancelled.
            thread::sleep(Duration::from_millis(300));
            cancel.store(true, Ordering::Relaxed);
        });
        download(
            &release(&base, hex_of(b"x")),
            dir.path(),
            &cancel,
            Duration::from_secs(1),
            &mut |_, _| {},
        )
    });

    assert_eq!(outcome.unwrap(), None);
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "the cancel waited for the socket"
    );
    assert!(
        closed_within(&closed, Duration::from_secs(5)),
        "the client kept the socket open"
    );
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn a_cancelled_download_returns_none_and_leaves_no_file_behind() {
    let body = payload();
    let base = serve(|_| vec![("/ElyDoc.bin".into(), Reply::Body(body.clone()))]);
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);

    let outcome = download(
        &release(&base, hex_of(&body)),
        dir.path(),
        &cancel,
        IDLE_TIMEOUT,
        &mut |_, _| {
            cancel.store(true, Ordering::Relaxed);
        },
    )
    .unwrap();

    assert_eq!(outcome, None);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn a_file_the_user_brought_is_checked_against_its_published_hash() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("ElyDoc.dmg");
    fs::write(&file, b"installer bytes").unwrap();

    verify_file(&file, &hex_of(b"installer bytes").to_uppercase()).unwrap();
    assert!(
        error_text(verify_file(&file, &hex_of(b"tampered bytes"))).contains("the release says")
    );
    assert!(error_text(verify_file(&dir.path().join("none"), "00")).contains("cannot open"));
}

#[test]
fn checks_and_downloads_are_logged_with_their_outcome() {
    let logs = tempfile::tempdir().unwrap();
    logging::init_in(AppId::Doc, logs.path()).unwrap();
    let body = payload();
    let sha = hex_of(&body);
    let base = serve(|base| {
        let latest = manifest("0.2.0", &platform(), base, &sha);
        vec![
            signature_of("latest.json", &latest),
            ("/latest.json".into(), latest),
            ("/ElyDoc.bin".into(), Reply::Body(body.clone())),
        ]
    });
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);

    check(&Version::new(0, 1, 0), &format!("{base}/latest.json"), KEY).unwrap();
    check(&Version::new(0, 1, 0), &format!("{base}/none.json"), KEY).unwrap_err();
    download(
        &release(&base, sha),
        dir.path(),
        &cancel,
        IDLE_TIMEOUT,
        &mut |_, _| {
            cancel.store(true, Ordering::Relaxed);
        },
    )
    .unwrap();
    log::logger().flush();

    let text = fs::read_to_string(logs.path().join("ely.log")).unwrap();
    for line in [
        "update check: 127.0.0.1 offers 0.2.0",
        "update check: 127.0.0.1 failed",
        "update download: 0.2.0 cancelled",
    ] {
        assert!(text.contains(line), "no '{line}' in:\n{text}");
    }
}
