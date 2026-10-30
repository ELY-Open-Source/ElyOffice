#!/usr/bin/env bash
# Seven apps name every control the AX tree shows.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: not macOS"; exit 0; }
[ -n "${ELY_LOK_DIR:-}" ] || [ -d "$root/engines/lo-src/instdir" ] || { echo "SKIP: no LOK build"; exit 0; }
[ -n "${ELY_MUPDF_DIR:-}" ] || [ -d "$root/engines/mupdf-build" ] || { echo "SKIP: no MuPDF build"; exit 0; }

work=$root/target/a11y-names-macos
bin=${CARGO_TARGET_DIR:-$root/target}/debug
pid=
stop() { [ -n "$pid" ] && { pkill -9 -P "$pid" 2>/dev/null || true; kill -9 "$pid" 2>/dev/null || true; }; pid=; }
trap stop EXIT
rm -rf "$work"
mkdir -p "$work"
(cd "$root" && cargo build -q -p ely-doc -p ely-table -p ely-show -p ely-pdf -p "path+file://$root/apps/ely-mail" \
  -p ely-tree -p ely-mark -p ely-lok -p ely-pdf-worker --bins)
for tool in ax-names ax-values; do swiftc -O -o "$work/$tool" "$root/tests/scripts/$tool.swift"; done
[ "$("$work/ax-values" $$)" != untrusted ] || { echo "SKIP: needs Accessibility permission"; exit 0; }
. "$root/tests/scripts/a11y-samples.sh"
a11y_samples "$work"

failed=0
while IFS='|' read -r app sample page; do
  mkdir -p "$work/data-$app"
  set -- "$bin/$app"
  [ "$sample" = - ] || set -- "$@" "$work/$sample"
  # The page names are English.
  ELY_DATA_ROOT=$work/data-$app ELY_NO_DIALOG=1 ELY_LOCALE=en-US "$@" >"$work/$app.out" 2>&1 </dev/null &
  pid=$!
  log=
  for _ in $(seq 120); do
    kill -0 "$pid" 2>/dev/null || { log="error $app exited: $(tail -n 3 "$work/$app.out")"; break; }
    grep -q ' window open$' "$work/$app.out" && break
    sleep 0.5
  done
  if [ -z "$log" ]; then
    # A reader that joins an idle app, as VoiceOver may.
    sleep 10
    log=$("$work/ax-names" "$pid" "$page")
  fi
  stop
  printf '%s\n' "$log" >"$work/$app.log"
  stops=$(printf '%s\n' "$log" | sed -n 's/^stops //p')
  # Window buttons are native here; empty ElyMail has two stops.
  if printf '%s\n' "$log" | grep -qE '^(unnamed|nameless|unreached|no page|no focus|trapped|error)' || [ "${stops:-0}" -lt 2 ]; then
    echo "FAIL: $app" >&2
    printf '%s\n' "$log" >&2
    failed=1
  else
    echo "ok: $app, $stops Tab stops"
  fi
done < <(a11y_apps)
[ "$failed" = 0 ] || exit 1
echo "PASS: seven apps name every control and every Tab stop; Tab reaches each document, F6 leaves it"
