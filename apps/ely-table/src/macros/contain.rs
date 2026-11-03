//! R and every process it starts end together.

use std::{
    io,
    process::{Child, Command, ExitStatus},
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

    /// Whether R exited; Unix leaves it unreaped, holding its group.
    pub fn exited(&mut self) -> io::Result<bool> {
        #[cfg(unix)]
        {
            let pid = libc::id_t::from(self.child.id());
            // SAFETY: zero is a valid record; WNOWAIT reaps nothing.
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            let flags = libc::WEXITED | libc::WNOHANG | libc::WNOWAIT;
            if unsafe { libc::waitid(libc::P_PID, pid, &mut info, flags) } != 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: waitid filled the record or left it zero.
            Ok(unsafe { info.si_pid() } != 0)
        }
        #[cfg(windows)]
        Ok(self.child.try_wait()?.is_some())
    }

    /// Ends R and every helper left, then reaps R.
    pub fn finish(mut self) -> io::Result<ExitStatus> {
        #[cfg(unix)]
        {
            let pid = libc::pid_t::try_from(self.child.id()).map_err(io::Error::other)?;
            // SAFETY: R is unreaped, so the group is still R's.
            if unsafe { libc::kill(-pid, libc::SIGKILL) } != 0 {
                let error = io::Error::last_os_error();
                // Only zombies left: Linux says ESRCH, macOS EPERM.
                if !matches!(error.raw_os_error(), Some(libc::ESRCH | libc::EPERM)) {
                    return Err(error);
                }
            }
        }
        #[cfg(windows)]
        self.job.end()?;
        self.child.wait()
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

    /// R's helper ends, whether R waits or exits first.
    fn helper_ends(waits: bool) {
        #[cfg(unix)]
        let mut command = {
            let mut command = Command::new("sh");
            let tail = if waits { "wait" } else { "exit 0" };
            command.args(["-c", &format!("sleep 30 & echo $!; {tail}")]);
            command
        };
        #[cfg(windows)]
        let mut command = {
            let mut command = Command::new("cmd");
            // Unquoted, start needs no title argument.
            let line = if waits { "ping -n 30 127.0.0.1 >nul" } else { "start /b ping -n 30 127.0.0.1 >nul" };
            command.args(["/c", line]);
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
        let helper = super::win::child_of(run.child.id(), "PING.EXE", Duration::from_secs(60)).expect("ping started");
        assert!(alive(helper), "the helper runs");
        if !waits {
            let deadline = Instant::now() + Duration::from_secs(60);
            while !run.exited().expect("polled") {
                assert!(Instant::now() < deadline, "R never exited");
                std::thread::sleep(Duration::from_millis(20));
            }
            assert!(alive(helper), "the helper outlives R's exit");
        }
        run.finish().expect("finished");
        let deadline = Instant::now() + Duration::from_secs(5);
        while alive(helper) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!alive(helper), "the helper ended with R");
    }

    #[test]
    fn a_stop_reaches_the_helpers_r_started() {
        helper_ends(true);
    }

    #[test]
    fn helpers_end_when_r_exits_first() {
        helper_ends(false);
    }

    #[test]
    fn r_alone_exits_with_its_own_status() {
        for code in [0, 3] {
            #[cfg(unix)]
            let mut command = Command::new("sh");
            #[cfg(unix)]
            command.args(["-c", &format!("exit {code}")]);
            #[cfg(windows)]
            let mut command = Command::new("cmd");
            #[cfg(windows)]
            command.args(["/c", &format!("exit {code}")]);
            let mut run = Run::start(command.stdin(Stdio::null())).expect("started");
            let deadline = Instant::now() + Duration::from_secs(60);
            while !run.exited().expect("polled") {
                assert!(Instant::now() < deadline, "R never exited");
                std::thread::sleep(Duration::from_millis(20));
            }
            assert_eq!(run.finish().expect("finished").code(), Some(code));
        }
    }
}
