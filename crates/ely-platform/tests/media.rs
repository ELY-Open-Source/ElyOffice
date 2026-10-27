//! The system player against a small H.264/AAC clip.

#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
fn main() {
    use std::time::{Duration, Instant};

    use ely_platform::media::{Player, State};

    let pump = |until: &dyn Fn() -> bool| {
        let end = Instant::now() + Duration::from_secs(10);
        while !until() {
            assert!(Instant::now() < end, "the player never got there");
            wait();
        }
    };
    let clip = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/media/clip.mp4");
    let player = Player::open(&clip).expect("opens");
    pump(&|| player.state() != State::Loading);
    let State::Ready { seconds, video } = player.state() else {
        panic!("not ready: {:?}", player.state());
    };
    assert!((seconds - 1.0).abs() < 0.2, "{seconds}");
    assert_eq!(video, Some((160, 90)));
    player.play();
    let frames = std::cell::Cell::new(0);
    pump(&|| {
        frames.set(frames.get() + usize::from(player.frame().is_some()));
        player.position() > 0.3 && frames.get() > 0
    });
    assert!(player.playing());
    // At its end it stops; a loop starts over.
    pump(&|| player.ended());
    assert_eq!(player.tick(false), Ok(false), "done, nothing to draw");
    assert_eq!(player.tick(true), Ok(true), "the loop goes on");
    pump(&|| player.playing() && player.position() < seconds - 0.2);
    // A pause near the end holds, loop or not.
    player.pause();
    player.seek(seconds - 0.02);
    pump(&|| player.position() >= seconds - 0.05);
    assert!(!player.ended());
    assert_eq!(player.tick(true), Ok(false));
    assert!(!player.playing(), "the pause holds");
    player.play();
    player.pause();
    assert!(Player::open(std::path::Path::new("/no/such/clip.mp4")).is_err());
    let dir = tempfile::tempdir().expect("a temp dir");
    let junk = dir.path().join("junk.mp4");
    std::fs::write(&junk, b"not a video at all").expect("writes");
    let broken = Player::open(&junk).expect("opens the file");
    pump(&|| broken.tick(false) != Ok(true));
    assert!(matches!(broken.tick(false), Err(reason) if !reason.is_empty()));
    // A track no one decodes fails; no half playback.
    let bytes = std::fs::read(&clip).expect("reads the clip");
    for (at, codec, track) in [(461, b"avc1", "video"), (1234, b"mp4a", "sound")] {
        assert_eq!(&bytes[at..at + 4], codec);
        let mut odd = bytes.clone();
        odd[at..at + 4].copy_from_slice(b"elyx");
        let path = dir.path().join(format!("odd-{track}.mp4"));
        std::fs::write(&path, odd).expect("writes");
        let player = Player::open(&path).expect("opens the file");
        pump(&|| player.state() != State::Loading);
        let state = player.state();
        assert!(matches!(&state, State::Failed(reason) if !reason.is_empty()), "odd {track}: {state:?}");
    }
    println!("media: plays, shows frames, and fails with reasons");
}

/// macOS plays through the main run loop.
#[cfg(target_os = "macos")]
fn wait() {
    use objc2_foundation::{NSDate, NSRunLoop};
    NSRunLoop::currentRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.02));
}

/// Media Foundation and GStreamer play on their own threads.
#[cfg(any(windows, target_os = "linux"))]
fn wait() {
    std::thread::sleep(std::time::Duration::from_millis(20));
}

#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
fn main() {
    println!("SKIP: no system player is built on this platform");
}
