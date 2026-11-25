//! Benchmarks only: frame times from gpui's journal (`ELY_FRAMES`).

use gpui::App;

/// Logs each busy stretch's frame times; idle logs nothing.
#[cfg(feature = "frames")]
pub(crate) fn watch(cx: &mut App) {
    use gpui::profiler::journal::{
        ForegroundEvent, ForegroundJournalEntry as Entry, FrameSkipReason, IntervalBoundary,
    };
    use std::time::Duration;
    const EVERY: Duration = Duration::from_secs(2);
    let mut collector = cx.foreground_journal().collector();
    cx.spawn(async move |cx: &mut gpui::AsyncApp| {
        let mut waiting = Vec::new();
        loop {
            cx.background_executor().timer(EVERY).await;
            let drained = collector.collect_unseen();
            if drained.lost > 0 {
                answered(&mut waiting, Step::Lost);
            }
            let mut draws = Vec::new();
            let mut lags = Vec::new();
            let (mut waits, mut presents, mut refresh) = (Vec::new(), Vec::new(), None);
            let mut held = 0;
            for entry in drained.entries {
                let step = match entry {
                    // gpui slows an unfocused window's frames.
                    Entry::Boundary(IntervalBoundary::FrameSkipped(skip))
                        if skip.reason == FrameSkipReason::InactiveFrameRateLimit =>
                    {
                        held += 1;
                        continue;
                    }
                    // Typed text lands after the key's own dispatch.
                    Entry::Event(ForegroundEvent::Input(input)) => {
                        Step::Pressed(input.kind, input.start)
                    }
                    Entry::Boundary(IntervalBoundary::Presented(shown)) => {
                        draws.push(shown.frame.draw_duration());
                        lags.extend(shown.dirty_to_present_duration());
                        let frame = &shown.frame;
                        waits.extend(frame.dirty_at.map(|dirty| frame.draw_start.saturating_duration_since(dirty)));
                        let present = &shown.presentation;
                        presents.push(present.present_end.saturating_duration_since(frame.draw_end));
                        refresh = frame.refresh_interval.or(refresh);
                        Step::Shown(shown.frame.dirty_at, shown.presentation.present_end)
                    }
                    Entry::Discontinuity { .. } => Step::Lost,
                    _ => continue,
                };
                for (kind, took) in answered(&mut waiting, step) {
                    match took {
                        Some(took) => log::debug!("ui: {kind} to frame {} ms", took.as_millis()),
                        None => log::debug!("ui: {kind} drew nothing"),
                    }
                }
            }
            if !draws.is_empty() {
                log::info!("{}", summary(&mut draws, &mut lags, drained.lost, held));
                log::info!("{}", parts(&mut waits, &mut presents, refresh));
            }
        }
    })
    .detach();
}

#[cfg(not(feature = "frames"))]
pub(crate) fn watch(_: &mut App) {
    log::warn!("ui: ELY_FRAMES needs a build with the frames feature");
}

