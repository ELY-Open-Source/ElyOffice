#!/usr/bin/env bash
# The shell gallery opens, switches pages, keeps focus, and quits.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

cargo build -q -p ely-ui --example shell --manifest-path "$root/Cargo.toml"
out=$(ELY_SMOKE=1 HOME="$work/smoke" XDG_DATA_HOME="$work/smoke/data" perl -e 'alarm 60; exec @ARGV' "$root/target/debug/examples/shell" 2>"$work/smoke.err") ||
  fail "the gallery did not exit cleanly: $(tail -n 5 "$work/smoke.err")"
[ "$out" = "ready ElyDoc" ] || fail "the gallery printed '$out', expected 'ready ElyDoc'"

run_gallery() {
  ELY_SMOKE=hold ELY_GALLERY_PROBE=1 ELY_PROBE_HOLD_AT=${3:-} ELY_APPEARANCE=dark ELY_LOCALE=${4:-en} HOME="$work/$1" XDG_DATA_HOME="$work/$1/data" \
    perl -e 'alarm 600; exec @ARGV' "$root/target/debug/examples/shell" >/dev/null 2>"$work/$1.err" &
  pid=$!
}
wait_line() {
  for _ in $(seq $(($3 * 5))); do
    grep -q -- "$2" "$(find "$work/$1" -path '*/ElyOffice/ely-doc/ely.log' 2>/dev/null | head -n 1)" 2>/dev/null && return 0
    sleep 0.2
  done
  return 1
}

run_gallery probe
wait "$pid" || fail "the probe did not exit cleanly: $(tail -n 5 "$work/probe.err")"
log=$(find "$work/probe" -path "*/ElyOffice/ely-doc/ely.log" | head -n 1)
[ -n "$log" ] || fail "the probe wrote no log"
expect() { grep -q "probe: $1" "$log" || fail "no 'probe: $1' in the probe log"; }
expect_raw() { grep -q "$1" "$log" || fail "no '$1' in the probe log"; }
expect_re() { grep -qE "$1" "$log" || fail "no '$1' in the probe log"; }
expect "nav on page one shown=true"
expect "layout with both panels fits=true"
expect "props on page one shown=true"
expect "props on page two hidden=true"
expect "layout without right panel fits=true"
expect "content on page two shown=true"
expect "content focus kept=true"
expect "props back on page one shown=true"
expect "layout restored fits=true"
expect "menus app=ElyDoc File=New|Open…|Save|Save As…|Export…|Print…"
expect "menu map New=office::New|Open…=office::Open|Save=office::Save|Save As…=office::SaveAs|Export…=office::Export|Print…=office::Print"
expect_raw "command: file.new ran in ElyDoc"
expect "trigger keys file.new -> file.new"
expect "trigger menu file.new -> file.new"
expect "trigger keys file.open -> file.open"
expect "trigger menu file.open -> file.open"
for id in save save_as export print; do
  expect "trigger keys file.$id -> file.$id blocked"
  expect "trigger menu file.$id -> file.$id blocked"
done
expect "palette open=true"
expect "palette took focus=true"
expect_raw "command: file.open ran in ElyDoc"
expect "palette open=false"
expect "palette toggle-off returned focus=true"
expect "blocked save -> file.save blocked"
expect_re "toast [0-9]+: No open document"
expect "recent=file.open,file.new"
expect "tab 1 -> content"
expect "tab 2 -> props"
expect "esc closed palette=true"
expect "props rows Format=Mixed|Pages=2"
expect "rename open=true text=fail"
expect "renamed title=Appendix|Page Two"
expect "delete cancel pages=Appendix|Page Two"
expect "delete done pages=Appendix"
expect "task export -> Succeeded"
expect "task error kept -> Failed(The disk is full)"
expect "task after retry -> Succeeded"
expect "task cancel beats=1"
expect_re "chip by tab=true"
expect "layout back fits=true"
expect "resized viewport=800x600"
expect "resized layout fits=true"
expect "resized click hits=true"
expect "restored viewport=1100x720"
expect_re "scale factor=[0-9]"
expect "keyup-only enter keeps dialog=true titles=Appendix|Page Two"
expect "keyup kept text=页面1"
expect "keyup-only enter keeps palette=true titles="
expect "keyup palette closed=true"
expect "cjk text=页面1"
expect_re "cjk undo=$"
expect "delete last pages="
expect "empty rename refused=true"
expect "empty delete refused=true"
expect_re "palette during task at Running [012]/4"
run_gallery holdmix "" 49
wait_line holdmix "probe: hold at 49" 60 || fail "the gallery never held the mixed state"
"$root/tests/scripts/shot.sh" "$pid" "$work/mixed.png" || fail "no window for the mixed shot"
kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
run_gallery holdrun "" running
wait_line holdrun "probe: hold at running" 60 || fail "the gallery never held the running task"
"$root/tests/scripts/shot.sh" "$pid" "$work/running.png" || fail "no window for the running shot"
kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
run_gallery holdfail "" failed
wait_line holdfail "probe: hold at failed" 60 || fail "the gallery never held the failed task"
"$root/tests/scripts/shot.sh" "$pid" "$work/failed.png" || fail "no window for the failed shot"
kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
for name in mixed running failed; do
  diff=$(python3 "$root/tests/scripts/ink.py" cmp "$work/$name.png" "$root/tests/fixtures/shell/$name.rgbz")
  if [ "$diff" -ne 0 ]; then
    cp "$work/$name.png" "$root/target/shell-$name.png"
    fail "the $name region differs in $diff pixels; shot kept at target/shell-$name.png"
  fi
done
for locale in zh-CN fr zh-Hans; do
  run_gallery "loc-$locale" "" 11 "$locale"
  wait_line "loc-$locale" "probe: menus app=" 60 || fail "locale $locale: no menus log"
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
done
loclog() { find "$work/loc-$1" -path '*/ElyOffice/ely-doc/ely.log' | head -n 1; }
grep -q "menus app=ElyDoc File=新建|打开…|保存|另存为…|导出…|打印…" "$(loclog zh-CN)" || fail "zh-CN menus are not localized"
grep -q "i18n: locale zh-CN -> zh-CN" "$(loclog zh-CN)" || fail "zh-CN did not resolve to zh-CN"
grep -q "menus app=ElyDoc File=New|Open…|Save|Save As…|Export…|Print…" "$(loclog fr)" || fail "fr menus did not fall back to English"
grep -q "i18n: locale fr -> en-US" "$(loclog fr)" || fail "fr did not resolve to en-US"
grep -q "menus app=ElyDoc File=新建|打开…|保存|另存为…|导出…|打印…" "$(loclog zh-Hans)" || fail "zh-Hans menus are not localized"
echo "shell: regions, layout, focus, menus, commands, palette, dialogs, tasks pass"
