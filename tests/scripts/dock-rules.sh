#!/usr/bin/env bash
# dock-macos.sh on fake apps: what passes, what fails.
set -euo pipefail
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'pkill -f "$work/" 2>/dev/null || true; rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
mkdir -p "$work/bin" "$work/real"
# LaunchServices by process name, from $work/types.
cat >"$work/bin/lsappinfo" <<'EOF'
#!/bin/bash
name() { basename "$(ps -o comm= -p "$1" 2>/dev/null)"; }
type_of() { awk -v n="$1" '$1 == n { print $2 }' "$DOCK_TYPES"; }
case $1 in
  find)
    pid=${2#pid=}
    [ -z "$(type_of "$(name "$pid")")" ] || echo "ASN:$pid"
    ;;
  info)
    seen=$(type_of "$(name "${4#ASN:}")")
    [ "$seen" != FAIL ] || exit 2
    echo " type=\"$seen\""
    ;;
esac
EOF
chmod +x "$work/bin/lsappinfo"
# Real processes named as the checks expect, alive an hour.
printf '#include <unistd.h>\nint main(void) { sleep(3600); return 0; }\n' >"$work/sleeper.c"
cc -o "$work/sleeper" "$work/sleeper.c"
for name in ely-doc ely-mail ely-probe ely-lok-worker other-helper; do cp "$work/sleeper" "$work/real/$name"; done
app() { # Name, slug, worker (yes/no), logged words, the script's end
  local folder=$work/apps/$1.app/Contents/MacOS log="\$HOME/Library/Application Support/ElyOffice/$2/ely.log"
  mkdir -p "$folder"
  [ "$3" = no ] || cp "$work/real/ely-lok-worker" "$folder/ely-lok-worker"
  {
    echo '#!/bin/bash'
    echo "mkdir -p \"\$(dirname \"$log\")\""
    echo "\"$work/real/ely-lok-worker\" &"
    echo "\"$work/real/other-helper\" &"
    echo "echo \"$4\" >>\"$log\""
    echo "$5"
  } >"$folder/$2"
  chmod +x "$folder/$2"
  echo "$work/apps/$1.app"
}
run() { # types, app; sets said and took; a hang fails
  local start=$SECONDS code=0 job
  printf '%s\n' "$1" >"$work/types"
  HOME=$work/home PATH="$work/bin:$PATH" DOCK_TYPES=$work/types ELY_DOCK_RUNS=1 \
    bash "$root/tests/scripts/dock-macos.sh" "$2" >"$work/said" 2>&1 &
  job=$!
  while kill -0 "$job" 2>/dev/null && [ $((SECONDS - start)) -lt 90 ]; do sleep 1; done
  if kill -0 "$job" 2>/dev/null; then
    pkill -P "$job" || true
    kill "$job"
    wait "$job" 2>/dev/null || true
    echo "still running after 90 s" >>"$work/said"
    code=124
  else
    wait "$job" || code=$?
  fi
  said=$(cat "$work/said")
  took=$((SECONDS - start))
  return "$code"
}
ready="window open lok: tile (0, 0) shown lok: the worker is an accessory app"
mail=$(app ElyMail ely-mail no "$ready" "exec \"$work/real/ely-mail\"")
doc=$(app ElyDoc ely-doc yes "$ready" "exec \"$work/real/ely-doc\"")
crash=$(app ElyMailCrash ely-mail no "$ready" "sleep 0.7; exit 42")
mute=$(app ElyMailMute ely-mail no "starting" "exec \"$work/real/ely-mail\"")
probe=$(app ElyProbe ely-probe no "$ready" "exec \"$work/real/ely-probe\"")

run $'ely-mail Foreground' "$mail" || fail "a well app failed: $said"
run $'ely-doc Foreground\nely-lok-worker UIElement' "$doc" || fail "a well kernel app failed: $said"
! run $'ely-doc Foreground\nely-lok-worker Background\nother-helper UIElement' "$doc" || fail "another helper's UIElement passed for the worker: $said"
[[ $said == *"the worker was never UIElement"* ]] || fail "mixed: $said"
! run $'ely-mail Foreground' "$crash" || fail "an app that quit early passed: $said"
[[ $said == *"quit (exit 42)"* ]] || fail "crash: $said"
# A live app is ended, not waited for.
! run $'ely-mail Foreground\nother-helper FAIL' "$mail" || fail "a failed LaunchServices query passed: $said"
[[ $said == *"cannot read"* ]] && [ "$took" -lt 20 ] || fail "query, $took s: $said"
! run $'ely-mail Foreground' "$mute" || fail "an app never ready passed: $said"
[[ $said == *"never logged"* ]] && [ "$took" -lt 75 ] || fail "never ready, $took s: $said"
! run $'ely-probe Foreground' "$probe" || fail "an app with no case passed: $said"
[[ $said == *"no Dock case for ely-probe"* ]] || fail "unknown: $said"
echo "PASS: dock-macos.sh fails a mixed worker type, an early quit, a failed query, an app never ready and an unknown app, in bounded time"
