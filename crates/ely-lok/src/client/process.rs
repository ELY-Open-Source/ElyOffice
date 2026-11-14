//! The worker process, started off the caller's thread.

use std::{
    fs::File,
    io::{self, PipeReader, PipeWriter},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc, mpsc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use super::{Stopper, at};
use crate::endpoint::Peer;

/// The worker process, or the spawn still starting it.
pub(crate) enum Process {
    Starting(mpsc::Receiver<io::Result<Started>>),
    Up(Child),
    Failed(io::ErrorKind, String),
    /// An idle engine taken over; set once its output ends.
    Adopted(u32, Arc<Peer>, Arc<AtomicBool>),
}

/// A spawned process, or one taken over.
pub(crate) enum Started {
    Spawned(Child),
    Adopted(u32, Arc<Peer>, Arc<AtomicBool>),
}

/// Sends what a start made, after the thread is up.
pub(super) type Sender = mpsc::Sender<io::Result<Started>>;

/// The child's stdout, and its stdin held until connected.
pub(super) type Ends = (PipeReader, PipeWriter);

/// The child's pipes; a log takes stderr.
pub(super) fn pipes(command: &mut Command, log: Option<File>) -> io::Result<(Ends, Option<PipeReader>)> {
    let (stdout, stdout_write) = io::pipe()?;
    // Closed early, it tells the child its app left.
    let (stdin_read, stdin) = io::pipe()?;
    let stderr = match log {
        Some(file) => {
            command.stderr(Stdio::from(file));
            None
        }
        None => {
            let (stderr, stderr_write) = io::pipe()?;
            command.stderr(Stdio::from(stderr_write));
            Some(stderr)
        }
    };
    command.stdin(Stdio::from(stdin_read)).stdout(Stdio::from(stdout_write));
    Ok(((stdout, stdin), stderr))
}

/// Spawns on this thread; `then` gets pid and stdout.
pub(super) fn spawn(
    mut command: Command,
    bin: PathBuf,
    stopper: &Stopper,
    sender: &Sender,
    ends: Ends,
    then: impl FnOnce(u32, Ends),
) {
    let began = Instant::now();
    // Held: a stop waits for the id it kills.
    let gate = stopper.gate();
    let child = command.spawn().map_err(at(&bin));
    // Our copies of the child's ends close here.
    drop(command);
    let pid = child.as_ref().map(Child::id).ok();
    if let Some(pid) = pid {
        stopper.started(pid);
        log::info!("lok: worker process {pid} started in {} ms", began.elapsed().as_millis());
    }
    drop(gate);
    if sender.send(child.map(Started::Spawned)).is_err() {
        log::error!("lok: the worker was dropped before its process started");
    }
    match pid {
        Some(pid) => then(pid, ends),
        None => drop(ends),
    }
}

impl Process {
    /// Spawns off this thread; `then` gets pid and stdout.
    pub(super) fn start(
        mut command: Command,
        bin: PathBuf,
        stopper: &Stopper,
        log: Option<File>,
        then: impl FnOnce(u32, Ends) + Send + 'static,
    ) -> io::Result<(Self, Option<PipeReader>)> {
        let (ends, stderr) = pipes(&mut command, log)?;
        let (sender, started) = mpsc::channel();
        let stopper = stopper.clone();
        std::thread::Builder::new()
            .name("lok-spawn".into())
            .spawn(move || spawn(command, bin, &stopper, &sender, ends, then))?;
        Ok((Self::Starting(started), stderr))
    }

    /// Waits for a start thread's result.
    pub(crate) fn resolve(&mut self) {
        let Self::Starting(started) = self else {
            return;
        };
        *self = match started.recv() {
            Ok(Ok(Started::Spawned(child))) => Self::Up(child),
            Ok(Ok(Started::Adopted(pid, peer, closed))) => Self::Adopted(pid, peer, closed),
            Ok(Err(error)) => Self::Failed(error.kind(), error.to_string()),
            Err(_) => Self::Failed(io::ErrorKind::Other, "the worker's spawn thread ended".into()),
        };
    }

    /// The process, once its spawn has returned.
    pub(crate) fn get(&mut self) -> io::Result<&mut Child> {
        self.resolve();
        match self {
            Self::Up(child) => Ok(child),
            Self::Failed(kind, why) => Err(io::Error::new(*kind, why.clone())),
            Self::Starting(_) => Err(io::Error::other("the worker is still starting")),
            Self::Adopted(pid, ..) => Err(io::Error::other(format!("engine {pid} was taken over, not spawned"))),
        }
    }

    /// Whether it is an idle engine taken over.
    pub(crate) fn adopted(&mut self) -> bool {
        self.resolve();
        matches!(self, Self::Adopted(..))
    }

    /// Why it ended, if it has; reaped under the gate.
    pub(crate) fn ended(&mut self, stopper: &Stopper) -> Option<String> {
        self.resolve();
        if let Self::Adopted(pid, _, closed) = self {
            return closed.load(Ordering::SeqCst).then(|| format!("engine {pid} closed its connection"));
        }
        // The spawn takes the gate; wait for it outside.
        let child = match self.get() {
            Ok(child) => child,
            Err(error) => return Some(error.to_string()),
        };
        let mut reaped = stopper.gate();
        match child.try_wait() {
            Ok(Some(status)) => {
                *reaped = true;
                Some(status.to_string())
            }
            Ok(None) => None,
            Err(error) => Some(error.to_string()),
        }
    }

    /// Kills and reaps it; how it ended, or why not.
    pub(crate) fn end(&mut self) -> String {
        self.resolve();
        if let Self::Adopted(pid, peer, _) = self {
            return match peer.end() {
                Ok(()) => format!("engine {pid} ended"),
                Err(error) => format!("engine {pid} not ended: {error}"),
            };
        }
        match self.get() {
            Ok(child) => {
                let _ = child.kill();
                child
                    .wait()
                    .map(|status| status.to_string())
                    .unwrap_or_else(|wait| wait.to_string())
            }
            Err(error) => error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;

    use super::*;

    #[test]
    fn a_spawn_that_fails_says_so_when_first_needed_and_closes_its_pipes() {
        let bin = std::env::temp_dir().join("ely-no-such-worker");
        let stopper = Stopper::new();
        let (mut process, stderr) =
            Process::start(Command::new(&bin), bin.clone(), &stopper, None, |_, _| panic!("no process, no stdout"))
                .expect("starting returns at once");
        let error = process.get().expect_err("no such program");
        assert_eq!(error.kind(), io::ErrorKind::NotFound, "{error}");
        assert!(error.to_string().contains("ely-no-such-worker"), "{error}");
        let mut stderr = stderr.expect("no log, so a pipe");
        assert_eq!(stderr.read(&mut [0; 8]).expect("reads"), 0, "the error pipe ends");
        stopper.stop().expect("nothing to stop");
    }

    #[test]
    fn asking_whether_it_ended_lets_a_waiting_spawn_take_the_gate() {
        let (spawned, started) = mpsc::channel();
        let (stopper, mut process) = (Stopper::new(), Process::Starting(started));
        let (asking, asked) = mpsc::channel();
        let (answer, answered) = mpsc::channel();
        let gate = stopper.clone();
        std::thread::spawn(move || {
            asking.send(()).expect("the test waits");
            answer.send(process.ended(&gate)).expect("the test waits");
        });
        asked.recv().expect("the question is asked");
        // As the spawn thread does: the gate, then the result.
        std::thread::spawn(move || {
            let _held = stopper.gate();
            spawned.send(Err(io::Error::other("no program"))).expect("the question waits");
        });
        let ended = answered.recv_timeout(std::time::Duration::from_secs(5)).expect("no deadlock with the spawn");
        assert_eq!(ended.as_deref(), Some("no program"));
    }
}
