//! The worker process, started off the caller's thread.

use std::{
    io::{self, PipeReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Instant,
};

use super::{Stopper, at};

/// The worker process, or the spawn still starting it.
pub(crate) enum Process {
    Starting(mpsc::Receiver<io::Result<Child>>),
    Up(Child),
    Failed(io::ErrorKind, String),
}

impl Process {
    /// Spawns; `then` gets pid and stdout.
    pub(super) fn start(
        mut command: Command,
        bin: PathBuf,
        stopper: &Stopper,
        then: impl FnOnce(u32, PipeReader) + Send + 'static,
    ) -> io::Result<(Self, PipeReader)> {
        let (stdout, stdout_write) = io::pipe()?;
        let (stderr, stderr_write) = io::pipe()?;
        command
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout_write))
            .stderr(Stdio::from(stderr_write));
        let (sender, started) = mpsc::channel();
        let stopper = stopper.clone();
        std::thread::Builder::new()
            .name("lok-spawn".into())
            .spawn(move || {
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
                if sender.send(child).is_err() {
                    log::error!("lok: the worker was dropped before its process started");
                }
                match pid {
                    Some(pid) => then(pid, stdout),
                    None => drop(stdout),
                }
            })?;
        Ok((Self::Starting(started), stderr))
    }

    /// The process, once its spawn has returned.
    pub(crate) fn get(&mut self) -> io::Result<&mut Child> {
        if let Self::Starting(started) = self {
            *self = match started.recv() {
                Ok(Ok(child)) => Self::Up(child),
                Ok(Err(error)) => Self::Failed(error.kind(), error.to_string()),
                Err(_) => Self::Failed(
                    io::ErrorKind::Other,
                    "the worker's spawn thread ended".into(),
                ),
            };
        }
        match self {
            Self::Up(child) => Ok(child),
            Self::Failed(kind, why) => Err(io::Error::new(*kind, why.clone())),
            Self::Starting(_) => Err(io::Error::other("the worker is still starting")),
        }
    }

    /// Why it ended, if it has; reaped under the gate.
    pub(crate) fn ended(&mut self, stopper: &Stopper) -> Option<String> {
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
        let (mut process, mut stderr) =
            Process::start(Command::new(&bin), bin.clone(), &stopper, |_, _| panic!("no process, no stdout"))
                .expect("starting returns at once");
        let error = process.get().expect_err("no such program");
        assert_eq!(error.kind(), io::ErrorKind::NotFound, "{error}");
        assert!(error.to_string().contains("ely-no-such-worker"), "{error}");
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
