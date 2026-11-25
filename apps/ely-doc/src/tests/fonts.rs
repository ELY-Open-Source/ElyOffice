//! Fonts the kernel ships, and stand-ins for missing ones.

mod free_fonts;
mod office;
#[cfg(target_os = "linux")]
mod stand_ins;
