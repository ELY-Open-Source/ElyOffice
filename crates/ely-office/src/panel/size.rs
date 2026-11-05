//! The font size box: common sizes, or any typed one.

use std::rc::Rc;

use ely_gpui_component::{
    forms::{Choice, Combobox, InputEvent, TextInput},
    theme::ControlSize,
};
use gpui::{AnyElement, App, AppContext as _, Context, Entity, IntoElement, SharedString, Subscription, Window};

use super::{Panel, known, ordered, send, text};

/// Sizes the list offers, in points.
const SIZES: [&str; 17] = [
    "8", "9", "10", "10.5", "11", "12", "14", "16", "18", "20", "22", "24", "26", "28", "36", "48",
    "72",
];

/// Sends a typed size.
type Commit = Rc<dyn Fn(&str, &mut App)>;

/// The box's field and what one edit already sent.
struct Field {
    input: Entity<TextInput>,
    sent: Option<SharedString>,
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
        Self { input, sent: None, commit: None, _events: events }
    }
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
        .on_change(move |value, _, cx| {
            // A picked row is sent; leaving sends nothing more.
            field.update(cx, |field, _| field.sent = Some(value.clone()));
            send(&this, "format.size", &[value.as_ref()], cx)
        });
    if let Some(size) = size {
        sized = sized.selected(size);
    }
    sized.into_any_element()
}

#[cfg(test)]
mod tests {
    use gpui::{Render, TestAppContext};

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
}
