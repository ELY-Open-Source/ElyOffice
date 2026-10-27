//! Ctrl (Cmd on macOS) and the wheel step a zoom.

use gpui::{
    App, DispatchPhase, HitboxBehavior, IntoElement, ScrollDelta, ScrollEvent, Styled as _,
    Window, canvas,
};

/// Trackpad pixels one step takes.
const STEP: f32 = 50.0;

/// Steps `event` makes, plus is larger; `carry` keeps trackpad rest.
pub fn steps(event: &ScrollEvent, carry: &mut f32) -> i32 {
    match event.delta {
        ScrollDelta::Lines(lines) => match lines.y {
            0.0 => 0,
            y => y.signum() as i32,
        },
        ScrollDelta::Pixels(point) => {
            *carry += f32::from(point.y);
            let whole = (*carry / STEP).trunc();
            *carry -= whole * STEP;
            whole as i32
        }
    }
}

/// Takes a zooming wheel before the parent scrolls.
pub fn zoom_layer(
    on: impl Fn(&ScrollEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    canvas(
        |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
        move |_, hitbox, window, _| {
            window.on_mouse_event(move |event: &ScrollEvent, phase, window, cx| {
                if phase == DispatchPhase::Capture
                    && event.modifiers.secondary(cx)
                    && hitbox.is_hovered(window)
                {
                    cx.stop_propagation();
                    on(event, window, cx);
                }
            })
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

#[cfg(test)]
mod tests {
    use gpui::{ScrollDelta, ScrollEvent, point, px};

    use super::steps;

    fn wheel(delta: ScrollDelta) -> ScrollEvent {
        ScrollEvent {
            delta,
            ..Default::default()
        }
    }

    #[test]
    fn a_notch_is_one_step_and_pixels_add_up() {
        let mut carry = 0.0;
        assert_eq!(
            steps(&wheel(ScrollDelta::Lines(point(0.0, 3.0))), &mut carry),
            1
        );
        assert_eq!(
            steps(&wheel(ScrollDelta::Lines(point(0.0, -1.0))), &mut carry),
            -1
        );
        assert_eq!(
            steps(&wheel(ScrollDelta::Lines(point(2.0, 0.0))), &mut carry),
            0
        );
        let pixels = |y| wheel(ScrollDelta::Pixels(point(px(0.0), px(y))));
        assert_eq!(steps(&pixels(30.0), &mut carry), 0);
        assert_eq!(steps(&pixels(30.0), &mut carry), 1);
        assert_eq!(carry, 10.0);
        assert_eq!(steps(&pixels(-120.0), &mut carry), -2);
    }
}
