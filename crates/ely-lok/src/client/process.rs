//! The worker process, started off the caller's thread.

use std::{
    io::{self, PipeReader, PipeWriter},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Instant,
};

use super::{Stopper, at};

/// The ends a worker talks through, made before its process.
pub(super) struct Ends {
    pub stdin: PipeWriter,
    pub stdout: PipeReader,
    pub stderr: PipeReader,
}

/// The worker process, or the spawn still starting it.
pub(crate) enum Process {
    Starting(mpsc::Receiver<io::Result<Child>>),
    Up(Child),
    Failed(io::ErrorKind, String),
}

impl Process {
    /// Spawns `command` off the caller's thread: cold spawns take seconds.
    pub(super) fn start(
        mut command: Command,
        bin: PathBuf,
        stopper: &Stopper,
    ) -> io::Result<(Self, Ends)> {
        let (stdin_read, stdin) = io::pipe()?;
        let (stdout, stdout_write) = io::pipe()?;
        let (stderr, stderr_write) = io::pipe()?;
        command
            .stdin(Stdio::from(stdin_read))
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
                if let Ok(child) = &child {
                    stopper.started(child.id());
                    log::info!(
                        "lok: worker process {} started in {} ms",
                        child.id(),
                        began.elapsed().as_millis()
                    );
                }
                drop(gate);
                if sender.send(child).is_err() {
                    log::error!("lok: the worker was dropped before its process started");
                }
            })?;
        Ok((
            Self::Starting(started),
            Ends {
                stdin,
                stdout,
                stderr,
            },
        ))
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
        let (mut process, mut ends) =
            Process::start(Command::new(&bin), bin.clone(), &stopper).expect("starting returns at once");
        let error = process.get().expect_err("no such program");
        assert_eq!(error.kind(), io::ErrorKind::NotFound, "{error}");
        assert!(error.to_string().contains("ely-no-such-worker"), "{error}");
        assert_eq!(ends.stdout.read(&mut [0; 8]).expect("reads"), 0, "the output pipe ends");
        stopper.stop().expect("nothing to stop");
    }
}
