//! The start scene: one spec, drawn by launcher and app.

/// Icon and name fade in and grow, ms.
pub const ENTER: u64 = 180;
/// The whole scene fades out, ms.
pub const LEAVE: u64 = 150;
/// One sweep of the bar, ms.
pub const SWEEP: u64 = 1280;
/// The launcher's longest stay, ms.
pub const LONGEST: u64 = 3000;
/// The icon grows from this scale.
pub const FROM: f32 = 0.92;
/// The bar's share of its track.
pub const BAR: f32 = 0.3;
/// Where a still bar rests, as a share.
pub const STILL: f32 = 0.35;
/// Sizes in 96-dpi pixels.
pub const ICON: f32 = 96.0;
pub const TRACK: f32 = 120.0;
pub const THICK: f32 = 3.0;
/// Gaps: icon to name, name to bar.
pub const GAP: f32 = 16.0;
/// The name's size.
pub const NAME: f32 = 22.0;
/// Background, dark and light, as 0xRRGGBB.
pub const DARK: u32 = 0x25_2525;
pub const LIGHT: u32 = 0xff_ffff;

/// Icon and name at `ms` into the scene: opacity, scale.
pub fn enter(ms: u64, still: bool) -> (f32, f32) {
    if still {
        return (1.0, 1.0);
    }
    let t = ease_out_cubic(ms.min(ENTER) as f32 / ENTER as f32);
    (t, FROM + (1.0 - FROM) * t)
}

/// The bar's left edge at `ms`, a share.
pub fn bar(ms: u64, still: bool) -> f32 {
    if still {
        return STILL;
    }
    let t = ease_in_out_cubic((ms % SWEEP) as f32 / SWEEP as f32);
    -BAR + (1.0 + BAR) * t
}

/// The scene's opacity `ms` after it began leaving.
pub fn leave(ms: u64, still: bool) -> f32 {
    if still || ms >= LEAVE {
        return 0.0;
    }
    1.0 - ease_out_cubic(ms as f32 / LEAVE as f32)
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_arrives_in_enter_and_stays() {
        assert_eq!(enter(0, false), (0.0, FROM));
        assert_eq!(enter(ENTER, false), (1.0, 1.0));
        assert_eq!(enter(ENTER * 10, false), (1.0, 1.0));
        assert_eq!(enter(0, true), (1.0, 1.0), "reduced motion starts in place");
    }

    #[test]
    fn the_bar_sweeps_across_and_holds_still_when_asked() {
        assert_eq!(bar(0, false), -BAR);
        assert!((bar(SWEEP / 2, false) - (1.0 - BAR) / 2.0).abs() < 1e-6);
        assert_eq!(bar(SWEEP, false), -BAR, "the sweep loops");
        assert_eq!(bar(SWEEP / 3, true), STILL);
    }

    #[test]
    fn leaving_fades_out_in_leave_and_at_once_when_still() {
        assert_eq!(leave(0, false), 1.0);
        assert_eq!(leave(LEAVE, false), 0.0);
        assert_eq!(leave(0, true), 0.0);
    }
}
