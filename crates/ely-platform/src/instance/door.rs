//! The running copy's door: a second copy hands it files.

use std::{
    ffi::OsString,
    fs,
    io::{self, Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

const FILE: &str = "instance.door";
const TOKEN: u8 = 1;
const PATH: u8 = 2;
const DONE: u8 = 3;
const REFUSED: u8 = 4;
const OPEN: u8 = 5;
/// A request from another program, then its answer.
const ASK: u8 = 6;
const ANSWER: u8 = 7;
/// How long a caller waits for an answer.
const ANSWERING: Duration = Duration::from_secs(90);
/// How long one caller may take, start to end.
const PATIENCE: Duration = Duration::from_secs(5);
/// The largest frame before the token is shown.
const SHOWN: usize = 64;
/// The largest path frame: a whole Windows path.
const LONGEST: usize = 128 * 1024;
/// A request or answer: whole cells, JSON-escaped.
const ASKED: usize = 64 * 1024 * 1024;

/// The running copy's listener; its token keeps others out.
pub struct Door {
    listener: TcpListener,
    token: [u8; 32],
}

impl Door {
    /// Opens a local door; its address goes in `dir`.
    pub(super) fn open(dir: &Path) -> io::Result<Door> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let mut token = [0; 32];
        getrandom::fill(&mut token).map_err(|_| io::Error::other("system randomness failed"))?;
        let port = listener.local_addr()?.port();
        let text = format!("{port} {} {}\n", std::process::id(), hex(&token));
        // Whole or absent to a reader.
        let fresh = dir.join("instance.door.new");
        write_private(&fresh, text.as_bytes())?;
        fs::rename(&fresh, dir.join(FILE))?;
        Ok(Door { listener, token })
    }

    /// Files go to `deliver`, false meaning untaken; requests to `reply`.
    pub fn serve(
        self,
        deliver: impl Fn(Vec<PathBuf>) -> bool + Send + Sync + 'static,
        reply: impl Fn(String) -> Result<String, String> + Send + Sync + 'static,
    ) -> io::Result<()> {
        let (deliver, reply) = (Arc::new(deliver), Arc::new(reply));
        thread::Builder::new()
            .name("instance door".into())
            .spawn(move || {
                for stream in self.listener.incoming() {
                    let (token, deliver, reply) = (self.token, deliver.clone(), reply.clone());
                    // A stalled caller must not hold up the next.
                    let answered = stream.and_then(|stream| {
                        thread::Builder::new()
                            .name("instance caller".into())
                            .spawn(move || {
                                if let Err(error) = answer(&stream, &token, &*deliver, &*reply) {
                                    log::warn!("instance: a hand-over failed: {error}");
                                }
                            })
                            .map(drop)
                    });
                    if let Err(error) = answered {
                        log::warn!("instance: a caller was not answered: {error}");
                    }
                }
            })?;
        Ok(())
    }
}

fn answer(
    stream: &TcpStream,
    token: &[u8],
    deliver: &dyn Fn(Vec<PathBuf>) -> bool,
    reply: &dyn Fn(String) -> Result<String, String>,
) -> io::Result<()> {
    let mut timed = Timed {
        stream,
        until: Instant::now() + PATIENCE,
    };
    let (tag, shown) = next(&mut timed, SHOWN)?;
    if tag != TOKEN || !same(&shown, token) {
        ely_ipc::write_frame(&mut &*stream, REFUSED, 0, &[])?;
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "a caller without the token",
        ));
    }
    ely_ipc::write_frame(&mut &*stream, OPEN, 0, &[])?;
    let mut files = Vec::new();
    let mut first = Some(next(&mut timed, ASKED)?);
    if let Some((ASK, request)) = &first {
        let request = String::from_utf8(request.clone()).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "a request not in UTF-8"))?;
        log::info!("instance: a request came ({} bytes)", request.len());
        return match reply(request) {
            Ok(answer) => ely_ipc::write_frame(&mut &*stream, ANSWER, 0, answer.as_bytes()),
            Err(why) => ely_ipc::write_frame(&mut &*stream, REFUSED, 0, why.as_bytes()),
        };
    }
    loop {
        match first.take().map_or_else(|| next(&mut timed, LONGEST), Ok)? {
            (PATH, bytes) => files.push(path(bytes)?),
            (DONE, _) => break,
            (tag, _) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("frame {tag} is not a hand-over's"),
                ));
            }
        }
    }
    log::info!("instance: another copy handed over {} files", files.len());
    // No receipt: the caller reports the files untaken.
    if !deliver(files) {
        return Err(io::Error::other("the app is quitting and took no files"));
    }
    ely_ipc::write_frame(&mut &*stream, DONE, 0, &[])
}

