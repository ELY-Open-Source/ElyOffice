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
# Real processes with the names the checks look for.
printf '#include <unistd.h>\nint main(void) { sleep(60); return 0; }\n' >"$work/sleeper.c"
cc -o "$work/sleeper" "$work/sleeper.c"
for name in ely-doc ely-mail ely-probe ely-lok-worker other-helper; do cp "$work/sleeper" "$work/real/$name"; done
app() { # Name, slug, worker (yes/no), then the script's end
  local folder=$work/apps/$1.app/Contents/MacOS log="\$HOME/Library/Application Support/ElyOffice/$2/ely.log"
  mkdir -p "$folder"
  [ "$3" = no ] || cp "$work/real/ely-lok-worker" "$folder/ely-lok-worker"
  printf '#!/bin/bash\nmkdir -p "$(dirname "%s")"\n"%s/real/ely-lok-worker" 60 &\n"%s/real/other-helper" 60 &\necho "%s window open lok: tile (0, 0) shown lok: the worker is an accessory app" >>"%s"\n%s\n' \
    "$log" "$work" "$work" "$1" "$log" "$4" >"$folder/$2"
  chmod +x "$folder/$2"
  echo "$work/apps/$1.app"
}
run() { # types, app; sets said
  printf '%s\n' "$1" >"$work/types"
  said=$(HOME=$work/home PATH="$work/bin:$PATH" DOCK_TYPES=$work/types ELY_DOCK_RUNS=1 bash "$root/tests/scripts/dock-macos.sh" "$2" 2>&1)
}
mail=$(app ElyMail ely-mail no "exec \"$work/real/ely-mail\" 60")
doc=$(app ElyDoc ely-doc yes "exec \"$work/real/ely-doc\" 60")
crash=$(app ElyMailCrash ely-mail no "sleep 0.7; exit 42")
probe=$(app ElyProbe ely-probe no "exec \"$work/real/ely-probe\" 60")

run $'ely-mail Foreground' "$mail" || fail "a well app failed: $said"
run $'ely-doc Foreground\nely-lok-worker UIElement' "$doc" || fail "a well kernel app failed: $said"
! run $'ely-doc Foreground\nely-lok-worker Background\nother-helper UIElement' "$doc" || fail "another helper's UIElement passed for the worker: $said"
[[ $said == *"the worker was never UIElement"* ]] || fail "mixed: $said"
! run $'ely-mail Foreground' "$crash" || fail "an app that quit early passed: $said"
[[ $said == *"quit (exit 42)"* ]] || fail "crash: $said"
! run $'ely-mail Foreground\nother-helper FAIL' "$mail" || fail "a failed LaunchServices query passed: $said"
[[ $said == *"cannot read"* ]] || fail "query: $said"
! run $'ely-probe Foreground' "$probe" || fail "an app with no case passed: $said"
[[ $said == *"no Dock case for ely-probe"* ]] || fail "unknown: $said"
echo "PASS: dock-macos.sh fails a mixed-up worker type, an early quit, a failed query and an unknown app"
