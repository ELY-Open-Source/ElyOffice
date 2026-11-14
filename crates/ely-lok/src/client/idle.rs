//! An idle engine: started ahead, checked, taken by an app.

use std::{
    fs::File,
    io,
    path::Path,
    sync::{
        Arc, mpsc,
        atomic::{AtomicBool, Ordering},
    },
};

use super::process::{self, Started};
use super::{Feed, Process, SESSIONS, Stopper, Wires, Worker, command};
use crate::endpoint::{self, Half, Peer, Start};
use crate::ipc::tag;

/// An idle engine's halves, pid and peer.
type Found = io::Result<(Half, Half, u32, Peer)>;

/// None waits, it refused, or another holds it.
pub fn no_idle_engine(error: &io::Error) -> bool {
    use io::ErrorKind::{ConnectionRefused, ConnectionReset, NotFound, ResourceBusy};
    matches!(error.kind(), NotFound | ConnectionRefused | ConnectionReset | ResourceBusy)
}

impl Worker {
    /// Starts `app`'s idle engine in `start`'s profile and checks it.
    pub fn idle(start: &Start, install: &Path, probe: crate::Probe, app: &str, log: File) -> io::Result<Self> {
        let name = endpoint::idle_name(app);
        let mut worker = Self::spawn_mode(install, start.dir(), false, Some((&name, log)))?;
        worker.probe = probe;
        worker.ready()?;
        Ok(worker)
    }

    /// Leaves the checked engine waiting; its pid.
    pub fn release(mut self) -> io::Result<u32> {
        self.request(tag::RELEASE, &[])?;
        self.down = true;
        let pid = self.child.get()?.id();
        let write = self.requests.lock().map_err(|_| io::Error::other("a frame write panicked"))?.take();
        let Some(write) = write else {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "the engine was never connected"));
        };
        endpoint::hang_up(write.into_inner().map_err(io::IntoInnerError::into_error)?)?;
        log::info!("lok: engine {pid} checked and released");
        Ok(pid)
    }

    /// Takes over `app`'s idle engine now, or fails.
    pub fn adopt(install: &Path, profile_dir: &Path, app: &str, probe: crate::Probe) -> io::Result<Self> {
        let found = endpoint::adopt(&endpoint::idle_name(app), install, profile_dir)?;
        Self::begin(install, profile_dir, app, probe, move |_, _, _| Ok(found))
    }

    /// Takes `app`'s idle engine, else starts one, off this thread.
    pub fn take(install: &Path, profile_dir: &Path, app: &str, probe: crate::Probe) -> io::Result<Self> {
        Self::begin(install, profile_dir, app, probe, endpoint::adopt)
    }

    fn begin(
        install: &Path,
        profile_dir: &Path,
        app: &str,
        probe: crate::Probe,
        find: impl FnOnce(&str, &Path, &Path) -> Found + Send + 'static,
    ) -> io::Result<Self> {
        let session = SESSIONS.fetch_add(1, Ordering::SeqCst);
        let private = endpoint::name(session)?;
        let start = (install.to_path_buf(), profile_dir.to_path_buf(), false);
        let (mut command, bin) = command::build(&start, &private, None)?;
        let (ends, stderr) = process::pipes(&mut command, None)?;
        let (feed, (sender, replies)) = (Feed::default(), mpsc::channel());
        let ((wired, wiring), (started, made)) = (mpsc::channel(), mpsc::channel());
        let (stopper, idle, kernel) = (Stopper::new(), endpoint::idle_name(app), install.to_path_buf());
        let (gate, reading, temp) = (stopper.clone(), feed.clone(), profile_dir.join("tmp").join(&private));
        let profile = profile_dir.to_path_buf();
        let take = move || match find(&idle, &kernel, &profile) {
            Ok((read, write, pid, peer)) => {
                // Our stderr end closes; the engine logs to its file.
                drop((command, ends));
                if let Err(error) = std::fs::remove_dir(&temp) {
                    log::warn!("lok: unused temp {} stays: {error}", temp.display());
                }
                let (closed, peer) = (Arc::new(AtomicBool::new(false)), Arc::new(peer));
                gate.adopted(Arc::clone(&peer));
                log::info!("lok: took over engine {pid} on {idle}");
                if started.send(Ok(Started::Adopted(pid, peer, Arc::clone(&closed)))).is_err() {
                    return;
                }
                command::serve(read, write, &reading, &sender, &wired);
                closed.store(true, Ordering::SeqCst);
            }
            Err(error) if no_idle_engine(&error) => {
                log::info!("lok: no idle engine on {idle} ({error}); starting one");
                let then = command::reach_then(private, kernel, reading, sender, wired);
                process::spawn(command, bin, &gate, &started, ends, then);
            }
            Err(error) => {
                log::error!("lok: the engine on {idle} is refused: {error}");
                let _ = started.send(Err(io::Error::new(error.kind(), error.to_string())));
                command::refuse(error, &reading, &sender, &wired);
            }
        };
        std::thread::Builder::new().name("lok-take".into()).spawn(take)?;
        let wires = Wires { wiring, replies, feed };
        let mut worker = Self::made(Process::Starting(made), stopper, wires, session, stderr, start);
        worker.probe = probe;
        Ok(worker)
    }
}
