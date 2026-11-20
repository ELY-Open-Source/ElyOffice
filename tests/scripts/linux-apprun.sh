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
cp "$root/packaging/linux/AppRun" "$root/packaging/linux/ely-engine@.service" "$dir/"
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
for slug in ely-doc ely-table ely-show; do ln -s ../../libreoffice "$dir/$slug/bin/libreoffice"; done
echo worker >"$dir/ely-doc/bin/ely-lok-worker"
echo w1 >"$dir/ely-worker-id"
home=$work/home
# The user's own manager stays untouched.
mkdir "$work/bin"
printf '#!/bin/sh\necho "$*" >>"%s/systemctl"\n' "$work" >"$work/bin/systemctl"
chmod +x "$work/bin/systemctl"
run() { # image path, args
  local image=$1
  shift
  HOME=$home XDG_DATA_HOME= XDG_CONFIG_HOME= APPIMAGE=$image PATH=$work/bin:$PATH "$dir/AppRun" "$@" 2>>"$work/said"
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
# The engine unit: own name, quoting, three wants, one reload.
units=$home/.config/systemd/user
unit=$units/ely-engine-image@.service
[ ! -e "$units/ely-engine@.service" ] || fail "the AppImage took the package's unit name"
want='ExecStart=/usr/bin/env "/apps/my \"odd\" $$dir/Ely%%Office.AppImage" %i engine'
grep -qxF "$want" "$unit" || fail "the unit reads: $(grep Exec "$unit")"
[ "$(sed -n 2p "$unit")" = 'ConditionPathExists=!/usr/lib/systemd/user/ely-engine@.service' ] || fail "the unit does not stand down for the package: $(head -3 "$unit")"
[ "$(cd "$units/graphical-session.target.wants" && echo *)" = "ely-engine-image@ely-doc.service ely-engine-image@ely-show.service ely-engine-image@ely-table.service" ] ||
  fail "login wants: $(ls "$units/graphical-session.target.wants")"
[ "$(cat "$work/systemctl")" = "--user daemon-reload" ] || fail "systemctl ran: $(cat "$work/systemctl")"

# Kept once: a second start copies nothing.
: >"$work/said"
run "$odd" ely-doc >/dev/null
! grep -q kept "$work/said" || fail "the kernel was copied twice"
! grep -q "menu items" "$work/said" || fail "unchanged items were rewritten"
[ "$(wc -l <"$work/systemctl")" = 1 ] || fail "an unchanged unit reloaded systemd"
# A link a failed start missed comes back.
rm "$units/graphical-session.target.wants/ely-engine-image@ely-show.service"
run "$odd" ely-doc >/dev/null
[ "$(readlink "$units/graphical-session.target.wants/ely-engine-image@ely-show.service")" = "$unit" ] ||
  fail "the missing link stayed missing"
[ "$(wc -l <"$work/systemctl")" = 2 ] || fail "a new link did not reload systemd"
# An earlier AppImage's shadowing unit goes, links too.
sed -e 's|^ExecStart=.*|ExecStart=/usr/bin/env "/apps/ElyOffice-0.1.16.AppImage" %i engine|' -e '/^ConditionPathExists/d' "$unit" >"$units/ely-engine@.service"
for slug in ely-doc ely-table ely-show; do ln -s "$units/ely-engine@.service" "$units/graphical-session.target.wants/ely-engine@$slug.service"; done
ln -s /elsewhere "$units/graphical-session.target.wants/ely-engine@ely-kept.service"
: >"$work/said"
run "$odd" ely-doc >/dev/null
[ ! -e "$units/ely-engine@.service" ] || fail "the old AppImage unit still hides the package's"
[ "$(cd "$units/graphical-session.target.wants" && echo ely-engine@*)" = "ely-engine@ely-kept.service" ] || fail "old links: $(ls "$units/graphical-session.target.wants")"
grep -q "removed the engine unit that hid" "$work/said" || fail "the removal was not logged: $(cat "$work/said")"
[ "$(wc -l <"$work/systemctl")" = 3 ] || fail "the removal did not reload systemd"
rm "$units/graphical-session.target.wants/ely-engine@ely-kept.service"
# A unit the user wrote there is kept.
printf '[Service]\nExecStart=/opt/mine %%i\n' >"$units/ely-engine@.service"
run "$odd" ely-doc >/dev/null
grep -qx 'ExecStart=/opt/mine %i' "$units/ely-engine@.service" || fail "the user's own unit was removed"
rm "$units/ely-engine@.service"
# Dry run (--test starts nothing): the package's name resolves there.
if [ -x /usr/lib/systemd/systemd ]; then
  pkg=$work/usr-lib-systemd-user
  mkdir -p "$pkg"
  sed 's/^Description=.*/Description=the package/' "$root/packaging/linux/ely-engine@.service" >"$pkg/ely-engine@.service"
  resolved() { # unit: its fragment path, from a dry run
    env -i PATH=/usr/bin:/bin HOME=$home XDG_CONFIG_HOME=$home/.config XDG_DATA_HOME=$home/.local/share XDG_RUNTIME_DIR=$work/run \
      SYSTEMD_UNIT_PATH="$units:$pkg:" /usr/lib/systemd/systemd --test --user --unit="$1" --no-pager 2>/dev/null |
      awk -v u="$1" '$0 ~ "Unit " u ":" { on = 1 } on && /Fragment Path:/ { print $3; exit }'
  }
  mkdir -p "$work/run"
  [ "$(resolved ely-engine@ely-doc.service)" = "$pkg/ely-engine@.service" ] || fail "the package's unit is hidden: $(resolved ely-engine@ely-doc.service)"
  [ "$(resolved ely-engine-image@ely-doc.service)" = "$unit" ] || fail "the AppImage's unit is not found: $(resolved ely-engine-image@ely-doc.service)"
else
  echo "SKIP: no /usr/lib/systemd/systemd for the dry run"
fi

# 0.1.16's name picks the app; a move rewrites items.
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
# systemd runs the unit from the same odd path.
if systemctl --user is-system-running >/dev/null 2>&1 || [ "$(systemctl --user is-system-running 2>/dev/null)" = degraded ]; then
  probe=ely-apprun-$$
  printf '#!/bin/sh\necho "$@" >>"%s/engine"\n' "$work" >"$odd/doc.AppImage"
  sed -e "s/^Description=.*/Description=linux-apprun probe/" -e '/^ConditionPathExists/d' "$unit" >"$work/$probe@.service"
  systemctl --user link "$work/$probe@.service" >/dev/null 2>&1
  trap 'systemctl --user disable "$probe@.service" >/dev/null 2>&1; rm -rf "$work"' EXIT
  systemctl --user start "$probe@ely-doc.service" && systemctl --user stop "$probe@ely-doc.service" ||
    fail "systemd cannot run the unit: $(systemctl --user status "$probe@ely-doc.service" 2>&1 | tail -n 3)"
  [ "$(cat "$work/engine")" = "$(printf 'ely-doc engine\nely-doc engine stop')" ] || fail "the unit ran: $(cat "$work/engine")"
else
  echo "SKIP: no user systemd for the unit's run"
fi
echo "linux-apprun: word, name or the picker; quoted menu items; the engine unit never takes the package's name and stands down for it, an earlier AppImage's shadow goes; the kernel kept once per id, the worker per build; starts at once; gio and systemd start paths with % and quotes"
