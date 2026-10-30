#!/usr/bin/env bash
# README's Linux packages build Ely; doctor.sh sees them.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
command -v docker >/dev/null || {
  echo "SKIP: no docker"
  exit 0
}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# The working tree as committed or staged, no build output.
git -C "$root" ls-files -z | tar -C "$root" --null -T - -cf "$work/src.tar"

# Doctor before and after README's line, then build.
inside='set -euo pipefail
prepare=$1 install=$2
eval "$prepare" >/tmp/prepare.log 2>&1 || { tail -20 /tmp/prepare.log; exit 1; }
mkdir /src && tar -xf /in/src.tar -C /src && cd /src
# As packaging/linux/Dockerfile pins rustup.
curl -fsSL -o /tmp/rustup-init https://static.rust-lang.org/rustup/archive/1.29.2/x86_64-unknown-linux-gnu/rustup-init
echo "5be191e8c70a5007074254cb74d78dbfdd25cd5a61c27de7b8e5410122ec24f4  /tmp/rustup-init" | sha256sum -c - >/dev/null
chmod +x /tmp/rustup-init && /tmp/rustup-init -y -q --profile minimal --default-toolchain none >/dev/null 2>&1
. "$HOME/.cargo/env"
ours="cc|pkg-config|dbus-1|fontconfig|xcb|xkbcommon|xkbcommon-x11"
before=$(bash scripts/doctor.sh || true)
grep -Eq "^MISSING dbus-1 " <<<"$before" || { echo "doctor.sh missed the absent dbus-1:"; echo "$before"; exit 1; }
eval "$install" >/tmp/install.log 2>&1 || { tail -20 /tmp/install.log; exit 1; }
after=$(bash scripts/doctor.sh || true)
if grep -E "^(MISSING|BROKEN) +($ours)( |:|$)" <<<"$after"; then exit 1; fi
cargo test -q -p ely-mail-engine --no-run >/tmp/mail.log 2>&1 || { tail -30 /tmp/mail.log; exit 1; }
cargo build -q --workspace --exclude ely-pdf-worker --bins --tests >/tmp/build.log 2>&1 || { tail -30 /tmp/build.log; exit 1; }
echo "README packages build Ely"'

# README's line for a package manager, made non-interactive.
line() { # command prefix, flag to add
  local found
  found=$(grep -E "^sudo $1 " "$root/README.md") || fail "README has no 'sudo $1' line"
  echo "${found/#sudo $1/$1 $2}"
}
run() { # image, preparation, install command
  echo "== $1"
  docker run --rm -v "$work:/in:ro" "$1" bash -c "$inside" _ "$2" "$3" || fail "README's packages do not build Ely in $1"
}
ubuntu=ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852
fedora=fedora:44@sha256:43b29f65a41eb9c35e1cd5323e3bdf3b655c2357a9f4f1ff2f9c2798e5045d80
arch=archlinux@sha256:4e77cf2ea5f410e6f8be5abf93ccf17ce2436e87a138c167208356711a405dbd
run "$ubuntu" "apt-get update -q && DEBIAN_FRONTEND=noninteractive apt-get install -y -q curl ca-certificates" \
  "DEBIAN_FRONTEND=noninteractive $(line 'apt install' -y)"
run "$fedora" "true" "$(line 'dnf install' -y)"
run "$arch" "pacman -Sy --noconfirm" "$(line 'pacman -S' --noconfirm)"
echo "PASS: README's Linux packages build Ely; doctor.sh checks them"
