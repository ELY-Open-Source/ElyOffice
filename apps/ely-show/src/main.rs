// Release builds open no console on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod chart;
mod check;
mod deck;
mod design;
mod dialog;
mod extra;
mod guides;
mod insert;
mod list;
mod media;
mod menu;
mod motion;
mod new;
mod notes;
mod object;
mod output;
mod preview;
mod remarks;
mod remote;
mod slides;
mod slideshow;
mod table;
mod text;

use ely_platform::AppId;

fn main() {
    if let Some(code) = remote::command_line() {
        std::process::exit(code);
    }
    ely_ui::run(AppId::Show, deck::open);
}

#[cfg(test)]
mod tests;
