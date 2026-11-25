//! The font size box: common sizes, or any typed one.

use std::rc::Rc;

use ely_gpui_component::{
    forms::{Choice, Combobox, InputEvent, TextInput},
    theme::ControlSize,
};
use gpui::{AnyElement, App, AppContext as _, Context, Entity, Focusable as _, IntoElement, SharedString, Subscription, Window};

use super::{Panel, known, ordered, send, text};

/// Sizes the list offers, in points.
const SIZES: [&str; 17] = [
    "8", "9", "10", "10.5", "11", "12", "14", "16", "18", "20", "22", "24", "26", "28", "36", "48",
    "72",
];

/// Sends a typed size.
type Commit = Rc<dyn Fn(&str, &mut App)>;

/// The field, its last send, the kernel's last report.
struct Field {
    input: Entity<TextInput>,
    sent: Option<SharedString>,
    reported: Option<SharedString>,
    commit: Option<Commit>,
    _events: Subscription,
}

impl Field {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| TextInput::new(window, cx));
        let events = cx.subscribe_in(&input, window, |field: &mut Self, input, event: &InputEvent, _, cx| match event {
            InputEvent::Focus => field.sent = None,
            InputEvent::Submit | InputEvent::Blur => {
                let typed = SharedString::from(input.read(cx).text().trim().to_string());
                if typed.is_empty() || field.sent.as_ref() == Some(&typed) {
                    return;
                }
                field.sent = Some(typed.clone());
                if let Some(commit) = field.commit.clone() {
                    commit(&typed, cx);
                }
            }
            _ => {}
        });
        Self { input, sent: None, reported: None, commit: None, _events: events }
    }
}

/// A size turned mixed or unknown clears the shown one.
fn follow(field: &Entity<Field>, reported: Option<SharedString>, window: &Window, cx: &mut App) {
    if field.read(cx).reported == reported {
        return;
    }
    let input = field.read(cx).input.clone();
    let typing = input.read(cx).focus_handle(cx).is_focused(window);
    if !typing && !reported.as_deref().is_some_and(known) {
        input.update(cx, |input, cx| input.set_text("", cx));
    }
    field.update(cx, |field, _| field.reported = reported);
}

/// Sends a typed size; a refused one reverts.
fn commit_for<P: Panel>(this: &Entity<P>, input: &Entity<TextInput>, shown: Option<SharedString>) -> Commit {
    let (this, input) = (this.clone(), input.clone());
    Rc::new(move |typed, cx| {
        if shown.as_deref() == Some(typed) {
            return;
        }
        log::info!("office: a typed font size goes to the kernel");
        send(&this, "format.size", &[typed], cx);
        let refused = ely_lok::commands::find("format.size").and_then(|size| size.args_json(&[typed])).is_err();
        if refused {
            let back = shown.as_deref().unwrap_or_default().to_string();
            input.update(cx, |input, cx| input.set_text(back, cx));
        }
    })
}

