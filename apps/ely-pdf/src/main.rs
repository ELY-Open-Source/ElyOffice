// Release builds open no console on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod annotate;
mod ask;
mod attach;
mod backdrop;
mod content;
mod document;
mod editing;
mod edits;
mod engine;
mod keys;
mod layout;
mod links;
mod marking;
mod marks;
mod notelist;
mod notes;
mod ocr;
mod organize;
mod output;
mod pages;
mod props;
mod reading;
mod protect;
mod redact;
mod search;
mod select;
mod side;
mod sign;
mod view;
mod visible;
mod viewer;
mod writes;

use ely_platform::AppId;

fn main() {
    ely_ui::run(AppId::Pdf, viewer::open);
}

#[cfg(test)]
mod tests;
