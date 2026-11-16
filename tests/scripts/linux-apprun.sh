#!/usr/bin/env bash
# The unified AppImage's AppRun: which app, menu items, the kernel.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Linux ] || {
  echo "SKIP: Linux only"
  exit 0
}
command -v gio >/dev/null || {
  echo "SKIP: no gio"
  exit 0
}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# An AppDir of fake apps that print what they got.
dir=$work/ElyOffice.AppDir
mkdir -p "$dir/libreoffice/program"
cp "$root/packaging/linux/AppRun" "$dir/"
echo kernel >"$dir/libreoffice/program/libmergedlo.so"
echo k1 >"$dir/libreoffice/program/ely-kernel-id"
awk -F'\t' '$4 == 1 { print $1 "\t" $2 }' "$root/packaging/apps.tsv" >"$dir/apps.tsv"
while IFS=$'\t' read -r slug name; do
  mkdir -p "$dir/$slug/bin" "$dir/$slug/share/applications" "$dir/$slug/share/icons/hicolor/512x512/apps"
  printf '#!/bin/sh\necho "%s pick=${ELY_PICK:-} lok=$ELY_LOK_DIR worker=$ELY_LOK_WORKER args=$*"\n' "$slug" >"$dir/$slug/bin/$slug"
  chmod +x "$dir/$slug/bin/$slug"
  printf '[Desktop Entry]\nType=Application\nName=%s\nExec=%s %%F\nIcon=%s\n' "$name" "$slug" "$slug" >"$dir/$slug/share/applications/$slug.desktop"
  echo "$slug" >"$dir/$slug/share/icons/hicolor/512x512/apps/$slug.png"
done <"$dir/apps.tsv"
echo worker >"$dir/ely-doc/bin/ely-lok-worker"
echo w1 >"$dir/ely-worker-id"
home=$work/home
run() { # image path, args
  local image=$1
  shift
  HOME=$home XDG_DATA_HOME= APPIMAGE=$image "$dir/AppRun" "$@" 2>>"$work/said"
}

# The first word names the app; files follow.
odd='/apps/my "odd" $dir/Ely%Office.AppImage'
got=$(run "$odd" ely-table /x/a.xlsx)
lib=$home/.local/lib/ely-office
kept=$lib/k1
[ "$got" = "ely-table pick= lok=$kept/libreoffice/program worker=$lib/worker-w1/ely-lok-worker args=/x/a.xlsx" ] || fail "by word: $got"
[ "$(cat "$kept/libreoffice/program/libmergedlo.so")" = kernel ] && [ "$(cat "$lib/worker-w1/ely-lok-worker")" = worker ] || fail "the kernel was not kept"
grep -q "kernel k1 kept in $kept" "$work/said" && grep -q "worker w1 kept" "$work/said" || fail "the copy was not logged: $(cat "$work/said")"
menu=$home/.local/share/applications
for slug in $(cut -f1 "$dir/apps.tsv"); do
  [ -f "$menu/$slug.desktop" ] && [ -f "$home/.local/share/icons/hicolor/512x512/apps/$slug.png" ] || fail "no menu item for $slug"
done
# Desktop Exec: \\ before the four, %% for %.
want='Exec=/usr/bin/env "/apps/my \\"odd\\" \\$dir/Ely%%Office.AppImage" ely-doc %F'
grep -qxF "$want" "$menu/ely-doc.desktop" || fail "Exec reads: $(grep Exec "$menu/ely-doc.desktop")"
grep -qx 'Name=ElyDoc' "$menu/ely-doc.desktop" || fail "the item lost its other lines"

# Kept once: a second start copies nothing.
: >"$work/said"
run "$odd" ely-doc >/dev/null
! grep -q kept "$work/said" || fail "the kernel was copied twice"
! grep -q "menu items" "$work/said" || fail "unchanged items were rewritten"

# 0.1.16's file name names the app; a move rewrites the items.
got=$(run /apps/ElyShow-0.1.16-linux-x86_64.AppImage /x/b.pptx)
case $got in "ely-show pick= "*"args=/x/b.pptx") ;; *) fail "by name: $got" ;; esac
grep -qxF 'Exec=/usr/bin/env "/apps/ElyShow-0.1.16-linux-x86_64.AppImage" ely-mark %F' "$menu/ely-mark.desktop" || fail "the move left old items"
grep -q "menu items now open /apps/ElyShow" "$work/said" || fail "the rewrite was not logged"
got=$(run /apps/ElyTree.AppImage)
case $got in "ely-tree pick= "*) ;; *) fail "by bare name: $got" ;; esac

# No name, no word: ElyMark's binary asks.
got=$(run /apps/doc.AppImage /x/c.md)
case $got in "ely-mark pick=1 "*"args=/x/c.md") ;; *) fail "the picker lost the file: $got" ;; esac
grep -q "doc.AppImage names no app" "$work/said" || fail "the ask was not logged"
got=$(run /apps/ElyOffice-9.9.9-linux-x86_64.AppImage)
case $got in "ely-mark pick=1 "*) ;; *) fail "the suite's name: $got" ;; esac

# A new kernel id: kept beside the old.
echo k2 >"$dir/libreoffice/program/ely-kernel-id"
run /apps/doc.AppImage ely-pdf >/dev/null
[ -d "$home/.local/lib/ely-office/k2/libreoffice" ] && [ -d "$kept" ] || fail "the new kernel was not kept"
[ -z "$(find "$home/.local/lib/ely-office" -maxdepth 1 -name '.*.*')" ] || fail "a part copy was left"
# A new worker, the same kernel: the new worker runs.
echo worker2 >"$dir/ely-doc/bin/ely-lok-worker"
echo w2 >"$dir/ely-worker-id"
got=$(run /apps/doc.AppImage ely-doc)
case $got in *"worker=$lib/worker-w2/ely-lok-worker "*) ;; *) fail "the old worker stayed: $got" ;; esac
[ "$(cat "$lib/worker-w2/ely-lok-worker")" = worker2 ] || fail "worker w2 is not the new one"

# Starts at once: each writes its own part files.
rm -rf "$home/.local/share"
for i in $(seq 32); do run /apps/doc.AppImage ely-doc >"$work/at-once.$i" & done
wait
for i in $(seq 32); do
  grep -q '^ely-doc ' "$work/at-once.$i" || fail "start $i of 32 did not run: $(grep -v kept "$work/said" | tail -n 2)"
done
[ -z "$(find "$home/.local/share" -name '*.[0-9]*')" ] || fail "a part file was left"

# GLib's launcher, a % in the path.
odd=$work/'a\b "c" $d `e` %done'
mkdir -p "$odd"
printf '#!/bin/sh\necho "$@" >"%s/launched"\n' "$work" >"$odd/doc.AppImage"
chmod +x "$odd/doc.AppImage"
run "$odd/doc.AppImage" ely-doc >/dev/null
XDG_DATA_HOME=$home/.local/share dbus-run-session -- gio launch "$home/.local/share/applications/ely-doc.desktop" /x/d.docx 2>>"$work/said" ||
  fail "gio cannot launch the item: $(tail -n 1 "$work/said")"
for _ in $(seq 50); do [ -s "$work/launched" ] && break; sleep 0.1; done
[ "$(cat "$work/launched" 2>/dev/null)" = "ely-doc /x/d.docx" ] || fail "the item did not start $odd/doc.AppImage: $(grep Exec "$home/.local/share/applications/ely-doc.desktop")"
echo "linux-apprun: word, name or the picker; quoted menu items; the kernel kept once per id, the worker per build; starts at once; gio launches paths with % and quotes"
