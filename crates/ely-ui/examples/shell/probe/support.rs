use ely_ui::Root;
use gpui::{App, Entity, KeyUpEvent, Keystroke, Modifiers, OwnedMenuItem, PlatformInput, Window};

use crate::Gallery;

pub enum Probe {
    Again,
    Next,
    Keys(Vec<Keystroke>),
    KeysAgain(Vec<Keystroke>),
    Actions(Vec<Box<dyn gpui::Action>>),
    Click { x: f32, y: f32 },
    KeyUpOnly(Keystroke),
    Done,
}

pub fn run(
    root: Entity<Root>,
    view: Entity<Gallery>,
    step: usize,
    window: &mut Window,
    cx: &mut App,
) {
    match view.update(cx, |this, cx| this.probe_step(&root, step, window, cx)) {
        Probe::Again => window.on_next_frame(move |window, cx| run(root, view, step, window, cx)),
        Probe::Next => {
            window.on_next_frame(move |window, cx| run(root, view, step + 1, window, cx))
        }
        Probe::Keys(keys) => {
            for key in keys {
                window.dispatch_keystroke(key.clone(), cx);
                window.dispatch_event(PlatformInput::KeyUp(KeyUpEvent { keystroke: key }), cx);
            }
            window.on_next_frame(move |window, cx| run(root, view, step + 1, window, cx))
        }
        Probe::KeysAgain(keys) => {
            for key in keys {
                window.dispatch_keystroke(key.clone(), cx);
                window.dispatch_event(PlatformInput::KeyUp(KeyUpEvent { keystroke: key }), cx);
            }
            window.on_next_frame(move |window, cx| run(root, view, step, window, cx))
        }
        Probe::Click { x, y } => {
            let position = gpui::point(gpui::px(x), gpui::px(y));
            let modifiers = Modifiers::default();
            window.dispatch_event(
                PlatformInput::MouseDown(gpui::MouseDownEvent {
                    button: gpui::MouseButton::Left,
                    position,
                    modifiers,
                    click_count: 1,
                    first_mouse: false,
                }),
                cx,
            );
            window.dispatch_event(
                PlatformInput::MouseUp(gpui::MouseUpEvent {
                    button: gpui::MouseButton::Left,
                    position,
                    modifiers,
                    click_count: 1,
                }),
                cx,
            );
            window.on_next_frame(move |window, cx| run(root, view, step + 1, window, cx))
        }
        Probe::Actions(actions) => {
            for action in actions {
                window.dispatch_action(action, cx);
            }
            window.on_next_frame(move |window, cx| run(root, view, step + 1, window, cx))
        }
        Probe::KeyUpOnly(key) => {
            window.dispatch_event(PlatformInput::KeyUp(KeyUpEvent { keystroke: key }), cx);
            window.on_next_frame(move |window, cx| run(root, view, step + 1, window, cx))
        }
        Probe::Done => {
            println!("probe done");
            cx.quit();
        }
    }
}

pub fn chord(i: usize, cx: &App) -> Keystroke {
    let (key, shift) = [
        ("n", false),
        ("o", false),
        ("s", false),
        ("s", true),
        ("e", false),
        ("p", false),
    ][i];
    Keystroke {
        key: key.into(),
        modifiers: Modifiers {
            shift,
            ..Modifiers::secondary_key(cx)
        },
        ..Default::default()
    }
}

pub fn shift_p(cx: &App) -> Keystroke {
    Keystroke {
        key: "p".into(),
        modifiers: Modifiers {
            shift: true,
            ..Modifiers::secondary_key(cx)
        },
        ..Default::default()
    }
}

pub fn secondary(key: &str, cx: &App) -> Keystroke {
    Keystroke {
        key: key.into(),
        modifiers: Modifiers::secondary_key(cx),
        ..Default::default()
    }
}

pub fn plain(key: &str) -> Keystroke {
    Keystroke {
        key: key.into(),
        ..Default::default()
    }
}

pub fn type_keys(word: &str) -> Vec<Keystroke> {
    word.chars()
        .map(|letter| Keystroke {
            key: letter.into(),
            key_char: Some(letter.into()),
            ..Default::default()
        })
        .collect()
}

pub fn file_menu(cx: &mut App) -> (String, Vec<String>, Vec<(String, String)>) {
    let menus = cx.get_menus().unwrap_or_default();
    let app = menus.first().map(|menu| menu.name.to_string());
    let mut labels = Vec::new();
    let mut map = Vec::new();
    for menu in menus.iter().skip(1) {
        for item in &menu.items {
            if let OwnedMenuItem::Action { name, action, .. } = item {
                labels.push(name.clone());
                map.push((name.clone(), action.name().to_string()));
            }
        }
    }
    (app.unwrap_or_default(), labels, map)
}

pub fn menu_action(i: usize, cx: &mut App) -> Box<dyn gpui::Action> {
    let actions: Vec<Box<dyn gpui::Action>> = cx
        .get_menus()
        .unwrap_or_default()
        .into_iter()
        .nth(1)
        .map(|menu| {
            menu.items
                .iter()
                .filter_map(|item| match item {
                    OwnedMenuItem::Action { action, .. } => Some(action.boxed_clone()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    let Some(action) = actions.into_iter().nth(i) else {
        panic!("the File menu has a {i}th action");
    };
    action
}

pub fn ran_first(root: &Entity<Root>, cx: &mut App) -> String {
    root.read(cx)
        .last()
        .map(|ran| ran.to_string())
        .unwrap_or_else(|| "none".into())
}
