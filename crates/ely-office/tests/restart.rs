//! A failed engine restart keeps the author for retries.

use ely_office::{Engine as _, Lok};
use ely_platform::AppId;

#[test]
fn a_restart_signs_with_the_newest_name_and_survives_a_failure() {
    let Some(install) = ely_lok::test_install() else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let home = tempfile::tempdir().expect("a temp dir");
    // The one test here: nothing else reads HOME.
    unsafe { std::env::set_var("HOME", home.path()) };
    let profile = ely_platform::data_dir(AppId::Doc).expect("a data dir").join("lok-profile");
    std::fs::create_dir_all(&profile).expect("the profile");
    let mut worker = ely_lok::Worker::launch(&install, &profile).expect("the worker");
    worker.ready().expect("checks");
    worker.set_author("Ada").expect("signs");
    // The new worker's check cannot write its sample here.
    let blocker = profile.join("lok-verify.fodt");
    if blocker.exists() {
        std::fs::remove_file(&blocker).expect("the last sample goes");
    }
    std::fs::create_dir(&blocker).expect("a folder in the way");
    assert!(Lok::restart(AppId::Doc, &mut worker).is_err(), "the check fails");
    assert_eq!(worker.author(), Some("Ada"), "the author survives a failed restart");
    std::fs::remove_dir(&blocker).expect("cleared");
    Lok::restart(AppId::Doc, &mut worker).expect("the next try restarts");
    assert_eq!(worker.author(), Some("Ada"));
    // A name asked while down signs after the restart.
    worker.shutdown().expect("stops");
    assert!(worker.set_author("Latest").is_err(), "a stopped engine cannot take it");
    Lok::restart(AppId::Doc, &mut worker).expect("restarts");
    assert_eq!(worker.author(), Some("Latest"), "the newest name signs");
}
