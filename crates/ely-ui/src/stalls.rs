//! Benchmarks' stall watch: busy main thread, or starved process.

use std::{
    fmt, io,
    time::{Duration, Instant},
};

use ely_platform::cpu::{thread_time, thread_waited};
use gpui::App;

/// Lateness, and CPU run, past this count.
const STALL: Duration = Duration::from_millis(100);
const BEAT: Duration = Duration::from_millis(20);

/// A timer that came over 100 ms late.
#[derive(Debug, PartialEq)]
pub(crate) enum Late {
    /// The thread ran through it.
    Busy { late: Duration, ran: Duration },
    /// Neither ran nor waited for a CPU: waited on something.
    Blocked { late: Duration, ran: Duration, waited: Duration },
    /// Mostly runnable, off the CPU: the machine, not the app.
    Starved { late: Duration, ran: Duration, waited: Duration },
    /// Off the CPU; macOS and Windows keep no run delay.
    Held { late: Duration, ran: Duration },
}

impl fmt::Display for Late {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ms = |span: &Duration| span.as_millis();
        match self {
            Late::Busy { late, ran } => write!(out, "the main thread was busy {} ms ({} ms late)", ms(ran), ms(late)),
            Late::Blocked { late, ran, waited } => write!(
                out,
                "the main thread was blocked {} ms (ran {} ms, waited for a CPU {} ms)",
                ms(late),
                ms(ran),
                ms(waited)
            ),
            Late::Starved { late, ran, waited } => write!(
                out,
                "a timer came {} ms late, the main thread ran {} ms, waited for a CPU {} ms: starved",
                ms(late),
                ms(ran),
                ms(waited)
            ),
            Late::Held { late, ran } => write!(
                out,
                "the main thread was held {} ms (ran {} ms; this system keeps no run delay)",
                ms(late),
                ms(ran)
            ),
        }
    }
}

/// A wait on this thread: start time, CPU so far.
pub(crate) struct Beat {
    asked: Instant,
    ran: Duration,
    waited: Option<Duration>,
}

impl Beat {
    pub(crate) fn start() -> io::Result<Self> {
        Ok(Beat { asked: Instant::now(), ran: thread_time()?, waited: thread_waited()? })
    }

    /// A late wait for `wanted`, judged by this thread's CPU.
    pub(crate) fn end(self, wanted: Duration) -> io::Result<Option<Late>> {
        let late = self.asked.elapsed().saturating_sub(wanted);
        if late <= STALL {
            return Ok(None);
        }
        let ran = thread_time()?.saturating_sub(self.ran);
        if ran > STALL {
            return Ok(Some(Late::Busy { late, ran }));
        }
        let waited = match (self.waited, thread_waited()?) {
            (Some(before), Some(now)) => now.saturating_sub(before),
            _ => return Ok(Some(Late::Held { late, ran })),
        };
        // Starved only when run delay covers most of it.
        Ok(Some(match waited * 2 >= late {
            true => Late::Starved { late, ran, waited },
            false => Late::Blocked { late, ran, waited },
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
mod tests;
