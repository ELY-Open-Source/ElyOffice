//! LOK shared access: worker process, IPC and client (PRD §7).

mod client;
pub mod commands;
mod events;
mod ffi;
mod handshake;
pub mod history;
mod ipc;
pub mod menu;
pub mod review;
pub mod sheet;
pub mod show;
mod table;
mod worker;

pub use client::{
    A11y, Cell, Doc, Events, Function, MIXED, Mark, Mouse, Pdf, Rect, Request, Sheet, Sheets, Tile,
    Worker, install_dir, is_nothing, is_stale, templates,
};
#[cfg(any(test, feature = "test-support"))]
pub use client::test_install;
#[cfg(feature = "view")]
pub use view::{
    CONTEXT, Copy, Cut, Geom, Layout, LokError, LokView, Paste, Reported, ZOOM_STEPS, notch,
    SCHEME, Zoomed, init, layout, paper, read_off,
};
#[cfg(feature = "view")]
mod view;

/// One tile edge in twips.
pub const TILE_TWIPS: i32 = 3840;
/// Twips per pixel at zoom 1.0 and DPI 1.0.
pub(crate) const TWIPS_PER_PIXEL: f32 = 15.0;
pub use ipc::STALE;
pub use ipc::file_url_of as file_url;
pub use ipc::path_of_file_url as file_path;
pub use worker::run as worker_run;
