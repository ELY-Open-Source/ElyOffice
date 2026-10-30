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
# LaunchServices' type for a pid; empty if unregistered or gone.
kind() {
  local asn info
  asn=$(lsappinfo find "pid=$1") || { echo "lsappinfo cannot look up pid $1"; return 1; }
  [ -n "$asn" ] || return 0
  info=$(lsappinfo info -only applicationtype "$asn") || { echo "lsappinfo cannot read $asn"; return 1; }
  grep -o 'type="[A-Za-z]*"' <<<"$info" && return 0
  # Only an exited process may lose its record.
  ! kill -0 "$1" 2>/dev/null || { echo "no type for live pid $1 ($asn)"; return 1; }
}

# One launch, sampled until ready plus three seconds.
launch() { # bin, log, whether it runs the LOK worker
  local bin=$1 log=$2 lok=$3 start pid worker="" own="" helpers="" types="" done_at=0 child seen name broken="" alive=0 code
  start=$(( $( [ -f "$log" ] && wc -l < "$log" || echo 0 ) + 1 ))
  ELY_NO_DIALOG=1 ELY_SMOKE=hold ELY_LOG=debug "$bin" >/dev/null 2>&1 &
  pid=$!
  local end=$((SECONDS + 60)) ready='window open'
  [ "$lok" = 0 ] || ready='lok: tile ('
  while [ $SECONDS -lt $end ] && kill -0 "$pid" 2>/dev/null; do
    seen=$(kind "$pid") || { broken=$seen; break; }
    own+=" $seen"
    [ "$lok" = 0 ] || [ -n "$worker" ] || worker=$(pgrep -P "$pid" -x ely-lok-worker || true)
    for child in $(family "$pid"); do
      seen=$(kind "$child") || { broken=$seen; break 2; }
      [ -n "$seen" ] || continue
      name=$(ps -o comm= -p "$child" 2>/dev/null || true)
      helpers+=" ${name##*/}=$seen"
      [ "$child" != "$worker" ] || types+=" $seen"
    done
    if [ "$done_at" = 0 ] && tail -n +"$start" "$log" 2>/dev/null | grep -q "$ready"; then done_at=$((SECONDS + 3)); fi
    [ "$done_at" = 0 ] || [ $SECONDS -lt "$done_at" ] || { alive=1; break; }
  done
  if [ "$alive" = 1 ] && kill -0 "$pid" 2>/dev/null; then
    kill "$pid"
    wait "$pid" 2>/dev/null || true
  else
    alive=0
    code=0
    wait "$pid" 2>/dev/null || code=$?
  fi
  echo "$(basename "$bin"): own$(tr ' ' '\n' <<<"$own" | grep . | sort | uniq -c | tr -s ' \n' ' '), helpers:$(tr ' ' '\n' <<<"$helpers" | grep . | sort | uniq -c | tr -s ' \n' ' ')"
  [ -z "$broken" ] || { echo "FAIL: $broken"; return 1; }
  [ "$done_at" != 0 ] || { echo "FAIL: never logged '$ready'"; return 1; }
  [ "$alive" = 1 ] || { echo "FAIL: the app quit (exit ${code:-?}) before three seconds passed"; return 1; }
  case $own in *Foreground*) ;; *) echo "FAIL: the app itself was never a Dock app"; return 1 ;; esac
  case $helpers in *Foreground*) echo "FAIL: a helper was a Dock app"; return 1 ;; esac
  [ "$lok" = 1 ] || return 0
  [ -n "$worker" ] || { echo "FAIL: no worker started"; return 1; }
  case $types in *UIElement*) ;; *) echo "FAIL: the worker was never UIElement:$types"; return 1 ;; esac
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
  case $(basename "$bin") in
    ely-doc | ely-table | ely-show) lok=1 ;;
    ely-pdf | ely-mail | ely-tree | ely-mark) lok=0 ;;
    *) echo "FAIL: no Dock case for $(basename "$bin")"; exit 1 ;;
  esac
  [ "$lok" = 0 ] || [ -x "$folder/ely-lok-worker" ] || { echo "FAIL: no worker beside $bin (cargo build -p ely-lok --bins)"; exit 1; }
  log="$HOME/Library/Application Support/ElyOffice/$(basename "$bin")/ely.log"
  for _ in $(seq 1 "$runs"); do launch "$bin" "$log" "$lok"; done
done
echo "PASS: ${#apps[@]} apps, each one Dock icon; no helper registered"
