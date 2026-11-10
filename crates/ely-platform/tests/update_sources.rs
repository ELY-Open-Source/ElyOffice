//! A second source answers when the first cannot (R-11).

use std::{
    fs,
    net::TcpListener,
    sync::atomic::AtomicBool,
};

use ely_platform::update::{self, Check, IDLE_TIMEOUT, platform};
use semver::Version;

mod served;
use served::{KEY, Reply, error_text, hex_of, manifest, payload, release, serve, signature_of};

/// A port nobody listens on.
fn dead() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port())
}

#[test]
fn a_dead_first_source_gives_way_to_the_second_and_two_dead_ones_name_both() {
    let sha = hex_of(&payload());
    let base = serve(|base| {
        let body = manifest("0.2.0", &platform(), base, &sha);
        vec![signature_of("latest.json", &body), ("/latest.json".into(), body)]
    });
    let (gone, also_gone) = (dead(), dead());
    let sources = [format!("{gone}/latest.json"), format!("{base}/latest.json")];
    let found = update::check_any(&Version::new(0, 1, 0), &sources, KEY).unwrap();
    assert_eq!(found, Check::Available(release(&base, sha.clone())));
    // A living first source is the only one asked.
    let living = [format!("{base}/latest.json"), format!("{gone}/latest.json")];
    assert_eq!(update::check_any(&Version::new(0, 2, 0), &living, KEY).unwrap(), Check::Current);
    let both = [format!("{gone}/latest.json"), format!("{also_gone}/latest.json")];
    let said = error_text(update::check_any(&Version::new(0, 1, 0), &both, KEY));
    assert!(said.contains(&gone[7..]) && said.contains(&also_gone[7..]), "{said}");
    assert!(error_text(update::check_any(&Version::new(0, 1, 0), &[], KEY)).contains("no update source"));
}

#[test]
fn a_second_source_still_needs_the_signature() {
    let sha = hex_of(&payload());
    let base = serve(|base| vec![("/latest.json".into(), manifest("0.2.0", &platform(), base, &sha))]);
    let sources = [format!("{}/latest.json", dead()), format!("{base}/latest.json")];
    let said = error_text(update::check_any(&Version::new(0, 1, 0), &sources, KEY));
    assert!(said.contains("latest.json.sig"), "{said}");
}

#[test]
fn a_failed_download_gives_way_to_the_mirror_and_the_hash_still_rules() {
    let bytes = payload();
    let sha = hex_of(&bytes);
    let base = serve(|_| vec![("/ElyDoc.bin".into(), Reply::Body(bytes.clone()))]);
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);
    let gone = release(&dead(), sha.clone());
    let mirror = format!("{base}/ElyDoc.bin");
    let file = update::download_any(&gone, Some(&mirror), dir.path(), &cancel, IDLE_TIMEOUT, &mut |_, _| {}).unwrap().expect("a file");
    assert_eq!(fs::read(&file).unwrap(), bytes);
    // No mirror: the first failure stands.
    assert!(update::download_any(&gone, None, dir.path(), &cancel, IDLE_TIMEOUT, &mut |_, _| {}).is_err());
    // A mirror serving other bytes is refused.
    let wrong = release(&dead(), hex_of(b"other"));
    let said = error_text(update::download_any(&wrong, Some(&mirror), dir.path(), &cancel, IDLE_TIMEOUT, &mut |_, _| {}));
    assert!(said.contains("hash") || said.contains("sha256") || said.contains("SHA"), "{said}");
}

/// The live sources: GitHub, then Ely's relay.
fn live(file: &str) -> [String; 2] {
    ["https://github.com/ELY-Open-Source/ElyOffice", "https://office.elygpui.com"].map(|home| format!("{home}/releases/{file}"))
}

#[test]
#[ignore = "asks the live relay; run it with github.com blocked"]
fn with_github_out_of_reach_the_relay_answers_and_serves_the_file() {
    let manifests = live(&format!("latest/download/ely-mark-{}.json", platform()));
    let blocked = update::check(&Version::new(0, 0, 1), &manifests[0], update::KEY);
    assert!(blocked.is_err(), "github.com answers here: block it first");
    let found = update::check_any(&Version::new(0, 0, 1), &manifests, update::KEY).expect("the relay answers");
    let Check::Available(release) = found else {
        panic!("0.0.1 is older than any release");
    };
    let mirror = release.url.replacen("https://github.com/ELY-Open-Source/ElyOffice", "https://office.elygpui.com", 1);
    assert_ne!(mirror, release.url, "the file is a GitHub release file");
    let dir = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);
    let file = update::download_any(&release, Some(&mirror), dir.path(), &cancel, IDLE_TIMEOUT, &mut |_, _| {}).unwrap().expect("a file");
    update::verify_file(&file, &release.sha256).expect("the published hash");
    eprintln!("relay: {} is {} bytes, hash as signed", release.version, fs::metadata(&file).unwrap().len());
}
