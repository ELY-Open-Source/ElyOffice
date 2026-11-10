#!/usr/bin/env bash
# deb on Ubuntu, rpm on Fedora: apps draw under Xvfb.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
dist=${ELY_DIST:-$root/dist}
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
for tool in docker magick tesseract git python3; do
  command -v "$tool" >/dev/null || {
    echo "SKIP: no $tool"
    exit 0
  }
done
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# shellcheck source=tests/scripts/linux-common.sh
. "$root/tests/scripts/linux-common.sh"
linux_inputs "$root" "$work"

# Xvfb, Mesa's software Vulkan and the X11 capture tools.
docker build -q -t ely-smoke-ubuntu - >/dev/null <<'EOF'
FROM ubuntu:26.04@sha256:f144425ff09be612d6d9ad965196e9cdc23dae1f42110a8a11a3e9a8198759f7
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  xvfb mesa-vulkan-drivers x11-apps x11-utils && rm -rf /var/lib/apt/lists/*
EOF
docker build -q -t ely-smoke-fedora - >/dev/null <<'EOF'
FROM fedora:44@sha256:43b29f65a41eb9c35e1cd5323e3bdf3b655c2357a9f4f1ff2f9c2798e5045d80
RUN dnf install -y -q xorg-x11-server-Xvfb mesa-vulkan-drivers xwd xwininfo && dnf clean all
EOF
# As the work dir's owner: Git refuses others' repos.
inside='set -euo pipefail
slug=$1 work=$2 file=$3
shift 3
eval "$INSTALL /dist/$file" >/tmp/install.log 2>&1 || { tail -20 /tmp/install.log; exit 1; }
user=$(getent passwd "$OWNER" | cut -d: -f1) || { useradd -u "$OWNER" -m ely; user=ely; }
home=$(getent passwd "$user" | cut -d: -f6)
Xvfb :9 -screen 0 1280x800x24 >/tmp/xvfb.log 2>&1 &
for _ in $(seq 40); do [ -e /tmp/.X11-unix/X9 ] && break; sleep 0.25; done
runuser -u "$user" -- env DISPLAY=:9 ELY_OMARCHY_DIR="$work/omarchy/current" ELY_SMOKE=hold ELY_LOCALE=en-US \
  HOME="$home" "/usr/bin/$slug" "$@" 2>"$work/$slug.err" &
pid=$!
id=
for _ in $(seq 120); do
  id=$(xwininfo -display :9 -root -children | awk -v class="\"$slug\"" "index(\$0, class) { print \$1; exit }")
  [ -n "$id" ] && break
  kill -0 "$pid" 2>/dev/null || { echo "$slug quit"; exit 1; }
  sleep 0.5
done
[ -n "$id" ] || { echo "$slug showed no window"; exit 1; }
sleep 8
kill -0 "$pid" 2>/dev/null || { echo "$slug quit after showing"; exit 1; }
xwd -display :9 -id "$id" -out "$work/$slug.xwd"
kill "$pid"'
run() { # image, install command, package file, slug, input
  local slug=$4 shot=$work/$4.png args=()
  [ -f "$dist/$3" ] || fail "no $dist/$3; run packaging/linux/package.sh $slug"
  [ -z "$5" ] || args=("$work/$5")
  docker run --rm -e INSTALL="$2" -e OWNER="$(id -u)" -v "$dist:/dist:ro" -v "$work:$work" "$1" \
    bash -c "$inside" _ "$slug" "$work" "$3" "${args[@]}" ||
    fail "$3 does not run in $1: $(tail -n 5 "$work/$slug.err" 2>/dev/null)"
  magick "$work/$slug.xwd" "$shot"
  judge "$slug" "$shot" "$6" "$work/$slug.err"
  echo "linux: $3 shows '$6' in Catppuccin on ${1#ely-smoke-}"
}
for case in "${cases[@]}"; do
  IFS='|' read -r slug words input <<<"$case"
  run ely-smoke-ubuntu "apt-get update -q && apt-get install -y -q" "${slug}_$version-1_amd64.deb" "$slug" "$input" "$words"
  run ely-smoke-fedora "dnf install -y -q" "$slug-$version-1.x86_64.rpm" "$slug" "$input" "$words"
  # The suite's app, on the kernel all three share.
  run ely-smoke-fedora "dnf install -y -q" "ely-office-$version-1.x86_64.rpm" "$slug" "$input" "$words"
done
