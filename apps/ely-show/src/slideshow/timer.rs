//! Countdowns on slides; ElyShow keeps their time.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use gpui::{App, Context, Entity, Task};

use super::{Show, presenter::clock};

pub(crate) mod dialog;
mod drag;
mod draw;

pub use drag::Drag;
pub use draw::{ASIDE, Act, BAND, Hand, Room, face};

/// How often a running countdown redraws.
const TICK: Duration = Duration::from_millis(200);

/// Where a countdown sits on its slide.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Spot {
    #[default]
    Top,
    Center,
    Bottom,
    Corner,
}

/// A slide's countdown; one id means one shared clock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timer {
    pub id: u64,
    pub length: Duration,
    pub spot: Spot,
    /// Where it was dragged: its middle, as slide shares.
    pub at: Option<(f32, f32)>,
    /// Beside scores, it has the focus and they shrink.
    pub ahead: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Ready,
    Running,
    Paused,
    Over,
}

impl Phase {
    /// The words for the button that starts or pauses.
    pub fn run_key(self) -> &'static str {
        match self {
            Phase::Running => "show.timer.pause",
            Phase::Ready | Phase::Paused | Phase::Over => "show.timer.start",
        }
    }
}

/// What a countdown shows now.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub text: String,
    pub phase: Phase,
    pub spot: Spot,
    pub at: Option<(f32, f32)>,
    pub ahead: bool,
    /// In its last seconds.
    pub ending: bool,
}

impl Face {
    /// The words for the button that swaps the focus.
    pub fn focus_key(&self) -> &'static str {
        match self.ahead {
            true => "show.focus.scores",
            false => "show.focus.timer",
        }
    }
}

/// The seconds a running countdown warns for.
const WARN: u64 = 10;

#[derive(Clone, Copy)]
struct Run {
    length: Duration,
    spent: Duration,
    since: Option<Instant>,
}

impl Run {
    fn spent(&self, now: Instant) -> Duration {
        self.spent + self.since.map_or(Duration::ZERO, |since| now.saturating_duration_since(since))
    }
}

impl Timer {
    fn face(&self, run: Option<&Run>, now: Instant) -> Face {
        let spent = run.map_or(Duration::ZERO, |run| run.spent(now));
        let left = self.length.saturating_sub(spent);
        let phase = match run {
            _ if left.is_zero() => Phase::Over,
            None => Phase::Ready,
            Some(run) if run.since.is_some() => Phase::Running,
            Some(_) => Phase::Paused,
        };
        // A started second counts until it ends.
        let seconds = left.as_secs() + u64::from(left.subsec_nanos() > 0);
        let ending = phase != Phase::Ready && seconds <= WARN;
        Face { text: clock(seconds), phase, spot: self.spot, at: self.at, ahead: self.ahead, ending }
    }
}

/// A deck's running countdowns, shared by editor and show.
#[derive(Default)]
pub struct Clocks {
    runs: HashMap<u64, Run>,
    /// The id the next countdown takes.
    next: u64,
    tick: Option<Task<()>>,
}

impl Clocks {
    /// An id no earlier countdown had.
    pub fn fresh(&mut self) -> u64 {
        self.next += 1;
        self.next
    }

    /// Ids up to `last` are already some countdown's.
    pub fn taken(&mut self, last: u64) {
        self.next = self.next.max(last);
    }

    /// What `timer` shows at `now`.
    pub fn face(&self, timer: &Timer, now: Instant) -> Face {
        timer.face(self.runs.get(&timer.id), now)
    }

    /// Starts or pauses `timer`; a done one restarts.
    pub fn toggle(&mut self, timer: Timer, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        let idle = Run { length: timer.length, spent: Duration::ZERO, since: None };
        let run = self.runs.entry(timer.id).or_insert(idle);
        if run.spent(now) >= run.length {
            *run = idle;
        }
        match run.since.take() {
            Some(since) => run.spent += now.saturating_duration_since(since),
            None => run.since = Some(now),
        }
        log::info!("show: countdown {} runs: {}", timer.id, run.since.is_some());
        self.keep_ticking(cx);
        cx.notify();
    }

    /// Sets countdown `id` back, stopped.
    pub fn reset(&mut self, id: u64, cx: &mut Context<Self>) {
        self.runs.remove(&id);
        log::info!("show: countdown {id} is reset");
        cx.notify();
    }

    /// Whether any countdown still runs at `now`.
    fn counting(&self, now: Instant) -> bool {
        self.runs.values().any(|run| run.since.is_some() && run.spent(now) < run.length)
    }

    /// Redraws while a countdown runs.
    fn keep_ticking(&mut self, cx: &mut Context<Self>) {
        self.tick = Some(cx.spawn(async |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                let counting = this.update(cx, |clocks, cx| {
                    cx.notify();
                    clocks.counting(cx.background_executor().now())
                });
                if !counting.is_ok_and(|counting| counting) {
                    break;
                }
            }
        }));
    }
}

