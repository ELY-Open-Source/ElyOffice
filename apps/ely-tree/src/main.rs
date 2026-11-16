// Release builds open no console on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod asks;
mod asks_places;
mod blame;
mod branches;
mod changes;
mod cloning;
mod commit;
mod commits;
mod committing;
mod conflict;
mod conflict_pane;
mod detail;
mod diff;
mod git;
mod hist;
mod hist_pane;
mod look;
mod look_pane;
mod menu;
mod moves;
mod network;
mod ops;
mod pane;
mod pick;
mod places;
mod places_pane;
mod recover;
mod recovering;
mod refs_pane;
mod remotes;
mod repo;
mod running;
mod shown;
mod side_log;
mod side_places;
mod side_refs;
mod stash;
mod status;
mod structure;
mod tools;
mod tree;
mod view;
mod watch;
mod work;

#[cfg(test)]
mod tests;

use ely_platform::AppId;

fn main() {
    ely_ui::run(AppId::Tree, |_| {}, tree::open, None);
}