/// A stream whose reads all end by one deadline.
struct Timed<'a> {
    stream: &'a TcpStream,
    until: Instant,
}

impl Read for Timed<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = self.until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "a caller held the door too long",
            ));
        }
        self.stream.set_read_timeout(Some(left))?;
        (&mut &*self.stream).read(buf)
    }
}

/// Hands `files` to `dir`'s door, waiting up to `wait`.
pub(super) fn hand(dir: &Path, files: &[PathBuf], wait: Duration) -> io::Result<()> {
    let deadline = Instant::now() + wait;
    loop {
        // Nothing handed yet, so every failure here may retry.
        match knock(dir).and_then(|(stream, token)| Ok((admit(&stream, &token)?, stream))) {
            Ok((true, stream)) => return give(&stream, files),
            // An old door file; the new one is coming.
            Ok((false, _)) if Instant::now() < deadline => {}
            Ok((false, _)) => {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "it refused the door's token",
                ));
            }
            Err(_) if Instant::now() < deadline => {}
            Err(error) => return Err(error),
        }
        thread::sleep(Duration::from_millis(100));
    }
}

/// Sends `request` to `dir`'s door; the app's answer or refusal.
pub(super) fn ask(dir: &Path, request: &str) -> io::Result<String> {
    let (stream, token) = knock(dir)?;
    if !admit(&stream, &token)? {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "it refused the door's token"));
    }
    ely_ipc::write_frame(&mut &stream, ASK, 0, request.as_bytes())?;
    stream.set_read_timeout(Some(ANSWERING))?;
    let text = |bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned();
    match next(&mut &stream, ASKED)? {
        (ANSWER, bytes) => Ok(text(bytes)),
        (REFUSED, bytes) => Err(io::Error::other(text(bytes))),
        (tag, _) => Err(unexpected(tag)),
    }
}

/// Connects to `dir`'s door; nothing sent, so retries are safe.
fn knock(dir: &Path) -> io::Result<(TcpStream, Vec<u8>)> {
    let text = fs::read_to_string(dir.join(FILE))?;
    let bad = || io::Error::new(io::ErrorKind::InvalidData, format!("an unreadable {FILE}"));
    let mut words = text.split_whitespace();
    let port: u16 = words
        .next()
        .and_then(|word| word.parse().ok())
        .ok_or_else(bad)?;
    let pid: u32 = words
        .next()
        .and_then(|word| word.parse().ok())
        .ok_or_else(bad)?;
    let token = words.next().and_then(unhex).ok_or_else(bad)?;
    let stream =
        TcpStream::connect_timeout(&SocketAddr::from((Ipv4Addr::LOCALHOST, port)), PATIENCE)?;
    stream.set_read_timeout(Some(PATIENCE))?;
    lend_foreground(pid);
    Ok((stream, token))
}

/// Shows the token; false when the door refuses it.
fn admit(stream: &TcpStream, token: &[u8]) -> io::Result<bool> {
    ely_ipc::write_frame(&mut &*stream, TOKEN, 0, token)?;
    match next(&mut &*stream, SHOWN)? {
        (OPEN, _) => Ok(true),
        (REFUSED, _) => Ok(false),
        (tag, _) => Err(unexpected(tag)),
    }
}

/// Sends the files once admitted; never retried.
fn give(stream: &TcpStream, files: &[PathBuf]) -> io::Result<()> {
    for file in files {
        ely_ipc::write_frame(&mut &*stream, PATH, 0, &bytes(file))?;
    }
    ely_ipc::write_frame(&mut &*stream, DONE, 0, &[])?;
    // The app may answer after the door's own wait.
    stream.set_read_timeout(Some(PATIENCE * 2))?;
    match next(&mut &*stream, SHOWN)? {
        (DONE, _) => Ok(()),
        (tag, _) => Err(unexpected(tag)),
    }
}