/// What the journal saw, as PERF-02 reads it.
#[cfg(any(feature = "frames", test))]
enum Step {
    Pressed(&'static str, std::time::Instant),
    /// When the frame first went dirty, and when shown.
    Shown(Option<std::time::Instant>, std::time::Instant),
    Lost,
}

/// Presses a frame settles: each answered one timed, else None.
#[cfg(any(feature = "frames", test))]
fn answered(
    waiting: &mut Vec<(&'static str, std::time::Instant)>,
    step: Step,
) -> Vec<(&'static str, Option<std::time::Duration>)> {
    match step {
        Step::Pressed(kind @ ("key_down" | "mouse_down"), at) => {
            waiting.push((kind, at));
            Vec::new()
        }
        // Presses dispatch in turn: the last before dirty made it.
        Step::Shown(Some(dirty), at) => {
            let Some(last) = waiting.iter().rposition(|(_, pressed)| *pressed <= dirty) else {
                return Vec::new();
            };
            let (kind, pressed) = waiting[last];
            let mut settled: Vec<_> = waiting
                .drain(..last)
                .map(|(kind, _)| (kind, None))
                .collect();
            waiting.remove(0);
            settled.push((kind, Some(at.duration_since(pressed))));
            settled
        }
        Step::Pressed(..) | Step::Shown(None, _) => Vec::new(),
        // A gap may hide the frame that answered.
        Step::Lost => {
            waiting.clear();
            Vec::new()
        }
    }
}

/// One line: counts, draw and dirty-to-present percentiles in ms.
#[cfg(any(feature = "frames", test))]
fn summary(
    draws: &mut [std::time::Duration],
    lags: &mut [std::time::Duration],
    lost: u64,
    held: u64,
) -> String {
    draws.sort();
    lags.sort();
    // 120 fps leaves 8.3 ms a frame.
    let late = lags
        .iter()
        .filter(|lag| lag.as_secs_f64() > 1.0 / 120.0)
        .count();
    format!(
        "ui: frames {} (lost {lost}): draw p50 {:.1} p95 {:.1} max {:.1} ms; \
         dirty to shown p50 {:.1} p95 {:.1} max {:.1} ms; over 8.3 ms {late}; held while inactive {held}",
        draws.len(),
        ms(draws, 50),
        ms(draws, 95),
        ms(draws, 100),
        ms(lags, 50),
        ms(lags, 95),
        ms(lags, 100),
    )
}

/// Dirty to shown, split at the draw.
#[cfg(any(feature = "frames", test))]
fn parts(
    waits: &mut [std::time::Duration],
    presents: &mut [std::time::Duration],
    refresh: Option<std::time::Duration>,
) -> String {
    waits.sort();
    presents.sort();
    let refresh = refresh.map_or("unknown".to_string(), |every| format!("{:.1} ms", every.as_secs_f64() * 1000.0));
    format!(
        "ui: frame parts: wait to draw p50 {:.1} p95 {:.1} ms; draw end to shown p50 {:.1} p95 {:.1} ms; refresh {refresh}",
        ms(waits, 50),
        ms(waits, 95),
        ms(presents, 50),
        ms(presents, 95),
    )
}

/// The `p`th percentile of sorted times, in ms.
#[cfg(any(feature = "frames", test))]
fn ms(all: &[std::time::Duration], p: usize) -> f64 {
    match all.len() {
        0 => f64::NAN,
        n => all[(n - 1) * p / 100].as_secs_f64() * 1000.0,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Step, answered};

    fn ms(n: u64) -> Option<Duration> {
        Some(Duration::from_millis(n))
    }

    #[test]
    fn a_press_is_timed_to_the_first_frame_dirtied_after_it() {
        let at = Instant::now();
        let later = |ms| at + Duration::from_millis(ms);
        let shown = |dirty, ms| Step::Shown(Some(later(dirty)), later(ms));
        let mut waiting = Vec::new();
        assert_eq!(answered(&mut waiting, shown(0, 1)), [], "no press, no time");
        assert_eq!(
            answered(&mut waiting, Step::Pressed("key_down", later(2))),
            []
        );
        assert_eq!(
            answered(&mut waiting, shown(1, 3)),
            [],
            "a frame pending before the key"
        );
        assert_eq!(
            answered(&mut waiting, Step::Shown(None, later(4))),
            [],
            "no dirty time"
        );
        // Two keys before one frame: the second made it.
        answered(&mut waiting, Step::Pressed("key_down", later(5)));
        answered(&mut waiting, Step::Pressed("mouse_move", later(6)));
        assert_eq!(
            answered(&mut waiting, shown(7, 12)),
            [("key_down", None), ("key_down", ms(7))]
        );
        assert_eq!(
            answered(&mut waiting, shown(15, 20)),
            [],
            "one frame answers once"
        );
        answered(&mut waiting, Step::Pressed("mouse_down", later(30)));
        assert_eq!(answered(&mut waiting, Step::Lost), []);
        assert_eq!(
            answered(&mut waiting, shown(35, 40)),
            [],
            "a gap drops the press"
        );
        answered(&mut waiting, Step::Pressed("mouse_move", later(50)));
        assert_eq!(
            answered(&mut waiting, shown(55, 60)),
            [],
            "a move is no press"
        );
    }

    #[test]
    fn a_click_that_drew_nothing_leaves_the_key_its_frame() {
        let at = Instant::now();
        let later = |ms| at + Duration::from_millis(ms);
        let mut waiting = Vec::new();
        answered(&mut waiting, Step::Pressed("mouse_down", later(2)));
        answered(&mut waiting, Step::Pressed("key_down", later(3000)));
        let settled = answered(&mut waiting, Step::Shown(Some(later(3001)), later(3010)));
        assert_eq!(settled, [("mouse_down", None), ("key_down", ms(10))]);
        assert!(waiting.is_empty());
    }

    #[test]
    fn a_press_after_the_frame_went_dirty_waits_for_the_next() {
        let at = Instant::now();
        let later = |ms| at + Duration::from_millis(ms);
        let mut waiting = Vec::new();
        answered(&mut waiting, Step::Pressed("key_down", later(2)));
        // Dirtied by the first key, shown after the second.
        answered(&mut waiting, Step::Pressed("key_down", later(5)));
        assert_eq!(
            answered(&mut waiting, Step::Shown(Some(later(3)), later(8))),
            [("key_down", ms(6))]
        );
        assert_eq!(
            answered(&mut waiting, Step::Shown(Some(later(9)), later(14))),
            [("key_down", ms(9))]
        );
    }

    #[test]
    fn a_gap_drops_every_waiting_press() {
        let at = Instant::now();
        let later = |ms| at + Duration::from_millis(ms);
        let mut waiting = Vec::new();
        answered(&mut waiting, Step::Pressed("key_down", later(2)));
        answered(&mut waiting, Step::Pressed("mouse_down", later(4)));
        assert_eq!(answered(&mut waiting, Step::Lost), []);
        assert!(waiting.is_empty());
        assert_eq!(
            answered(&mut waiting, Step::Shown(Some(later(6)), later(9))),
            []
        );
    }

    #[test]
    fn the_summary_reads_percentiles_and_late_frames() {
        let mut draws: Vec<_> = (1..=100).map(Duration::from_millis).collect();
        let mut lags = vec![
            Duration::from_millis(4),
            Duration::from_millis(9),
            Duration::from_millis(20),
        ];
        let line = super::summary(&mut draws, &mut lags, 0, 3);
        assert!(line.contains("frames 100"), "{line}");
        assert!(line.contains("draw p50 50.0 p95 95.0 max 100.0"), "{line}");
        assert!(
            line.contains("dirty to shown p50 9.0 p95 9.0 max 20.0"),
            "{line}"
        );
        assert!(
            line.ends_with("over 8.3 ms 2; held while inactive 3"),
            "{line}"
        );
    }

    #[test]
    fn the_parts_split_waiting_from_presenting() {
        let mut waits: Vec<_> = (1..=100).map(Duration::from_millis).collect();
        let mut presents = vec![Duration::from_millis(3), Duration::from_millis(5)];
        let line = super::parts(&mut waits, &mut presents, Some(Duration::from_micros(16_667)));
        assert!(line.contains("wait to draw p50 50.0 p95 95.0 ms"), "{line}");
        assert!(line.contains("draw end to shown p50 3.0 p95 3.0 ms"), "{line}");
        assert!(line.ends_with("refresh 16.7 ms"), "{line}");
        assert!(super::parts(&mut [], &mut [], None).ends_with("refresh unknown"));
    }
}
