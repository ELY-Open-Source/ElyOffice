//! A long Git run: progress, and a stop.

use std::{
    io::{self, Read as _},
    path::Path,
    process::{Child, ChildStderr, ChildStdout, Stdio},
    sync::{Arc, Mutex},
};

use crate::git::Git;

#[cfg(windows)]
mod win;

/// Git running; progress comes line by line.
pub struct Running {
    verb: String,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    child: Arc<Mutex<Child>>,
}

/// Stops a run from another thread.
#[derive(Clone)]
pub struct Stop(Arc<Mutex<Child>>);

impl Git {
    /// Its own session: SSH finds no terminal.
    pub fn start(&self, dir: &Path, args: &[&str]) -> io::Result<Running> {
        let mut command = self.command(dir, args);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            // SAFETY: setsid is async-signal-safe.
            unsafe {
                command.pre_exec(|| match libc::setsid() {
                    -1 => Err(io::Error::last_os_error()),
                    _ => Ok(()),
                });
            }
        }
        // Git started after this hears Ctrl+C.
        #[cfg(windows)]
        win::hear()?;
        let mut child = command.spawn()?;
        let verb = args.first().copied().unwrap_or("").to_string();
        log::info!("tree: git {verb} started, {}", child.id());
        Ok(Running {
            verb,
            stdout: child.stdout.take(),
            stderr: child.stderr.take(),
            child: Arc::new(Mutex::new(child)),
        })
    }
}

/// Lines as shown; one ended by CR is rewritten.
#[derive(Default)]
struct Said {
    lines: Vec<String>,
    rewrite: bool,
}

impl Said {
    /// Ends a line at `byte`; empty CRLF halves end nothing.
    fn end(&mut self, text: String, byte: u8) -> Option<&str> {
        if text.is_empty() {
            self.rewrite &= byte == b'\r';
            return None;
        }
        if self.rewrite {
            self.lines.pop();
        }
        self.rewrite = byte == b'\r';
        self.lines.push(text);
        self.lines.last().map(String::as_str)
    }
}

impl Running {
    pub fn stop(&self) -> Stop {
        Stop(Arc::clone(&self.child))
    }

    /// Reads to the end; failure quotes Git, both streams.
    pub fn follow(mut self, mut seen: impl FnMut(&str)) -> io::Result<String> {
        let mut stdout = self
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("no output"))?;
        let mut stderr = self
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("no progress"))?;
        // Reading apart keeps either pipe from filling.
        let out = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).map(|_| bytes)
        });
        let (mut said, mut line, mut buffer) = (Said::default(), Vec::new(), [0u8; 4096]);
        let mut flush = |line: &mut Vec<u8>, said: &mut Said, byte: u8| {
            let text = String::from_utf8_lossy(line).trim().to_string();
            line.clear();
            if let Some(text) = said.end(text, byte) {
                seen(text);
            }
        };
        loop {
            let read = stderr.read(&mut buffer)?;
            // Progress rewrites its line with carriage returns.
            for &byte in &buffer[..read] {
                match byte {
                    b'\r' | b'\n' => flush(&mut line, &mut said, byte),
                    _ => line.push(byte),
                }
            }
            if read == 0 {
                flush(&mut line, &mut said, b'\n');
                break;
            }
        }
        let out = out
            .join()
            .map_err(|_| io::Error::other("the output reader panicked"))??;
        let out = String::from_utf8_lossy(&out).trim().to_string();
        let status = self.child.lock().expect("the run lock").wait()?;
        let verb = &self.verb;
        if !status.success() {
            log::warn!("tree: git {verb} failed, {status}");
            let all: Vec<String> = [out]
                .into_iter()
                .chain(said.lines)
                .filter(|part| !part.is_empty())
                .collect();
            return Err(io::Error::other(format!(
                "git {verb} ({status}):\n{}",
                all.join("\n")
            )));
        }
        log::info!("tree: git {verb} done");
        Ok(out)
    }
}

impl Stop {
    /// Asks Git and its helpers to stop; Git cleans up.
    pub fn now(&self) -> io::Result<()> {
        let mut child = self.0.lock().expect("the run lock");
        // An ended Git's number may be another's now.
        if let Some(status) = child.try_wait()? {
            log::info!("tree: git {} had ended, {status}", child.id());
            return Ok(());
        }
        log::info!("tree: git {} asked to stop", child.id());
        terminate(child.id())
    }

    /// A stop over any child, for tests.
    #[cfg(all(test, unix))]
    pub fn over(child: Child) -> Stop {
        Stop(Arc::new(Mutex::new(child)))
    }
}

/// Its own session: the group is Git and its helpers.
#[cfg(unix)]
fn terminate(pid: u32) -> io::Result<()> {
    let pid = libc::pid_t::try_from(pid).map_err(io::Error::other)?;
    match unsafe { libc::kill(-pid, libc::SIGTERM) } {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error()),
    }
}

/// Ctrl+C to Git and its helpers, as Git for Windows.
#[cfg(windows)]
fn terminate(pid: u32) -> io::Result<()> {
    win::stop(pid)
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_a_line_ended_by_return_is_rewritten() {
        let mut said = super::Said::default();
        for (text, byte) in [
            ("Receiving objects: 1%", b'\r'),
            ("Receiving objects: 100%", b'\n'),
            ("remote: POLICY_A required", b'\r'),
            ("", b'\n'),
            ("remote: POLICY_B required", b'\n'),
        ] {
            said.end(text.to_string(), byte);
        }
        assert_eq!(
            said.lines,
            [
                "Receiving objects: 100%",
                "remote: POLICY_A required",
                "remote: POLICY_B required"
            ]
        );
    }
}
