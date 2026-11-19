//! Benchmarks' stall watch: busy main thread, or starved process.

use std::{
    fmt, io,
    time::{Duration, Instant},
};

use ely_platform::cpu::thread_time;
use gpui::App;

/// Lateness, and CPU run, past this count.
const STALL: Duration = Duration::from_millis(100);
const BEAT: Duration = Duration::from_millis(20);

/// A timer that came over 100 ms late.
#[derive(Debug, PartialEq)]
pub(crate) enum Late {
    /// The thread ran through it.
    Busy { late: Duration, ran: Duration },
    /// The thread barely ran: the process went without CPU.
    Starved { late: Duration, ran: Duration },
}

impl fmt::Display for Late {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Late::Busy { late, ran } => {
                write!(out, "the main thread was busy {} ms ({} ms late)", ran.as_millis(), late.as_millis())
            }
            Late::Starved { late, ran } => write!(
                out,
                "a timer came {} ms late, the main thread ran {} ms: starved",
                late.as_millis(),
                ran.as_millis()
            ),
        }
    }
}

/// A wait on this thread: start time, CPU so far.
pub(crate) struct Beat {
    asked: Instant,
    ran: Duration,
}

impl Beat {
    pub(crate) fn start() -> io::Result<Self> {
        Ok(Beat { asked: Instant::now(), ran: thread_time()? })
    }

    /// A late wait for `wanted`, judged by this thread's CPU.
    pub(crate) fn end(self, wanted: Duration) -> io::Result<Option<Late>> {
        let late = self.asked.elapsed().saturating_sub(wanted);
        if late <= STALL {
            return Ok(None);
        }
        let ran = thread_time()?.saturating_sub(self.ran);
        Ok(Some(match ran > STALL {
            true => Late::Busy { late, ran },
            false => Late::Starved { late, ran },
        }))
    }
}

/// Benchmarks only: idle apps do nothing (PERF-02).
pub(crate) fn watch(cx: &mut App) {
    cx.spawn(async move |cx: &mut gpui::AsyncApp| {
        loop {
            let beat = Beat::start();
            cx.background_executor().timer(BEAT).await;
            match beat.and_then(|beat| beat.end(BEAT)) {
                Ok(Some(late)) => log::warn!("ui: {late}"),
                Ok(None) => {}
                Err(error) => {
                    log::error!("ui: the stall watch stops, no thread CPU time: {error}");
                    return;
                }
            }
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ely_platform::cpu::thread_time;

    use super::{Beat, Late};

    /// Runs `span` of this thread's CPU; other tests share it.
    fn busy(span: Duration) {
        let before = thread_time().expect("a thread time");
        while thread_time().expect("a thread time") - before < span {
            std::hint::black_box(0u64);
        }
    }

    #[test]
    fn a_busy_loop_is_busy_and_an_on_time_wait_is_nothing() {
        let beat = Beat::start().expect("a beat");
        busy(Duration::from_millis(300));
        let late = beat.end(Duration::from_millis(20)).expect("a thread time");
        assert!(matches!(late, Some(Late::Busy { ran, .. }) if ran >= Duration::from_millis(300)), "{late:?}");
        assert!(late.expect("late").to_string().starts_with("the main thread was busy "));
        let beat = Beat::start().expect("a beat");
        // The wait itself, on time.
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(beat.end(Duration::from_millis(20)).expect("a thread time"), None);
    }

    /// Run by the stop test in its own process.
    #[test]
    #[ignore]
    fn a_child_waits_while_stopped() {
        if std::env::var_os("ELY_STALL_CHILD").is_none() {
            return;
        }
        let beat = Beat::start().expect("a beat");
        println!("ready");
        // The wait the parent stops in.
        std::thread::sleep(Duration::from_millis(300));
        println!("judged {:?}", beat.end(Duration::from_millis(300)).expect("a thread time"));
    }

    #[cfg(unix)]
    #[test]
    fn a_stopped_process_is_starved_not_busy() {
        use std::io::{BufRead, BufReader};
        use std::process::{Command, Stdio};

        let mut child = Command::new(std::env::current_exe().expect("this test binary"))
            .args(["--exact", "stalls::tests::a_child_waits_while_stopped", "--ignored", "--nocapture"])
            .env("ELY_STALL_CHILD", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the child starts");
        let pid = child.id().to_string();
        let mut lines = BufReader::new(child.stdout.take().expect("its output")).lines();
        while lines.next().expect("a line").expect("text") != "ready" {}
        let signal = |name: &str| {
            let status = Command::new("kill").args([name, &pid]).status().expect("kill runs");
            assert!(status.success(), "kill {name} {pid}");
        };
        signal("-STOP");
        // Stopped this long: its 300 ms wait comes late.
        std::thread::sleep(Duration::from_millis(600));
        signal("-CONT");
        // Read to the end: a closed pipe fails the child.
        let rest: Vec<String> = lines.map(|line| line.expect("text")).collect();
        let judged = rest.into_iter().find(|line| line.starts_with("judged "));
        let mut said = String::new();
        std::io::Read::read_to_string(&mut child.stderr.take().expect("its errors"), &mut said).expect("reads");
        let ended = child.wait().expect("the child ends");
        assert!(ended.success(), "the child {ended}: {said}");
        let judged = judged.expect("the child judged its wait");
        assert!(judged.contains("Starved"), "{judged}");
    }
}
