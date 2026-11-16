// Release builds open no console on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ely_platform::AppId;

fn main() {
    ely_ui::run(AppId::Agent, |_| {}, ely_ui::blank, None);
}
