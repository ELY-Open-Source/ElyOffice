// Release builds open no console on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod analysis;
mod bars;
mod book;
mod cells;
mod data;
mod dialog;
mod figures;
mod format;
mod formulas;
mod grid;
mod menu;
mod output;
mod page;
mod protect;
mod remote;
mod rules;
mod sheets;
mod template;
mod text;

use ely_platform::AppId;

fn main() {
    if let Some(code) = remote::command_line() {
        std::process::exit(code);
    }
    ely_ui::run(AppId::Table, book::open);
}

#[cfg(test)]
mod tests;
