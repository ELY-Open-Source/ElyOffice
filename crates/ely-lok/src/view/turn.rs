//! The wheel and Up, Down turn slides (SHOW-01).

use std::time::{Duration, Instant};

use gpui::{Keystroke, Modifiers, ScrollDelta, ScrollEvent, TouchPhase};

use super::LokView;
use super::input::{Input, vcl_key};

/// Trackpad pixels one turn takes.
const SWIPE: f32 = 50.0;
/// A pause this long ends a gesture without phases.
const PAUSE: Duration = Duration::from_millis(200);

/// Lines one wheel notch scrolls on this platform.
pub fn notch() -> f32 {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETWHEELSCROLLLINES, SystemParametersInfoW};
        let mut lines = 3u32;
        // Same setting gpui scales the wheel by.
        unsafe { SystemParametersInfoW(SPI_GETWHEELSCROLLLINES, 0, (&raw mut lines).cast(), 0) };
        lines as f32
    }
    #[cfg(target_os = "macos")]
    {
        1.0
    }
    // gpui's X11 and Wayland scroll three lines a notch.
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        3.0
    }
}

/// The wheel's progress towards a turn.
#[derive(Default)]
pub(super) struct Turning {
    /// This gesture's travel, in turns.
    travel: f32,
    /// This gesture already turned.
    spent: bool,
    /// The platform began a gesture it has not ended.
    held: bool,
    /// Fractional lines in this run: a touchpad, not a wheel.
    smooth: bool,
    last: Option<Instant>,
}

impl Turning {
    /// The turn a wheel event makes; 0 for none.
    pub(super) fn wheel(&mut self, delta: ScrollDelta, phase: TouchPhase, now: Instant, notch: f32) -> i32 {
        let paused = self.last.is_none_or(|last| now.duration_since(last) >= PAUSE);
        self.last = Some(now);
        // X11 lines mix wheels and touchpads; they carry no phase.
        let (travel, phase) = match delta {
            ScrollDelta::Lines(_) if notch <= 0.0 => return 0,
            ScrollDelta::Lines(lines) => (lines.y / notch, None),
            ScrollDelta::Pixels(pixels) => (f32::from(pixels.y) / SWIPE, Some(phase)),
        };
        // Phases bound a gesture; without them, a pause does.
        if phase == Some(TouchPhase::Started) || paused && !self.held {
            (self.travel, self.spent, self.smooth) = (0.0, false, false);
        }
        let whole = travel.round();
        if phase.is_none() && (whole == 0.0 || (travel - whole).abs() >= 1e-3) {
            self.smooth = true;
        }
        // Whole wheel notches each turn, until fractions show a touchpad.
        if phase.is_none() && !self.smooth {
            (self.travel, self.spent, self.held) = (0.0, true, false);
            return -(whole as i32);
        }
        self.held = match phase {
            Some(TouchPhase::Started) => true,
            Some(TouchPhase::Moved) => self.held,
            Some(TouchPhase::Ended | TouchPhase::Cancelled) | None => false,
        };
        // One gesture turns once, momentum included.
        if self.spent {
            return 0;
        }
        self.travel += travel;
        if self.travel.abs() < 1.0 - 1e-3 {
            return 0;
        }
        self.spent = true;
        // Content moving down shows the part before.
        -(self.travel.signum() as i32)
    }
}

impl LokView {
    /// The wheel and Up, Down turn parts from now on.
    pub fn turn_parts(&mut self) {
        self.turning.get_or_insert_default();
    }

    /// Queues Up, Down; the worker turns if nothing is picked.
    pub(super) fn turn_key(&mut self, keystroke: &Keystroke) -> bool {
        if self.turning.is_none() || keystroke.modifiers != Modifiers::none() {
            return false;
        }
        let step = match keystroke.key.as_str() {
            "down" => 1,
            "up" => -1,
            _ => return false,
        };
        let Some(key) = vcl_key(keystroke) else {
            return false;
        };
        self.send(Input::Turn { step, key: Some(key) });
        true
    }

