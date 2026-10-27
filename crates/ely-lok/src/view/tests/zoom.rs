use super::*;

/// A view fitting a page far larger than 800 by 600.
fn fitted_large(view: &mut LokView, cx: &mut gpui::Context<LokView>) {
    view.doc_size = (170_000, 170_000);
    view.fit_page(cx);
    view.prepare(Bounds { origin: point(px(0.0), px(0.0)), size: size(px(800.0), px(600.0)) });
}

#[test]
fn gestures_without_magnification_keep_the_fit() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            fitted_large(view, cx);
            let (zoom, scroll) = (view.zoom(), view.scroll);
            let at = point(px(300.0), px(200.0));
            for phase in [gpui::TouchPhase::Started, gpui::TouchPhase::Ended] {
                let pinch = gpui::PinchEvent { position: at, delta: 0.0, modifiers: Default::default(), phase };
                view.pinch_zoom(&pinch, cx);
            }
            let sideways = gpui::ScrollWheelEvent {
                position: at,
                delta: gpui::ScrollDelta::Pixels(point(px(30.0), px(0.0))),
                modifiers: gpui::Modifiers::secondary_key(),
                ..Default::default()
            };
            assert!(view.wheel_zoom(&sideways, cx), "a Ctrl scroll is taken");
            assert!(view.fitting(), "nothing magnified: still fitting");
            assert_eq!((view.zoom(), view.scroll), (zoom, scroll), "the page stays put");
        });
    });
}

#[test]
fn zooming_out_from_a_small_fit_never_enlarges() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    cx.update(|app| {
        let (view, _) = live_view(app, &install);
        view.update(app, |view, cx| {
            fitted_large(view, cx);
            let fit = view.zoom();
            assert!(fit < 0.25, "the fit is under the least step: {fit}");
            view.zoom_step(false, cx);
            assert!(view.zoom() <= fit, "out went from {fit} to {}", view.zoom());
            view.zoom_step(true, cx);
            assert_eq!(view.zoom(), 0.25, "in reaches the least step");
        });
    });
}
