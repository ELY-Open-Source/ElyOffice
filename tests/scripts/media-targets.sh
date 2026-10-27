#!/usr/bin/env bash
# The media player interface compiles for Linux and Windows.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
targets=(x86_64-unknown-linux-gnu x86_64-pc-windows-msvc)
installed=$(cd "$root" && rustup target list --installed)
for target in "${targets[@]}"; do
  if ! grep -qx "$target" <<<"$installed"; then
    echo "SKIP: media-targets needs rustup target $target"
    exit 0
  fi
done
native() { if command -v cygpath >/dev/null; then cygpath -m "$1"; else printf '%s' "$1"; fi; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/src"
cp "$root/rust-toolchain.toml" "$work/"
cat >"$work/Cargo.toml" <<TOML
[package]
name = "media-targets"
version = "0.1.0"
edition = "2024"
[dependencies]
anyhow = "1"
log = "0.4"
[target.'cfg(target_os = "linux")'.dependencies]
libc = "0.2"
[target.'cfg(windows)'.dependencies]
dunce = "1"
windows-core = "0.62"
windows = { version = "0.62", features = ["Win32_Foundation", "Win32_Graphics_Dxgi_Common", "Win32_Graphics_Imaging", "Win32_Media_MediaFoundation", "Win32_System_Com"] }
TOML
cat >"$work/src/lib.rs" <<RS
#[path = "$(native "$root")/crates/ely-platform/src/media.rs"]
pub mod media;
// Every call ElyShow makes.
pub fn touch(path: &std::path::Path) -> anyhow::Result<Option<media::Frame>> {
    let player = media::Player::open(path)?;
    let _: media::State = player.state();
    let (_, _): (bool, f64) = (player.playing(), player.position());
    player.play();
    player.pause();
    player.seek(0.0);
    Ok(player.frame())
}
RS
for target in "${targets[@]}"; do
  (cd "$work" && cargo check --quiet --target "$target") \
    || { echo "FAIL: media does not compile for $target"; exit 1; }
done
echo "media-targets: the player interface compiles for ${targets[*]}"
