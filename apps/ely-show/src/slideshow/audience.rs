//! The audience screen: the slide alone, full screen.

use ely_gpui_component::{
    feedback::InlineMessage,
    primitives::{FocusScope, Severity},
    theme::ActiveTheme as _,
};
use gpui::{
    Context, Entity, FocusHandle, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Render, Styled as _, Window, div,
};

use super::{Show, stage};

pub struct Audience {
    pub show: Entity<Show>,
    pub focus: FocusHandle,
}

impl Audience {
    pub fn new(show: Entity<Show>, cx: &mut Context<Self>) -> Self {
        cx.observe(&show, |_, _, cx| cx.notify()).detach();
        Audience {
            show,
            focus: cx.focus_handle(),
        }
    }
}

impl Render for Audience {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.show.update(cx, |show, cx| show.media_tick(cx)) {
            window.request_animation_frame();
        }
        self.show.update(cx, |show, _| show.free(0, window));
        let show = self.show.read(cx);
        let room = window.viewport_size();
        let size = stage::fit(show.slide, (room.width, room.height));
        let origin = ((room.width - size.0) / 2.0, (room.height - size.1) / 2.0);
        if show.moving() {
            window.request_animation_frame();
        }
        let shown = stage::stage(show, size, cx);
        // Alone, the audience screen says what failed.
        let notice = (show.notice.clone())
            .filter(|_| !show.presenter)
            .map(|why| {
                div()
                    .absolute()
                    .bottom_4()
                    .left_4()
                    .child(InlineMessage::new(Severity::Danger, why))
            });
        let (keys, click, back, moved) = (
            self.show.clone(),
            self.show.clone(),
            self.show.clone(),
            self.show.clone(),
        );
        // Keys reach ancestors of the focused scope.
        div()
            .size_full()
            .font_family(cx.theme().font_family.clone())
            .on_key_down(move |event, _, cx| {
                if keys.update(cx, |show, cx| show.key(&event.keystroke, cx)) {
                    cx.stop_propagation();
                }
            })
            .child(
                FocusScope::new(&self.focus).root().size_full().child(
                    div()
                        .id("show-audience")
                        .relative()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(gpui::black())
                        .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                            let x = (event.position.x - origin.0) / size.0;
                            let y = (event.position.y - origin.1) / size.1;
                            click.update(cx, |show, cx| match show.media_at((x, y)) {
                                Some(at) => show.toggle_media(at, cx),
                                None => show.next(cx),
                            })
                        })
                        .on_mouse_down(MouseButton::Right, move |_, _, cx| {
                            back.update(cx, |show, cx| show.previous(cx))
                        })
                        .on_mouse_move(move |event, _, cx| {
                            let x = (event.position.x - origin.0) / size.0;
                            let y = (event.position.y - origin.1) / size.1;
                            let inside = (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y);
                            moved.update(cx, |show, cx| show.point(inside.then_some((x, y)), cx))
                        })
                        .child(shown)
                        .children(notice),
                ),
            )
    }
}
