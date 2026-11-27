//! The shown slide at a size: transition, effects, black, laser.

use ely_gpui_component::theme::ActiveTheme as _;
use gpui::{AnyElement, App, IntoElement, ParentElement as _, Pixels, Styled as _, div, px};

use super::{Show, steps};
use crate::preview::{compose, transition};

/// The laser dot's width as a share of the slide.
const DOT: f32 = 0.012;

/// The largest slide size within `room`.
pub fn fit(slide: (f32, f32), room: (Pixels, Pixels)) -> (Pixels, Pixels) {
    let scale = (f32::from(room.0) / slide.0).min(f32::from(room.1) / slide.1);
    (px(slide.0 * scale), px(slide.1 * scale))
}

fn black((w, h): (Pixels, Pixels)) -> AnyElement {
    div().w(w).h(h).bg(gpui::black()).into_any_element()
}

/// The show's slide now, as the audience sees it.
pub fn stage(show: &Show, (w, h): (Pixels, Pixels), cx: &App) -> AnyElement {
    let Some(part) = show.part().filter(|_| !show.black) else {
        return black((w, h));
    };
    let Some(stack) = show.stack(part) else {
        return black((w, h));
    };
    let paper = ely_lok::paper();
    let t = show.entered.elapsed().as_secs_f64();
    let slide = &show.slides[part];
    let shown = match &show.leaving {
        Some(leaving) if t < slide.transition.duration => {
            let p = (t / slide.transition.duration) as f32;
            let frame = transition::frame(&slide.transition.preset, p);
            div()
                .bg(gpui::black())
                .size_full()
                .child(compose::transition((Some(compose::left((&leaving.stack, &leaving.runs), (show.slide, leaving.at), (w, h), paper)), stack), &frame, (w, h), paper))
                .into_any_element()
        }
        _ => {
            let runs = steps::runs(&slide.effects, &show.clicks);
            div()
                .bg(paper)
                .size_full()
                .child(compose::effects(stack, &runs, show.slide, t, (w, h)))
                .into_any_element()
        }
    };
    let dot = (show.pointer)
        .filter(|_| show.laser)
        .map(|(x, y)| {
            let side = w * DOT;
            div()
                .absolute()
                .left(w * x - side / 2.0)
                .top(h * y - side / 2.0)
                .size(side)
                .rounded_full()
                .bg(cx.theme().palette().danger)
        });
    let video = show.media_frame().map(|(frame, (x, y, vw, vh))| {
        crate::media::video(frame, (gpui::point(w * x, h * y), gpui::point(w * vw, h * vh)))
    });
    let countdown = show.countdown(part, cx).zip(show.timer(part)).map(|(face, (timer, clocks))| {
        let words = |key| crate::deck::Deck::text(cx, key, &[]);
        let (run, reset) = (words(face.phase.run_key()), words("show.timer.reset"));
        (face, super::timer::Hand { clocks: clocks.clone(), timer, run, reset, edit: None })
    });
    let over = super::overlays(show.board.as_ref(), countdown, part, (w, h), cx);
    div()
        .relative()
        .w(w)
        .h(h)
        .overflow_hidden()
        .child(shown)
        .children(video)
        .children(over)
        .children(dot)
        .into_any_element()
}

/// A slide as it first shows, still.
pub fn still(show: &Show, part: usize, size: (Pixels, Pixels)) -> AnyElement {
    match show.stack(part) {
        Some(stack) => compose::slide(stack, |visible| visible, size, ely_lok::paper())
            .into_any_element(),
        None => black(size),
    }
}
