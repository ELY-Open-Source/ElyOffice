#!/usr/bin/env bash
# bench.py on Linux: Xvfb in a container, private AT-SPI bus.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
[ "$(uname -s)" = Linux ] || { echo "SKIP: not Linux"; exit 0; }
for tool in docker dbus-run-session xwininfo; do
  command -v "$tool" >/dev/null || { echo "SKIP: no $tool"; exit 0; }
done
python3 -I -c 'import gi; gi.require_version("Atspi", "2.0")' 2>/dev/null || { echo "SKIP: no AT-SPI for Python"; exit 0; }
launcher=
for at in /usr/libexec/at-spi-bus-launcher /usr/lib/at-spi-bus-launcher /usr/lib/at-spi2-core/at-spi-bus-launcher; do
  [ -x "$at" ] && launcher=$at && break
done
[ -n "$launcher" ] || { echo "SKIP: no at-spi-bus-launcher"; exit 0; }

# An X server with XTest; no desktop's focus rules.
docker build -q -t ely-xvfb - >/dev/null <<'EOF'
FROM ubuntu:26.04@sha256:f144425ff09be612d6d9ad965196e9cdc23dae1f42110a8a11a3e9a8198759f7
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends xvfb \
  && rm -rf /var/lib/apt/lists/*
EOF
xvfb=$(docker run -d --rm --network host ely-xvfb Xvfb :9 -listen tcp -ac -screen 0 1920x1080x24)
trap 'docker stop -t 1 "$xvfb" >/dev/null' EXIT
export DISPLAY=127.0.0.1:9
for _ in $(seq 40); do xwininfo -root >/dev/null 2>&1 && break; sleep 0.25; done
xwininfo -root >/dev/null || { docker logs "$xvfb" >&2; exit 1; }

# A private session bus with accessibility on.
inside='set -euo pipefail
launcher=$1 bench=$2
shift 2
"$launcher" --launch-immediately >/dev/null 2>&1 &
for _ in $(seq 40); do
  python3 -I -c "
from gi.repository import Gio, GLib
Gio.bus_get_sync(Gio.BusType.SESSION).call_sync(\"org.a11y.Bus\", \"/org/a11y/bus\", \"org.freedesktop.DBus.Properties\",
    \"Set\", GLib.Variant(\"(ssv)\", (\"org.a11y.Status\", \"IsEnabled\", GLib.Variant(\"b\", True))), None, 0, -1, None)
" 2>/dev/null && break
  sleep 0.25
done
exec python3 "$bench" "$@"'
unset WAYLAND_DISPLAY
ELY_GIT=${ELY_GIT:-$(command -v git)} dbus-run-session -- bash -c "$inside" _ "$launcher" "$here/bench.py" "$@"
