//! One running copy per app; two would wipe shared data.

mod door;

use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

pub use door::Door;

use crate::AppId;

/// The lock this process holds while it runs.
static HELD: OnceLock<File> = OnceLock::new();

/// How long a second copy waits for the door.
const WAIT: Duration = Duration::from_secs(10);

/// What starting a copy found.
pub enum Start {
    /// This copy runs; its door takes later copies' files.
    First(Door),
    /// A running copy took the files; this one ends.
    Handed,
}

/// Takes `app`'s lock, or hands `files` to its holder.
pub fn hold(app: AppId, files: &[PathBuf]) -> io::Result<Start> {
    let dir = crate::data_dir(app).map_err(io::Error::other)?;
    std::fs::create_dir_all(&dir)?;
    start(app, &dir, files)
}

/// Sends `request` to the running `app`; its answer.
pub fn ask(app: AppId, request: &str) -> io::Result<String> {
    let dir = crate::data_dir(app).map_err(io::Error::other)?;
    if !dir.join("instance.door").try_exists()? {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("{} is not running", app.name())));
    }
    door::ask(&dir, request)
}

fn start(app: AppId, dir: &Path, files: &[PathBuf]) -> io::Result<Start> {
    match take(app, dir) {
        Ok(()) => Ok(Start::First(Door::open(dir)?)),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => match door::hand(dir, files, WAIT) {
            Ok(()) => Ok(Start::Handed),
            Err(handing) => Err(io::Error::new(handing.kind(), format!("{error} and did not take the files: {handing}"))),
        },
        Err(error) => Err(error),
    }
}

fn take(app: AppId, dir: &Path) -> io::Result<()> {
    let file = std::fs::OpenOptions::new().write(true).create(true).truncate(false).open(dir.join("instance.lock"))?;
    match lock(&file)? {
        None => HELD.set(file).map_err(|_| io::Error::other("this process took its instance lock twice")),
        Some(holder) => Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("{} is already running ({holder})", app.name()))),
    }
}

/// Every app's lock under `root`, or the first app running.
pub(crate) fn closed(root: &Path) -> io::Result<Result<Vec<File>, AppId>> {
    let mut held = Vec::new();
    for app in AppId::ALL {
        let dir = root.join(app.slug());
        if !dir.try_exists()? {
            continue;
        }
        let file = std::fs::OpenOptions::new().write(true).create(true).truncate(false).open(dir.join("instance.lock"))?;
        if lock(&file)?.is_some() {
            return Ok(Err(app));
        }
        held.push(file);
    }
    Ok(Ok(held))
}

