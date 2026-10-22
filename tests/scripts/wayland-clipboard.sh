#!/usr/bin/env bash
# Clipboard crosses to native Wayland apps (sway, empty X11).
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
command -v docker >/dev/null || {
  echo "SKIP: no docker"
  exit 0
}
built=$("$root/packaging/linux/container.sh" \
  cargo test -p ely-platform --test wayland_clipboard --no-run 2>&1) || {
  echo "$built" >&2
  exit 1
}
test=$(sed -n 's/.*Executable .*(\(.*\)).*/\1/p' <<<"$built" | tail -1)
[ -n "$test" ] || {
  echo "FAIL: no test binary in: $built" >&2
  exit 1
}
docker build -q -t ely-wayland-clipboard - >/dev/null <<'EOF'
FROM ubuntu:26.04@sha256:f144425ff09be612d6d9ad965196e9cdc23dae1f42110a8a11a3e9a8198759f7
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  sway wl-clipboard xvfb && rm -rf /var/lib/apt/lists/*
EOF
inside='set -euo pipefail
export XDG_RUNTIME_DIR=$(mktemp -d)
Xvfb :9 >/tmp/xvfb.log 2>&1 &
printf "xwayland disable\n" >"$XDG_RUNTIME_DIR/sway.conf"
WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 \
  sway -c "$XDG_RUNTIME_DIR/sway.conf" >/tmp/sway.log 2>&1 &
for _ in $(seq 40); do
  socket=$(cd "$XDG_RUNTIME_DIR" && ls wayland-? 2>/dev/null | head -1) || true
  [ -n "$socket" ] && [ -e /tmp/.X11-unix/X9 ] && break
  sleep 0.25
done
[ -n "$socket" ] || { cat /tmp/sway.log; exit 1; }
WAYLAND_DISPLAY=$socket DISPLAY=:9 ELY_WAYLAND_CLIPBOARD_TEST=1 "$1" --nocapture'
docker run --rm --user "$(id -u):$(id -g)" -v "$root:$root" -w "$root" \
  ely-wayland-clipboard bash -c "$inside" _ "$test"
echo "PASS: wayland clipboard"
