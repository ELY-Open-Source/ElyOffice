#!/usr/bin/env bash
# The media player compiles for every installed foreign target.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
# One per platform; msvc needs no mingw dlltool to check.
targets=(x86_64-unknown-linux-gnu x86_64-pc-windows-msvc aarch64-apple-darwin)
host=$(cd "$root" && rustc -vV | sed -n 's/^host: //p')
os() { case $1 in *linux*) echo linux ;; *windows*) echo windows ;; *apple*) echo macos ;; esac; }
installed=$(cd "$root" && rustup target list --installed)
checked=() missing=()
for target in "${targets[@]}"; do
  [ "$(os "$target")" = "$(os "$host")" ] && continue
  grep -qx "$target" <<<"$installed" && checked+=("$target") || missing+=("$target")
done
[ ${#missing[@]} -eq 0 ] || echo "SKIP: media-targets has no rustup target ${missing[*]}"
[ ${#checked[@]} -gt 0 ] || { echo "FAIL: media-targets has no foreign target to check"; exit 1; }
native() { if command -v cygpath >/dev/null; then cygpath -m "$1"; else printf '%s' "$1"; fi; }
# Under the repo's target, never the system temp folder.
work=$root/target/media-targets
mkdir -p "$work/src"
cp "$root/rust-toolchain.toml" "$work/"
python3 - "$root" "$work/Cargo.toml" <<'PY'
"""The probe's manifest, from ely-platform's own dependency sections."""
import json, re, sys, tomllib
from pathlib import Path
root, out = Path(sys.argv[1]), Path(sys.argv[2])
shared = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["dependencies"]
member = tomllib.loads((root / "crates/ely-platform/Cargo.toml").read_text())
media = root / "crates/ely-platform/src"
sources = [media / "media.rs", *sorted((media / "media").glob("*.rs"))]
named = set(re.findall(r"\b([a-z][a-z0-9_]*)::", "".join(p.read_text() for p in sources)))
def resolved(name, spec):
    spec = {"version": spec} if isinstance(spec, str) else dict(spec)
    if spec.pop("workspace", False):
        base = shared[name]
        base = {"version": base} if isinstance(base, str) else dict(base)
        base["features"] = base.get("features", []) + spec.pop("features", [])
        spec = {**base, **spec}
    return spec
def line(name, spec):
    return f"{name} = {{ " + ", ".join(f"{k} = {json.dumps(v)}" for k, v in resolved(name, spec).items()) + " }"
lines = ['[package]', 'name = "media-targets"', 'version = "0.1.0"', 'edition = "2024"', '[workspace]', '[dependencies]']
# Shared dependencies only where the player names them.
lines += [line(n, s) for n, s in member["dependencies"].items() if n.replace("-", "_") in named]
for cfg, section in member.get("target", {}).items():
    lines.append(f"[target.'{cfg}'.dependencies]")
    lines += [line(n, s) for n, s in section.get("dependencies", {}).items()]
out.write_text("\n".join(lines) + "\n")
PY
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
for target in "${checked[@]}"; do
  (cd "$work" && cargo check --quiet --target "$target") \
    || { echo "FAIL: media does not compile for $target"; exit 1; }
done
echo "media-targets: the player interface compiles for ${checked[*]}"