/// Locks `file`, or names its holder as the OS reports.
#[cfg(unix)]
fn lock(file: &File) -> io::Result<Option<String>> {
    use std::os::unix::io::AsRawFd;
    let fd = file.as_raw_fd();
    loop {
        let mut want: libc::flock = unsafe { std::mem::zeroed() };
        want.l_type = libc::F_WRLCK as _;
        want.l_whence = libc::SEEK_SET as _;
        if unsafe { libc::fcntl(fd, libc::F_SETLK, &want) } == 0 {
            return Ok(None);
        }
        let error = io::Error::last_os_error();
        if !matches!(error.raw_os_error(), Some(libc::EACCES | libc::EAGAIN)) {
            return Err(error);
        }
        let mut held = want;
        if unsafe { libc::fcntl(fd, libc::F_GETLK, &mut held) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // Let go between the two calls: try again.
        if i64::from(held.l_type) != i64::from(libc::F_UNLCK) {
            return Ok(Some(format!("process {}", held.l_pid)));
        }
    }
}

/// Windows names no holder; the lock alone refuses.
#[cfg(windows)]
fn lock(file: &File) -> io::Result<Option<String>> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx};
    const ERROR_LOCK_VIOLATION: i32 = 33;
    let flags = LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY;
    let mut overlapped = unsafe { std::mem::zeroed() };
    match unsafe { LockFileEx(file.as_raw_handle(), flags, 0, u32::MAX, u32::MAX, &mut overlapped) } {
        0 if io::Error::last_os_error().raw_os_error() == Some(ERROR_LOCK_VIOLATION) => Ok(Some("another copy holds its lock".into())),
        0 => Err(io::Error::last_os_error()),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_started_together_name_the_one_holder() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let exe = std::env::current_exe().expect("this test");
        let copies: Vec<_> = (0..6)
            .map(|_| {
                std::process::Command::new(&exe)
                    .args(["--exact", "instance::tests::one_copy", "--nocapture"])
                    .env("ELY_INSTANCE_DIR", dir.path())
                    .stdout(std::process::Stdio::piped())
                    .spawn()
                    .expect("a copy")
            })
            .collect();
        let said: Vec<String> = copies
            .into_iter()
            .map(|copy| String::from_utf8_lossy(&copy.wait_with_output().expect("ends").stdout).into_owned())
            .collect();
        let holders: Vec<&String> = said.iter().filter(|out| out.contains("HELD")).collect();
        assert_eq!(holders.len(), 1, "{said:?}");
        let holder = holders[0].split("HELD ").nth(1).and_then(|rest| rest.split_whitespace().next()).expect("its pid");
        let refused: Vec<&String> = said.iter().filter(|out| out.contains("REFUSED")).collect();
        assert_eq!(refused.len(), 5, "{said:?}");
        // Unix names the holder; Windows cannot ask.
        let named = if cfg!(unix) { format!("(process {holder})") } else { "(another copy holds its lock)".into() };
        for out in refused {
            assert!(out.contains(&named), "{out}");
        }
    }

    #[test]
    fn a_second_copy_hands_its_files_to_the_first() {
        use std::io::BufRead;
        let dir = tempfile::tempdir().expect("a temp dir");
        let exe = std::env::current_exe().expect("this test");
        let mut first = std::process::Command::new(&exe)
            .args(["--exact", "instance::tests::first_copy", "--nocapture"])
            .env("ELY_FIRST_DIR", dir.path())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("the first copy");
        let mut said = std::io::BufReader::new(first.stdout.take().expect("its output")).lines().map(|line| line.expect("a line"));
        assert!(said.by_ref().any(|line| line.contains("HELD")), "the first copy did not start");
        let file = std::path::PathBuf::from("C:/notes/a b.md");
        let second = start(AppId::Doc, dir.path(), std::slice::from_ref(&file)).expect("handed over");
        assert!(matches!(second, Start::Handed));
        let got = said.find(|line| line.contains("GOT"));
        assert_eq!(got.as_deref().and_then(|line| line.split("GOT ").nth(1)), Some(file.to_str().expect("text")));
        assert!(first.wait().expect("ends").success());
    }

    #[test]
    fn a_request_gets_the_app_s_answer_or_its_refusal() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let door = Door::open(dir.path()).expect("a door");
        let (sender, handed) = std::sync::mpsc::channel();
        let sender = std::sync::Mutex::new(sender);
        let reply = |request: String| match request.as_str() {
            "cell" => Ok("42".to_string()),
            "long" => Ok("汉".repeat(200_000)),
            other => Err(format!("{other} is unknown")),
        };
        door.serve(move |files| sender.lock().expect("a sender").send(files).is_ok(), reply).expect("served");
        assert_eq!(door::ask(dir.path(), "cell").expect("answered"), "42");
        assert_eq!(door::ask(dir.path(), "long").expect("a long answer").chars().count(), 200_000);
        assert_eq!(door::ask(dir.path(), &"x".repeat(300_000)).expect_err("echoed").to_string().len(), 300_011);
        let refused = door::ask(dir.path(), "save").expect_err("refused");
        assert_eq!(refused.to_string(), "save is unknown");
        let file = PathBuf::from("C:/a.xlsx");
        door::hand(dir.path(), std::slice::from_ref(&file), WAIT).expect("handed");
        assert_eq!(handed.recv().expect("files"), vec![file], "hand-overs still work");
    }

    /// The first copy: holds, then reports one hand-over.
    #[test]
    fn first_copy() {
        let Ok(dir) = std::env::var("ELY_FIRST_DIR") else {
            return;
        };
        let Ok(Start::First(door)) = start(AppId::Doc, Path::new(&dir), &[]) else {
            panic!("the first copy did not hold");
        };
        let (sender, heard) = std::sync::mpsc::channel();
        let sender = std::sync::Mutex::new(sender);
        door.serve(move |files| sender.lock().expect("a sender").send(files).is_ok(), |_| Err("no requests".into())).expect("served");
        println!("HELD");
        let files = heard.recv_timeout(std::time::Duration::from_secs(20)).expect("a hand-over");
        println!("GOT {}", files[0].display());
    }

    /// One copy, in a process of its own.
    #[test]
    fn one_copy() {
        let Ok(dir) = std::env::var("ELY_INSTANCE_DIR") else {
            return;
        };
        match take(AppId::Doc, Path::new(&dir)) {
            Ok(()) => {
                println!("HELD {} ", std::process::id());
                std::thread::sleep(std::time::Duration::from_millis(1500));
            }
            Err(error) => println!("REFUSED {error}"),
        }
    }
}