/// The size box; Enter or leaving sends a typed size.
pub fn size_box<P: Panel>(
    panel: &P,
    (id, name): (&'static str, SharedString),
    window: &mut Window,
    cx: &mut Context<P>,
) -> AnyElement {
    let this = cx.entity();
    let field = window.use_keyed_state(SharedString::from(format!("{id}-field")), cx, Field::new);
    let input = field.read(cx).input.clone();
    let reported = panel.state(".uno:FontHeight", cx);
    follow(&field, reported.clone(), window, cx);
    let size = reported.clone().filter(|size| known(size));
    let commit = commit_for(&this, &input, size.clone());
    field.update(cx, |field, _| field.commit = Some(commit));
    let hint = match reported.as_deref() == Some(ely_lok::MIXED) {
        true => text(cx, "office.props.mixed"),
        false => SharedString::default(),
    };
    if input.read(cx).label_text() != &name {
        input.update(cx, |input, cx| input.set_label(name, cx));
    }
    if input.read(cx).placeholder_text() != &hint {
        input.update(cx, |input, cx| input.set_placeholder(hint, cx));
    }
    let mut sizes: Vec<SharedString> = SIZES.iter().map(|size| (*size).into()).collect();
    if let Some(size) = size.clone().filter(|size| !sizes.contains(size)) {
        sizes.push(size);
    }
    let choices = ordered(sizes).into_iter().map(|size| Choice::new(size.clone(), size));
    let mut sized = Combobox::new(id, &input, choices)
        .free()
        .size(ControlSize::Sm)
        .steps(|text, by| stepped(text, f64::from(by)).map(SharedString::from))
        .on_change(move |value, _, cx| {
            // A pick or step is sent; leaving sends nothing.
            field.update(cx, |field, _| field.sent = Some(value.clone()));
            send(&this, "format.size", &[value.as_ref()], cx)
        });
    if let Some(size) = size {
        sized = sized.selected(size);
    }
    sized.into_any_element()
}

/// A size a point off; None if not a number.
fn stepped(from: &str, by: f64) -> Option<String> {
    let size = from.trim().parse::<f64>().ok().filter(|size| size.is_finite())?;
    Some(format!("{}", (size + by).clamp(1.0, 409.0)))
}

#[cfg(test)]
mod tests {
    use gpui::{Render, TestAppContext, VisualTestContext};

    use super::*;

    struct Blank;

    impl Render for Blank {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            gpui::div()
        }
    }

    /// Reports 12 pt; keeps the commands it ran.
    struct Fake(Vec<Vec<String>>);

    impl Panel for Fake {
        fn state(&self, uno: &str, _: &App) -> Option<SharedString> {
            (uno == ".uno:FontHeight").then(|| "12".into())
        }

        fn run(&mut self, id: &str, values: &[&str], _: &mut Context<Self>) {
            self.0.push([id].iter().chain(values).map(|value| value.to_string()).collect());
        }
    }

    #[test]
    fn the_arrows_step_one_point_within_the_limits() {
        assert_eq!(stepped("12", 1.0).as_deref(), Some("13"));
        assert_eq!(stepped("10.5", -1.0).as_deref(), Some("9.5"));
        assert_eq!(stepped("1", -1.0).as_deref(), Some("1"));
        assert_eq!(stepped("409", 1.0).as_deref(), Some("409"));
        assert_eq!(stepped("", 1.0), None);
        assert_eq!(stepped("big", 1.0), None);
        for odd in ["NaN", "inf", "-inf", "1e309"] {
            assert_eq!(stepped(odd, 1.0), None, "{odd}");
        }
    }

    #[test]
    fn typed_sizes_go_once_and_a_refused_one_reverts() {
        let mut cx = TestAppContext::single();
        cx.update(ely_gpui_component::init_for_tests);
        let window = cx.add_window(|_, _| Blank);
        let (panel, field) = window
            .update(&mut cx, |_, window, cx| {
                let panel = cx.new(|_| Fake(Vec::new()));
                let field = cx.new(|cx| Field::new(window, cx));
                (panel, field)
            })
            .expect("the window lives");
        let input = cx.update(|app| field.read(app).input.clone());
        let commit = commit_for(&panel, &input, Some("12".into()));
        cx.update(|app| field.update(app, |field, _| field.commit = Some(commit)));
        let typed = |cx: &mut TestAppContext, text: &str, event: InputEvent| {
            cx.update(|app| {
                input.update(app, |input, cx| {
                    input.set_text(text, cx);
                    cx.emit(event)
                })
            });
            cx.run_until_parked();
        };
        let ran = |cx: &mut TestAppContext| cx.update(|app| panel.read(app).0.clone());
        typed(&mut cx, "15", InputEvent::Submit);
        typed(&mut cx, "15", InputEvent::Blur);
        assert_eq!(ran(&mut cx), [["format.size", "15"]], "sent once");
        typed(&mut cx, "15", InputEvent::Focus);
        typed(&mut cx, "12", InputEvent::Submit);
        assert_eq!(ran(&mut cx).len(), 1, "the shown size is not sent");
        typed(&mut cx, "7.5", InputEvent::Blur);
        assert_eq!(ran(&mut cx)[1], ["format.size", "7.5"], "any size goes");
        typed(&mut cx, "big", InputEvent::Submit);
        assert_eq!(ran(&mut cx)[2], ["format.size", "big"], "the kernel's table refuses it");
        assert_eq!(cx.update(|app| input.read(app).text().to_string()), "12", "the box shows 12 again");
    }

    /// Draws the real size box for a reported size.
    struct Shown(Option<SharedString>, Option<Entity<TextInput>>);

    impl Panel for Shown {
        fn state(&self, uno: &str, _: &App) -> Option<SharedString> {
            (uno == ".uno:FontHeight").then(|| self.0.clone()).flatten()
        }

        fn run(&mut self, _: &str, _: &[&str], _: &mut Context<Self>) {}
    }

    impl Render for Shown {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let sized = size_box(self, ("size", "Size".into()), window, cx);
            let field = window.use_keyed_state(SharedString::from("size-field"), cx, Field::new);
            self.1 = Some(field.read(cx).input.clone());
            sized
        }
    }

    #[test]
    fn a_size_turned_mixed_shows_its_hint_not_the_old_size() {
        let mut cx = TestAppContext::single();
        cx.update(|cx| {
            ely_gpui_component::init_for_tests(cx);
            ely_ui::i18n::install_as("en-US", cx);
        });
        let window = cx.add_window(|_, _| Shown(None, None));
        let mut cx = VisualTestContext::from_window(window.into(), &cx);
        let shown = |cx: &mut VisualTestContext, reported: Option<&str>, focus: bool| {
            window
                .update(cx, |shown, window, cx| {
                    shown.0 = reported.map(SharedString::from);
                    if let (true, Some(input)) = (focus, shown.1.clone()) {
                        window.focus(&input.focus_handle(cx), cx);
                    }
                    cx.notify();
                })
                .expect("the window lives");
            cx.run_until_parked();
            window
                .update(cx, |shown, _, cx| {
                    let input = shown.1.clone().expect("the box drew");
                    let input = input.read(cx);
                    (input.text().to_string(), input.placeholder_text().to_string())
                })
                .expect("the window lives")
        };
        let pair = |text: &str, hint: &str| (text.to_string(), hint.to_string());
        assert_eq!(shown(&mut cx, Some("17"), false), pair("17", ""));
        assert_eq!(shown(&mut cx, Some(ely_lok::MIXED), false), pair("", "Mixed"), "the hint shows");
        assert_eq!(shown(&mut cx, Some("11"), false), pair("11", ""));
        assert_eq!(shown(&mut cx, Some(ely_lok::MIXED), true).0, "11", "a size being typed stays");
    }

    /// The real size box at 12 pt, keeping its commands.
    struct Arrows(Vec<Vec<String>>);

    impl Panel for Arrows {
        fn state(&self, uno: &str, _: &App) -> Option<SharedString> {
            (uno == ".uno:FontHeight").then(|| "12".into())
        }

        fn run(&mut self, id: &str, values: &[&str], _: &mut Context<Self>) {
            self.0.push([id].iter().chain(values).map(|value| value.to_string()).collect());
        }
    }

    impl Render for Arrows {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            use gpui::{ParentElement as _, Styled as _};
            gpui::div().w(gpui::px(160.)).child(size_box(self, ("size", "Size".into()), window, cx))
        }
    }

    #[test]
    fn the_size_arrows_send_a_point_up_and_down() {
        let mut cx = TestAppContext::single();
        cx.update(|cx| {
            ely_gpui_component::init_for_tests(cx);
            ely_ui::i18n::install_as("en-US", cx);
        });
        let window = cx.add_window(|_, _| Arrows(Vec::new()));
        let mut cx = VisualTestContext::from_window(window.into(), &cx);
        cx.run_until_parked();
        for half in ["combobox-step-up", "combobox-step-down", "combobox-step-down"] {
            let at = cx.debug_bounds(half).unwrap_or_else(|| panic!("{half} is drawn")).center();
            cx.simulate_mouse_move(at, None, gpui::Modifiers::none());
            cx.simulate_click(at, gpui::Modifiers::none());
            cx.run_until_parked();
        }
        let ran = window.update(&mut cx, |arrows, _, _| arrows.0.clone()).expect("the window lives");
        assert_eq!(ran, [["format.size", "13"], ["format.size", "12"], ["format.size", "11"]]);
    }
}
