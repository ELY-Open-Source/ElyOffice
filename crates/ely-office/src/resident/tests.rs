use std::{sync::mpsc, time::Duration};

use ely_platform::AppId;

use super::beside;

#[test]
fn the_start_does_not_wait_for_the_unit_check() {
    let (release, held) = mpsc::channel::<()>();
    let (done, finished) = mpsc::channel();
    let (returned, back) = mpsc::channel();
    std::thread::spawn(move || {
        beside(AppId::Doc, move || {
            held.recv().expect("the test releases the check");
            done.send(()).expect("the test waits for the end");
            Ok(())
        });
        returned.send(()).expect("the test waits for the start");
    });
    back.recv_timeout(Duration::from_secs(5)).expect("the start went on while the check was held");
    assert!(finished.try_recv().is_err(), "the check ran before it was released");
    release.send(()).expect("the check is waiting");
    finished.recv_timeout(Duration::from_secs(5)).expect("the check ran after release");
}
