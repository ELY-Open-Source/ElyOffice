pub mod commands;
#[cfg(any(test, feature = "test-support"))]
mod ending;
pub mod i18n;
pub mod menu;
mod frames;
mod opening;
mod run;
pub mod settings;
mod shell;
pub mod tasks;
mod theme;
pub mod tray;
pub mod updates;
pub mod watching;
pub mod wheel;

#[cfg(any(test, feature = "test-support"))]
pub use ending::at_thread_end;
pub use opening::{launch_files, on_open};
#[cfg(any(test, feature = "test-support"))]
pub use opening::install_no_launch_files;
pub use run::{run, window_options};
pub use shell::{Root, Shell, blank, document_root, root, set_document_menus, tool_root};
pub use tasks::{LongTask, TaskState};