/// A show's countdowns.
pub struct Timed {
    /// Each slide's countdown, by place in the deck.
    slides: Vec<Option<Timer>>,
    clocks: Entity<Clocks>,
}

impl Show {
    pub fn time_slides(&mut self, slides: Vec<Option<Timer>>, clocks: Entity<Clocks>, cx: &mut Context<Self>) {
        cx.observe(&clocks, |_, _, cx| cx.notify()).detach();
        self.timers = Some(Timed { slides, clocks });
    }

    /// Slide `part`'s countdown, and the clock it runs on.
    pub fn timer(&self, part: usize) -> Option<(Timer, &Entity<Clocks>)> {
        let timed = self.timers.as_ref()?;
        let timer = timed.slides.get(part).copied().flatten()?;
        Some((timer, &timed.clocks))
    }

    /// What slide `part`'s countdown shows now.
    pub fn countdown(&self, part: usize, cx: &App) -> Option<Face> {
        let (timer, clocks) = self.timer(part)?;
        Some(clocks.read(cx).face(&timer, cx.background_executor().now()))
    }

    /// Gives the shown slide's focus to the other one.
    pub fn swap_focus(&mut self, cx: &mut Context<Self>) {
        let Some(part) = self.part().filter(|part| self.board.as_ref().is_some_and(|board| board.linked(*part))) else {
            return;
        };
        let timer = self.timers.as_mut().and_then(|timed| timed.slides.get_mut(part)).and_then(Option::as_mut);
        if let Some(timer) = timer {
            timer.ahead = !timer.ahead;
            log::info!("show: the countdown has the focus: {}", timer.ahead);
            cx.notify();
        }
    }

    /// Starts or pauses the shown countdown.
    pub fn toggle_timer(&mut self, cx: &mut Context<Self>) {
        if let Some((timer, clocks)) = self.part().and_then(|part| self.timer(part)) {
            clocks.update(cx, |clocks, cx| clocks.toggle(timer, cx));
        }
    }

    /// Sets the shown countdown back, stopped.
    pub fn reset_timer(&mut self, cx: &mut Context<Self>) {
        if let Some((timer, clocks)) = self.part().and_then(|part| self.timer(part)) {
            clocks.update(cx, |clocks, cx| clocks.reset(timer.id, cx));
        }
    }
}

/// `5` is minutes, `1:30` minutes and seconds.
pub fn length(typed: &str) -> Option<Duration> {
    let (minutes, seconds) = match typed.trim().split_once(':') {
        Some((minutes, seconds)) if seconds.len() == 2 => (minutes, seconds),
        Some(_) => return None,
        None => (typed.trim(), "0"),
    };
    let figures = |part: &str| part.chars().all(|c| c.is_ascii_digit()).then(|| part.parse::<u64>().ok()).flatten();
    let (minutes, seconds) = (figures(minutes)?, figures(seconds)?);
    let total = minutes.checked_mul(60)?.checked_add(seconds)?;
    (seconds < 60 && (1..=MOST).contains(&total)).then(|| Duration::from_secs(total))
}

/// The longest countdown: a day.
const MOST: u64 = 24 * 60 * 60;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_lengths_are_minutes_or_minutes_and_seconds() {
        let seconds = |typed: &str| length(typed).map(|length| length.as_secs());
        assert_eq!(seconds("5"), Some(300));
        assert_eq!(seconds(" 1:30 "), Some(90));
        assert_eq!(seconds("0:45"), Some(45));
        for bad in ["", "0", "0:00", "1:5", "1:60", "-1", "+2", "a", "1:2:3", "100000"] {
            assert_eq!(seconds(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_countdown_rounds_up_warns_and_ends() {
        let timer = Timer { id: 1, length: Duration::from_secs(70), spot: Spot::Top, at: None, ahead: false };
        let start = Instant::now();
        let run = Run { length: timer.length, spent: Duration::ZERO, since: Some(start) };
        let at = |millis| timer.face(Some(&run), start + Duration::from_millis(millis));
        let ready = timer.face(None, start);
        assert_eq!((ready.text.as_str(), ready.phase), ("1:10", Phase::Ready));
        assert_eq!((at(100).text.as_str(), at(100).ending), ("1:10", false));
        assert_eq!((at(60_500).text.as_str(), at(60_500).ending), ("0:10", true));
        assert_eq!((at(90_000).text.as_str(), at(90_000).phase), ("0:00", Phase::Over));
        let paused = Run { length: timer.length, spent: Duration::from_secs(5), since: None };
        assert_eq!(timer.face(Some(&paused), start).phase, Phase::Paused);
    }
}
