#!/usr/bin/env bash
# dock-macos.sh: each app is one Dock icon; helpers never are.
set -euo pipefail
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
root=$(cd "$(dirname "$0")/../.." && pwd)
runs=${ELY_DOCK_RUNS:-5}
apps=("$@")
# Every first-batch bundle in dist, and one unbundled dev build.
if [ ${#apps[@]} -eq 0 ]; then
  while IFS=$'\t' read -r _ name _ batch _; do
    [ "$batch" = 1 ] && apps+=("$root/dist/$name.app")
  done <"$root/packaging/apps.tsv"
  [ ${#apps[@]} -gt 0 ] || { echo "FAIL: apps.tsv lists no first-batch app"; exit 1; }
  apps+=("$root/target/debug/ely-doc")
fi

# Descendants of a pid, at any depth.
family() {
  local kid
  for kid in $(pgrep -P "$1" || true); do
    echo "$kid"
    family "$kid"
  done
}
# LaunchServices' type for a pid, if it registered.
kind() {
  local asn
  asn=$(lsappinfo find "pid=$1" || true)
  [ -z "$asn" ] || lsappinfo info -only applicationtype "$asn" | grep -o 'type="[A-Za-z]*"' || true
}

# One launch, sampled until ready plus three seconds.
launch() { # bin, log, whether it runs the LOK worker
  local bin=$1 log=$2 lok=$3 start pid worker="" own="" helpers="" done_at=0 child seen name
  start=$(( $( [ -f "$log" ] && wc -l < "$log" || echo 0 ) + 1 ))
  ELY_NO_DIALOG=1 ELY_SMOKE=hold ELY_LOG=debug "$bin" >/dev/null 2>&1 &
  pid=$!
  local end=$((SECONDS + 60)) ready='window open'
  [ "$lok" = 0 ] || ready='lok: tile ('
  while [ $SECONDS -lt $end ] && kill -0 "$pid" 2>/dev/null; do
    own+=" $(kind "$pid")"
    for child in $(family "$pid"); do
      seen=$(kind "$child")
      [ -n "$seen" ] || continue
      name=$(ps -o comm= -p "$child" 2>/dev/null || true)
      helpers+=" ${name##*/}=$seen"
    done
    [ "$lok" = 0 ] || [ -n "$worker" ] || worker=$(pgrep -P "$pid" -x ely-lok-worker || true)
    if [ "$done_at" = 0 ] && tail -n +"$start" "$log" 2>/dev/null | grep -q "$ready"; then done_at=$((SECONDS + 3)); fi
    [ "$done_at" = 0 ] || [ $SECONDS -lt "$done_at" ] || break
  done
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  echo "$(basename "$bin"): own$(tr ' ' '\n' <<<"$own" | grep . | sort | uniq -c | tr -s ' \n' ' '), helpers:$(tr ' ' '\n' <<<"$helpers" | grep . | sort | uniq -c | tr -s ' \n' ' ')"
  [ "$done_at" != 0 ] || { echo "FAIL: never logged '$ready'"; return 1; }
  case $own in *Foreground*) ;; *) echo "FAIL: the app itself was never a Dock app"; return 1 ;; esac
  case $helpers in *Foreground*) echo "FAIL: a helper was a Dock app"; return 1 ;; esac
  [ "$lok" = 1 ] || return 0
  [ -n "$worker" ] || { echo "FAIL: no worker started"; return 1; }
  case $helpers in *ely-lok-worker=*UIElement*) ;; *) echo "FAIL: the worker's type was never seen"; return 1 ;; esac
  tail -n +"$start" "$log" | grep -q 'lok: the worker is an accessory app' || { echo "FAIL: no accessory log"; return 1; }
}

for app in "${apps[@]}"; do
  if [ -f "$app" ]; then
    bin=$app
    folder=$(dirname "$app")
  else
    [ -d "$app" ] || { echo "FAIL: $app is missing (packaging/macos/bundle.sh)"; exit 1; }
    folder=$app/Contents/MacOS
    bin=$(find "$folder" -maxdepth 1 -type f -name 'ely-*' ! -name 'ely-lok-worker' ! -name 'ely-pdf-worker' | head -1)
    [ -n "$bin" ] || { echo "FAIL: $app has no app binary"; exit 1; }
  fi
  # The kernel apps ship the worker beside them.
  lok=0
  case $(basename "$bin") in ely-doc | ely-table | ely-show) lok=1 ;; esac
  [ "$lok" = 0 ] || [ -x "$folder/ely-lok-worker" ] || { echo "FAIL: no worker beside $bin (cargo build -p ely-lok --bins)"; exit 1; }
  log="$HOME/Library/Application Support/ElyOffice/$(basename "$bin")/ely.log"
  for _ in $(seq 1 "$runs"); do launch "$bin" "$log" "$lok"; done
done
echo "PASS: ${#apps[@]} apps, each one Dock icon; no helper registered"
