// Release builds open no console on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod chrome;
mod compare;
mod docs;
mod files;
mod insert;
mod menu;
mod merge;
mod navigate;
mod output;
mod props;
mod refs;
mod remote;
mod review;

use ely_platform::AppId;

fn main() {
    if let Some(code) = remote::command_line() {
        std::process::exit(code);
    }
    ely_ui::run(AppId::Doc, docs::open);
}

#[cfg(test)]
mod tests;
