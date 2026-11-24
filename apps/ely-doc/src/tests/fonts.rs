//! Fonts the kernel ships, and stand-ins for missing ones.

mod roboto;
#[cfg(target_os = "linux")]
mod stand_ins;
