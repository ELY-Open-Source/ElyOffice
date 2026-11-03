//! R and every process it starts end together.

use std::{
    io,
    process::{Child, Command},
};

#[cfg(windows)]
mod win;

/// Rscript in its own process group or job.
pub struct Run {
    pub child: Child,
    #[cfg(windows)]
    job: win::Job,
}

impl Run {
    /// Starts `command`, opening no console window.
    pub fn start(command: &mut Command) -> io::Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            command.process_group(0);
            Ok(Self { child: command.spawn()? })
        }
        #[cfg(windows)]
        {
            let (child, job) = win::start(command)?;
            Ok(Self { child, job })
        }
    }

    /// Ends R and its helpers, then reaps R.
    pub fn end(&mut self) -> io::Result<()> {
        #[cfg(unix)]
        {
            let pid = libc::pid_t::try_from(self.child.id()).map_err(io::Error::other)?;
            // SAFETY: kill only signals; R is not reaped yet.
            if unsafe { libc::kill(-pid, libc::SIGKILL) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        #[cfg(windows)]
        self.job.end()?;
        self.child.wait().map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::Run;
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };

    /// Whether `pid` still runs; an unreaped zombie does not.
    #[cfg(unix)]
    fn alive(pid: u32) -> bool {
        // SAFETY: signal 0 only checks.
        let exists = unsafe { libc::kill(pid as libc::pid_t, 0) == 0 };
        #[cfg(target_os = "linux")]
        let exists = exists
            && std::fs::read_to_string(format!("/proc/{pid}/stat"))
                .is_ok_and(|stat| stat.rsplit_once(") ").is_some_and(|(_, rest)| !rest.starts_with('Z')));
        exists
    }

    #[cfg(windows)]
    use super::win::alive;

    #[test]
    fn an_end_reaches_the_helpers_r_started() {
        #[cfg(unix)]
        let mut command = {
            let mut command = Command::new("sh");
            command.args(["-c", "sleep 30 & echo $!; wait"]);
            command
        };
        #[cfg(windows)]
        let mut command = {
            let mut command = Command::new("cmd");
            command.args(["/c", "ping -n 30 127.0.0.1 >nul"]);
            command
        };
        command.stdin(Stdio::null()).stdout(Stdio::piped());
        let mut run = Run::start(&mut command).expect("started");
        #[cfg(unix)]
        let helper = {
            use std::io::BufRead as _;
            let out = run.child.stdout.take().expect("stdout");
            let mut line = String::new();
            std::io::BufReader::new(out).read_line(&mut line).expect("the helper's pid");
            line.trim().parse::<u32>().expect("a pid")
        };
        #[cfg(windows)]
        let helper = super::win::child_of(run.child.id(), "PING.EXE", Duration::from_secs(10)).expect("ping started");
        assert!(alive(helper), "the helper runs");
        run.end().expect("ended");
        let deadline = Instant::now() + Duration::from_secs(5);
        while alive(helper) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!alive(helper), "the helper ended with R");
    }
}
