// Release builds open no console on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod attach;
mod autosave;
mod blocks;
mod clip;
mod compare;
mod count;
mod desk;
mod engine;
mod export;
mod finding;
mod folder;
mod keys;
mod look;
mod output;
mod outside;
mod paths;
mod places;
mod remote;
mod rename;
mod restore;
mod side;
mod text;
mod tools;
mod vcs;
mod view;

use ely_platform::AppId;

fn main() {
    if let Some(code) = remote::command_line() {
        std::process::exit(code);
    }
    ely_ui::run(AppId::Mark, desk::open);
}
