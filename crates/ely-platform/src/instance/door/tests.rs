use super::*;

fn files(files: &[PathBuf]) -> Launch {
    Launch { files: files.to_vec(), new_window: false }
}

fn opened(dir: &Path) -> std::sync::mpsc::Receiver<Launch> {
    let (sender, heard) = std::sync::mpsc::channel();
    let sender = std::sync::Mutex::new(sender);
    let door = Door::open(dir).expect("a door");
    door.serve(move |launch| sender.lock().expect("a sender").send(launch).is_ok(), |_| Err("no requests".into()))
        .expect("served");
    heard
}

/// Writes a door file whose token is wrong.
fn forge(dir: &Path) -> String {
    let door = dir.join(FILE);
    let text = fs::read_to_string(&door).expect("the door");
    let (head, _) = text.rsplit_once(' ').expect("a token");
    fs::write(&door, format!("{head} {}\n", hex(&[7; 32]))).expect("a forged door");
    text
}

#[test]
fn a_hand_over_brings_every_path_whole() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let heard = opened(dir.path());
    let files = vec![
        PathBuf::from("C:/a b/笔记 #1.md"),
        PathBuf::from("/tmp/x.md"),
    ];
    hand(dir.path(), &self::files(&files), Duration::from_secs(2)).expect("handed");
    assert_eq!(heard.recv_timeout(Duration::from_secs(2)), Ok(self::files(&files)));
    hand(dir.path(), &Launch::default(), Duration::from_secs(2)).expect("an empty hand-over raises");
    assert_eq!(heard.recv_timeout(Duration::from_secs(2)), Ok(Launch::default()));
    let window = Launch { files, new_window: true };
    hand(dir.path(), &window, Duration::from_secs(2)).expect("a window asked");
    assert_eq!(heard.recv_timeout(Duration::from_secs(2)), Ok(window));
}

#[test]
fn an_old_copy_is_told_apart_before_anything_is_sent() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let door = Door::open(dir.path()).expect("a door");
    // An old door opens with an empty list.
    let old = thread::spawn(move || {
        let (stream, _) = door.listener.accept().expect("a caller");
        let (_, token) = next(&mut &stream, SHOWN).expect("the token");
        assert!(same(&token, &door.token));
        ely_ipc::write_frame(&mut &stream, OPEN, 0, &[]).expect("open");
        next(&mut &stream, LONGEST).map(|(tag, _)| tag)
    });
    let window = Launch { files: vec![PathBuf::from("x.md")], new_window: true };
    let error = hand(dir.path(), &window, Duration::from_secs(2)).expect_err("an old copy");
    assert_eq!(error.kind(), io::ErrorKind::Unsupported, "{error}");
    assert!(old.join().expect("answered").is_err(), "no frame went to the old copy");
}

#[test]
fn a_current_refusal_is_not_called_old() {
    let dir = tempfile::tempdir().expect("a temp dir");
    Door::open(dir.path()).expect("a door").serve(|_| false, |_| Err("no requests".into())).expect("served");
    let window = Launch { files: vec![PathBuf::from("x.md")], new_window: true };
    let error = hand(dir.path(), &window, Duration::from_secs(2)).expect_err("refused");
    assert_ne!(error.kind(), io::ErrorKind::Unsupported, "{error}");
}

#[test]
fn a_wrong_token_hands_nothing_over() {
    for _ in 0..12 {
        let dir = tempfile::tempdir().expect("a temp dir");
        let heard = opened(dir.path());
        forge(dir.path());
        let refused = hand(
            dir.path(),
            &files(&[PathBuf::from("x.md")]),
            Duration::from_millis(300),
        );
        assert_eq!(
            refused.map_err(|error| error.kind()),
            Err(io::ErrorKind::PermissionDenied)
        );
        assert!(heard.try_recv().is_err(), "nothing reached the copy");
    }
}

#[test]
fn an_old_door_file_is_waited_out() {
    for _ in 0..5 {
        let dir = tempfile::tempdir().expect("a temp dir");
        let heard = opened(dir.path());
        let real = forge(dir.path());
        let door = dir.path().join(FILE);
        let fixer = thread::spawn(move || {
            // The stale door stays this long, then the fix.
            thread::sleep(Duration::from_millis(100));
            fs::write(door, real).expect("the real door");
        });
        let files = vec![PathBuf::from("x.md")];
        hand(dir.path(), &self::files(&files), Duration::from_secs(3))
            .expect("handed once the door is right");
        fixer.join().expect("fixed");
        assert_eq!(heard.recv_timeout(Duration::from_secs(2)), Ok(self::files(&files)));
    }
}

#[test]
fn a_dripping_caller_neither_blocks_nor_stays() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let heard = opened(dir.path());
    let text = fs::read_to_string(dir.path().join(FILE)).expect("the door");
    let port: u16 = text
        .split_whitespace()
        .next()
        .and_then(|word| word.parse().ok())
        .expect("a port");
    let mut drip = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("connected");
    let started = Instant::now();
    let dripper = thread::spawn(move || {
        // A token frame's header, a byte at a time.
        for byte in [32u8, 0, 0, 0, TOKEN, 0, 0, 0, 0, 0, 0, 0, 0]
            .into_iter()
            .chain([0; 32])
        {
            if drip.write_all(&[byte]).is_err() {
                return;
            }
            thread::sleep(Duration::from_millis(300));
        }
    });
    let files = vec![PathBuf::from("x.md")];
    hand(dir.path(), &self::files(&files), Duration::from_secs(2)).expect("handed past the drip");
    assert_eq!(heard.recv_timeout(Duration::from_secs(2)), Ok(self::files(&files)));
    dripper.join().expect("dripped");
    let held = started.elapsed();
    assert!(
        held < PATIENCE + Duration::from_secs(2),
        "the drip was cut at the deadline, not after {held:?}"
    );
}

#[test]
fn files_the_app_did_not_take_are_reported() {
    let dir = tempfile::tempdir().expect("a temp dir");
    Door::open(dir.path())
        .expect("a door")
        .serve(|_| false, |_| Err("no requests".into()))
        .expect("served");
    let started = Instant::now();
    let untaken = hand(dir.path(), &files(&[PathBuf::from("x.md")]), Duration::from_secs(3));
    assert!(untaken.is_err(), "untaken files are no success");
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "sent paths are not sent again"
    );
}

#[test]
fn no_door_fails_after_the_wait() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let started = Instant::now();
    let missing = hand(dir.path(), &Launch::default(), Duration::from_millis(300));
    assert_eq!(
        missing.map_err(|error| error.kind()),
        Err(io::ErrorKind::NotFound)
    );
    assert!(started.elapsed() >= Duration::from_millis(300));
}
