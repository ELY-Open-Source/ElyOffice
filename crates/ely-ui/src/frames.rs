//! Benchmarks only: frame times from gpui's journal (`ELY_FRAMES`).

use gpui::App;

/// Logs each busy stretch's frame times; idle logs nothing.
#[cfg(feature = "frames")]
pub(crate) fn watch(cx: &mut App) {
    use gpui::profiler::journal::{ForegroundEvent, ForegroundJournalEntry as Entry, IntervalBoundary};
    use std::time::Duration;
    const EVERY: Duration = Duration::from_secs(2);
    let mut collector = cx.foreground_journal().collector();
    cx.spawn(async move |cx: &mut gpui::AsyncApp| {
        let mut waiting = None;
        loop {
            cx.background_executor().timer(EVERY).await;
            let drained = collector.collect_unseen();
            if drained.lost > 0 {
                answered(&mut waiting, Step::Lost);
            }
            let mut draws = Vec::new();
            let mut lags = Vec::new();
            for entry in drained.entries {
                let step = match entry {
                    // Typed text lands after the key's own dispatch.
                    Entry::Event(ForegroundEvent::Input(input)) => Step::Pressed(input.kind, input.start),
                    Entry::Boundary(IntervalBoundary::Presented(shown)) => {
                        draws.push(shown.frame.draw_duration());
                        lags.extend(shown.dirty_to_present_duration());
                        Step::Shown(shown.frame.dirty_at, shown.presentation.present_end)
                    }
                    Entry::Discontinuity { .. } => Step::Lost,
                    _ => continue,
                };
                if let Some((kind, took)) = answered(&mut waiting, step) {
                    log::debug!("ui: {kind} to frame {} ms", took.as_millis());
                }
            }
            if !draws.is_empty() {
                log::info!("{}", summary(&mut draws, &mut lags, drained.lost));
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

/// A press, and the first frame dirtied after it.
#[cfg(any(feature = "frames", test))]
fn answered(
    waiting: &mut Option<(&'static str, std::time::Instant)>,
    step: Step,
) -> Option<(&'static str, std::time::Duration)> {
    match step {
        Step::Pressed(kind @ ("key_down" | "mouse_down"), at) => {
            waiting.get_or_insert((kind, at));
            None
        }
        Step::Pressed(..) => None,
        // A frame already pending cannot show the press.
        Step::Shown(Some(dirty), at) if waiting.is_some_and(|(_, pressed)| dirty >= pressed) => {
            waiting.take().map(|(kind, pressed)| (kind, at.duration_since(pressed)))
        }
        Step::Shown(..) => None,
        // A gap may hide the frame that answered.
        Step::Lost => {
            *waiting = None;
            None
        }
    }
}

/// One line: counts, draw and dirty-to-present percentiles in ms.
#[cfg(any(feature = "frames", test))]
fn summary(draws: &mut [std::time::Duration], lags: &mut [std::time::Duration], lost: u64) -> String {
    draws.sort();
    lags.sort();
    let ms = |all: &[std::time::Duration], p: usize| match all.len() {
        0 => f64::NAN,
        n => all[(n - 1) * p / 100].as_secs_f64() * 1000.0,
    };
    // 120 fps leaves 8.3 ms a frame.
    let late = lags.iter().filter(|lag| lag.as_secs_f64() > 1.0 / 120.0).count();
    format!(
        "ui: frames {} (lost {lost}): draw p50 {:.1} p95 {:.1} max {:.1} ms; \
         dirty to shown p50 {:.1} p95 {:.1} max {:.1} ms; over 8.3 ms {late}",
        draws.len(),
        ms(draws, 50),
        ms(draws, 95),
        ms(draws, 100),
        ms(lags, 50),
        ms(lags, 95),
        ms(lags, 100),
    )
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Step, answered};

    #[test]
    fn a_press_is_timed_to_the_first_frame_dirtied_after_it() {
        let at = Instant::now();
        let later = |ms| at + Duration::from_millis(ms);
        let shown = |dirty, ms| Step::Shown(Some(later(dirty)), later(ms));
        let mut waiting = None;
        assert_eq!(answered(&mut waiting, shown(0, 1)), None, "no press, no time");
        assert_eq!(answered(&mut waiting, Step::Pressed("key_down", later(2))), None);
        assert_eq!(answered(&mut waiting, shown(1, 3)), None, "a frame pending before the key");
        assert_eq!(answered(&mut waiting, Step::Shown(None, later(4))), None, "no dirty time");
        // A second press before the frame keeps the first.
        answered(&mut waiting, Step::Pressed("key_down", later(5)));
        answered(&mut waiting, Step::Pressed("mouse_move", later(6)));
        let took = answered(&mut waiting, shown(7, 12));
        assert_eq!(took, Some(("key_down", Duration::from_millis(10))));
        assert_eq!(answered(&mut waiting, shown(15, 20)), None, "one frame answers once");
        answered(&mut waiting, Step::Pressed("mouse_down", later(30)));
        assert_eq!(answered(&mut waiting, Step::Lost), None);
        assert_eq!(answered(&mut waiting, shown(35, 40)), None, "a gap drops the press");
        answered(&mut waiting, Step::Pressed("mouse_move", later(50)));
        assert_eq!(answered(&mut waiting, shown(55, 60)), None, "a move is no press");
    }

    #[test]
    fn the_summary_reads_percentiles_and_late_frames() {
        let mut draws: Vec<_> = (1..=100).map(Duration::from_millis).collect();
        let mut lags = vec![Duration::from_millis(4), Duration::from_millis(9), Duration::from_millis(20)];
        let line = super::summary(&mut draws, &mut lags, 0);
        assert!(line.contains("frames 100"), "{line}");
        assert!(line.contains("draw p50 50.0 p95 95.0 max 100.0"), "{line}");
        assert!(line.contains("dirty to shown p50 9.0 p95 9.0 max 20.0"), "{line}");
        assert!(line.ends_with("over 8.3 ms 2"), "{line}");
    }
}