    /// The wheel turns while the whole page shows.
    pub(super) fn turn_wheel(&mut self, event: &ScrollEvent) -> bool {
        let (width, height) = self.extent();
        // A pixel of slack for rounding.
        let whole = self.geom.px_of(width) <= self.viewport.0 + 1.0
            && self.geom.px_of(height) <= self.viewport.1 + 1.0;
        let Some(turning) = self.turning.as_mut().filter(|_| whole) else {
            return false;
        };
        let step = turning.wheel(event.delta, event.touch_phase, Instant::now(), notch());
        if step != 0 {
            // In order with the input around it.
            self.send(Input::Turn { step, key: None });
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use gpui::{ScrollDelta, TouchPhase, point, px};

    use super::{PAUSE, Turning};

    const NOTCH: f32 = 3.0;

    fn lines(y: f32) -> ScrollDelta {
        ScrollDelta::Lines(point(0.0, y))
    }

    fn pixels(y: f32) -> ScrollDelta {
        ScrollDelta::Pixels(point(px(0.0), px(y)))
    }

    #[test]
    fn each_notch_with_a_pause_turns_once() {
        let (mut turning, now) = (Turning::default(), std::time::Instant::now());
        let mut wheel = |y, at| turning.wheel(lines(y), TouchPhase::Moved, at, NOTCH);
        assert_eq!(wheel(-3.0, now), 1, "Windows and Linux scroll three lines a notch");
        assert_eq!(wheel(-3.0, now + PAUSE), 1);
        assert_eq!(wheel(3.0, now + PAUSE * 2), -1);
        assert_eq!(wheel(-6.0, now + PAUSE * 3), 2, "two notches in one event");
        assert_eq!(wheel(-3.0, now + PAUSE * 3), 1, "rapid notches each turn");
        assert_eq!(wheel(-3.0, now + PAUSE * 3), 1);
        assert_eq!(wheel(-1.0, now + PAUSE * 4), 0, "a third of a notch");
        let mut mac = Turning::default();
        assert_eq!(mac.wheel(lines(-1.0), TouchPhase::Moved, now, 1.0), 1, "macOS scrolls a line a notch");
    }

    #[test]
    fn a_stream_of_lines_turns_once() {
        let (mut turning, now) = (Turning::default(), std::time::Instant::now());
        // A whole step, then fractions: no more turns once smooth.
        let stream = [-3.0, -0.4, -0.4, -0.4, -0.4, -0.4, -0.4, -0.4, -0.4, -3.0];
        let turned: i32 = stream.iter().map(|&y| turning.wheel(lines(y), TouchPhase::Moved, now, NOTCH)).sum();
        assert_eq!(turned, 1, "one swipe turns once");
        let mut turning = Turning::default();
        let turned: i32 = (0..30).map(|_| turning.wheel(lines(-0.4), TouchPhase::Moved, now, NOTCH)).sum();
        assert_eq!(turned, 1, "fractions add up once");
        assert_eq!(turning.wheel(lines(-3.0), TouchPhase::Moved, now + PAUSE, NOTCH), 1, "a pause ends it");
    }

    #[test]
    fn a_swipe_turns_once_until_the_next() {
        let (mut turning, now) = (Turning::default(), std::time::Instant::now());
        let mut swipe = |y, phase, at| turning.wheel(pixels(y), phase, at, NOTCH);
        assert_eq!(swipe(-30.0, TouchPhase::Started, now), 0);
        assert_eq!(swipe(-30.0, TouchPhase::Moved, now), 1);
        // A pause with fingers down is the same gesture.
        assert_eq!(swipe(-60.0, TouchPhase::Moved, now + PAUSE * 2), 0);
        assert_eq!(swipe(0.0, TouchPhase::Ended, now + PAUSE * 2), 0);
        // Momentum after the turn turns nothing more.
        for _ in 0..20 {
            assert_eq!(swipe(-80.0, TouchPhase::Moved, now + PAUSE * 2), 0);
        }
        assert_eq!(swipe(60.0, TouchPhase::Started, now + PAUSE * 2), -1);
        assert_eq!(swipe(0.0, TouchPhase::Ended, now + PAUSE * 2), 0);
        // Without phases a pause ends a gesture (Wayland).
        let later = now + PAUSE * 4;
        assert_eq!(swipe(60.0, TouchPhase::Moved, later), -1);
        assert_eq!(swipe(60.0, TouchPhase::Moved, later), 0);
        assert_eq!(swipe(60.0, TouchPhase::Moved, later + PAUSE), -1);
    }
}