fn unexpected(tag: u8) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("it answered frame {tag}"),
    )
}

fn next(stream: &mut impl Read, cap: usize) -> io::Result<(u8, Vec<u8>)> {
    match ely_ipc::read_capped(stream, cap)? {
        Some((tag, _, payload)) => Ok((tag, payload)),
        None => Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "the other copy hung up",
        )),
    }
}

/// Lets the running copy bring its window to the front.
#[cfg(windows)]
fn lend_foreground(pid: u32) {
    // Refused when not in front; then nothing to lend.
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(pid) };
}

#[cfg(not(windows))]
fn lend_foreground(_: u32) {}

fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path)?.write_all(bytes)
}

/// Equal tokens, compared in constant time.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0, |diff, (x, y)| diff | (x ^ y)) == 0
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    let pairs = text.as_bytes().chunks(2);
    pairs
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

#[cfg(unix)]
fn bytes(path: &Path) -> Vec<u8> {
    std::os::unix::ffi::OsStrExt::as_bytes(path.as_os_str()).to_vec()
}

#[cfg(unix)]
fn path(bytes: Vec<u8>) -> io::Result<PathBuf> {
    Ok(<OsString as std::os::unix::ffi::OsStringExt>::from_vec(bytes).into())
}

/// UTF-16 units, little-endian, so any Windows name survives.
#[cfg(windows)]
fn bytes(path: &Path) -> Vec<u8> {
    std::os::windows::ffi::OsStrExt::encode_wide(path.as_os_str())
        .flat_map(u16::to_le_bytes)
        .collect()
}

#[cfg(windows)]
fn path(bytes: Vec<u8>) -> io::Result<PathBuf> {
    if !bytes.len().is_multiple_of(2) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "a cut UTF-16 path",
        ));
    }
    let wide: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    Ok(<OsString as std::os::windows::ffi::OsStringExt>::from_wide(&wide).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened(dir: &Path) -> std::sync::mpsc::Receiver<Vec<PathBuf>> {
        let (sender, heard) = std::sync::mpsc::channel();
        let sender = std::sync::Mutex::new(sender);
        let door = Door::open(dir).expect("a door");
        door.serve(move |files| sender.lock().expect("a sender").send(files).is_ok(), |_| Err("no requests".into()))
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
        hand(dir.path(), &files, Duration::from_secs(2)).expect("handed");
        assert_eq!(heard.recv_timeout(Duration::from_secs(2)), Ok(files));
        hand(dir.path(), &[], Duration::from_secs(2)).expect("an empty hand-over raises");
        assert_eq!(heard.recv_timeout(Duration::from_secs(2)), Ok(vec![]));
    }

    #[test]
    fn a_wrong_token_hands_nothing_over() {
        for _ in 0..12 {
            let dir = tempfile::tempdir().expect("a temp dir");
            let heard = opened(dir.path());
            forge(dir.path());
            let refused = hand(
                dir.path(),
                &[PathBuf::from("x.md")],
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
                thread::sleep(Duration::from_millis(100));
                fs::write(door, real).expect("the real door");
            });
            let files = vec![PathBuf::from("x.md")];
            hand(dir.path(), &files, Duration::from_secs(3))
                .expect("handed once the door is right");
            fixer.join().expect("fixed");
            assert_eq!(heard.recv_timeout(Duration::from_secs(2)), Ok(files));
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
        hand(dir.path(), &files, Duration::from_secs(2)).expect("handed past the drip");
        assert_eq!(heard.recv_timeout(Duration::from_secs(2)), Ok(files));
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
        let untaken = hand(dir.path(), &[PathBuf::from("x.md")], Duration::from_secs(3));
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
        let missing = hand(dir.path(), &[], Duration::from_millis(300));
        assert_eq!(
            missing.map_err(|error| error.kind()),
            Err(io::ErrorKind::NotFound)
        );
        assert!(started.elapsed() >= Duration::from_millis(300));
    }
}
