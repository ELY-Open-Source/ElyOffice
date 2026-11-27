//! The Slide Show menu and a running show's keys.

use ely_gpui_component::{primitives::IconName, shell::StatusBarItem};
use ely_ui::commands;
use gpui::{
    App, Context, Div, InteractiveElement as _, KeyBinding, Keystroke, Menu, MenuItem, actions,
};

use super::Show;
use crate::deck::Deck;

actions!(show, [StartShow, StartShowHere, CheckShow, LiveScoreboard]);

/// Slide numbers past this many digits are not typed.
const DIGITS: usize = 6;

/// The Slide Show menu.
pub fn menu(cx: &App) -> Menu {
    let label = |key: &str| Deck::text(cx, key, &[]);
    Menu::new(label("menu.play")).items([
        MenuItem::action(label("show.play.from_start"), StartShow),
        MenuItem::action(label("show.play.from_here"), StartShowHere),
        MenuItem::separator(),
        MenuItem::action(label("show.score.menu"), LiveScoreboard),
        MenuItem::action(label("show.check.menu"), CheckShow),
    ])
}

/// PowerPoint's status bar button: show from this slide.
pub fn present_button(cx: &mut Context<Deck>) -> StatusBarItem {
    let deck = cx.entity();
    StatusBarItem::new("show-present")
        .icon(IconName::Play)
        .label(Deck::text(cx, "show.play.present", &[]))
        .tooltip(Deck::text(cx, "show.play.from_here", &[]))
        .on_click(move |_, _, cx| deck.update(cx, |deck, cx| deck.start_show(true, cx)))
}

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("f5", StartShow, Some(commands::CONTEXT)),
        KeyBinding::new("shift-f5", StartShowHere, Some(commands::CONTEXT)),
    ]);
}

pub fn bind(root: Div, cx: &mut Context<Deck>) -> Div {
    root.on_action(cx.listener(|deck, _: &StartShow, _, cx| deck.start_show(false, cx)))
        .on_action(cx.listener(|deck, _: &StartShowHere, _, cx| deck.start_show(true, cx)))
        .on_action(cx.listener(|deck, _: &CheckShow, _, cx| deck.check_show(cx)))
        .on_action(cx.listener(|deck, _: &LiveScoreboard, _, cx| deck.ask_scoreboard(cx)))
}

impl Show {
    /// Show keys; true when taken.
    pub fn key(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) -> bool {
        let held = &keystroke.modifiers;
        if held.platform || held.control || held.alt {
            return false;
        }
        match keystroke.key.as_str() {
            "right" | "down" | "space" | "pagedown" | "n" => self.next(cx),
            "enter" if self.typed.is_empty() => self.next(cx),
            "enter" => {
                let number = self.typed.parse().expect("typed digits only");
                self.jump(number, cx);
            }
            "left" | "up" | "pageup" | "backspace" | "p" => self.previous(cx),
            "home" => self.jump(1, cx),
            "end" => {
                let last = (0..self.slides.len()).rev().find(|at| !self.slides[*at].hidden);
                self.jump(last.map_or(0, |at| at + 1), cx);
            }
            "b" | "." => self.toggle_black(cx),
            "l" => self.toggle_laser(cx),
            "s" => self.cycle_scores(cx),
            "escape" if !self.typed.is_empty() => {
                self.typed.clear();
                cx.notify();
            }
            "escape" => self.close(cx),
            digit if digit.len() == 1 && digit.chars().all(|c| c.is_ascii_digit()) => {
                if self.typed.len() < DIGITS {
                    self.typed.push_str(digit);
                    cx.notify();
                }
            }
            _ => return false,
        }
        true
    }
}
