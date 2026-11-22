#!/usr/bin/env bash
# One kernel shared only when the copies match.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Linux ] || {
  echo "SKIP: Linux only"
  exit 0
}
base=$(mktemp -d)
trap 'rm -rf "$base"' EXIT
# A path that is no regular expression.
work=$base/odd[
mkdir "$work"
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
arch=$(uname -m)

# Only what bundle.sh reads; nfpm records its calls.
plant() {
  local tree=$1 slug name batch
  mkdir -p "$tree/packaging/linux" "$tree/engines"
  cp "$root/packaging/linux/bundle.sh" "$tree/packaging/linux/"
  cp "$root/packaging/apps.tsv" "$tree/packaging/"
  echo 'version = "9.9.9"' >"$tree/Cargo.toml"
  printf '#!/bin/sh\nfor last; do :; done\n[ -d "$last" ] || { echo "Open $last: is a directory" >&2; exit 1; }\necho "$@" >>"%s/nfpm.log"\n' "$tree" >"$tree/engines/nfpm-2.47.0-$arch"
  chmod +x "$tree/engines/nfpm-2.47.0-$arch"
  cp "$root/packaging/linux/AppRun" "$root/packaging/linux/ely-engine@.service" "$tree/packaging/linux/"
  cp "$root/packaging/manifest.sh" "$tree/packaging/"
  mkdir -p "$tree/assets/icons"
  echo icon >"$tree/assets/icons/ely-doc.png"
  # appimagetool: the last word is the image.
  printf '#!/bin/sh\necho "$@" >>"%s/pack.log"\nfor last; do :; done\necho image >"$last"\n' "$tree" >"$tree/engines/appimagetool-1.9.1-$arch.AppImage"
  chmod +x "$tree/engines/appimagetool-1.9.1-$arch.AppImage"
  echo runtime >"$tree/engines/type2-runtime-20251108-$arch"
  while IFS=$'\t' read -r slug name _ batch _; do
    [ "$batch" = 1 ] || continue
    local package=$tree/target/package/$slug
    mkdir -p "$package/$name.AppDir/usr/bin"
    echo app >"$package/$name.AppDir/usr/bin/$slug"
    for kind in deb rpm archlinux; do echo "lib-$kind" >"$package/depends.$kind"; done
    case $slug in ely-doc | ely-table | ely-show)
      mkdir -p "$package/$name.AppDir/usr/bin/libreoffice/program" "$package/$name.AppDir/usr/bin/libreoffice/share/template/common"
      echo kernel >"$package/$name.AppDir/usr/bin/libreoffice/program/libmergedlo.so"
      echo worker >"$package/$name.AppDir/usr/bin/ely-lok-worker"
      ;;
    esac
  done <"$root/packaging/apps.tsv"
  # Each staged kernel carries its own id, as stage.sh writes.
  local kernel
  for kernel in "$tree"/target/package/*/*.AppDir/usr/bin/libreoffice; do
    "$root/scripts/kernel-id.sh" "$kernel" "$kernel/program/ely-kernel-id" >/dev/null
  done
}
kernel_of() { echo "$1/target/package/$2/$3.AppDir/usr/bin/libreoffice"; }
bundle() { ELY_DIST=$1/dist "$1/packaging/linux/bundle.sh" >"$1/out.log" 2>&1; }

plant "$work/same"
bundle "$work/same" || fail "equal kernels did not bundle: $(cat "$work/same/out.log")"
[ "$(wc -l <"$work/same/nfpm.log")" = 3 ] || fail "nfpm did not make the three packages"
shared=$work/same/target/package/ely-office/opt
[ -f "$shared/libreoffice/program/libmergedlo.so" ] || fail "the shared kernel is missing"
[ ! -e "$shared/ely-doc/bin/libreoffice" ] || fail "ElyDoc kept a kernel of its own"
held=$(cat "$shared/libreoffice/program/ely-kernel-id")
"$root/scripts/kernel-id.sh" "$shared/libreoffice" "$base/ely-kernel-id" >/dev/null
[ "$held" = "$(cat "$base/ely-kernel-id")" ] || fail "the shared kernel's id is not its own"

# The AppImage: the same tree, links relative, eight manifests.
appdir=$work/same/target/package/ely-office/ElyOffice.AppDir
[ "$(wc -l <"$work/same/pack.log")" = 1 ] || fail "appimagetool did not run once"
grep -qF -- "--runtime-file $work/same/engines/type2-runtime-20251108-$arch $appdir" "$work/same/pack.log" ||
  fail "appimagetool packed another folder: $(cat "$work/same/pack.log")"
for slug in ely-doc ely-table ely-show; do
  [ "$(readlink "$appdir/$slug/bin/libreoffice")" = ../../libreoffice ] || fail "$slug's kernel link is not relative"
  [ -f "$appdir/$slug/bin/libreoffice/program/libmergedlo.so" ] || fail "$slug's kernel link leads nowhere"
done
[ "$(cat "$appdir/ely-worker-id")" = "$(echo worker | sha256sum | cut -c1-16)" ] || fail "the worker id is not the worker's"
[ -x "$appdir/AppRun" ] && [ -f "$appdir/ely-office.desktop" ] && [ -f "$appdir/ely-doc.png" ] || fail "the AppDir lacks its entry"
cmp -s "$appdir/ely-engine@.service" "$root/packaging/linux/ely-engine@.service" || fail "the AppDir lacks the engine unit"
# The packages: the unit, three login links.
pkg=$work/same/target/package/ely-office
grep -qx '    dst: /usr/lib/systemd/user/ely-engine@.service' "$pkg/nfpm.yaml" || fail "the packages lack the engine unit"
wants=/etc/systemd/user/graphical-session.target.wants
grep -qF "$wants/ely-engine@ely-doc.service $wants/ely-engine@ely-table.service $wants/ely-engine@ely-show.service" "$pkg/postinstall" ||
  fail "postinstall links: $(cat "$pkg/postinstall")"
grep -qF "[ -e /usr/lib/systemd/user/ely-engine@.service ] || rm -f" "$pkg/postremove" || fail "postremove: $(cat "$pkg/postremove")"
grep -qx '  preremove: .*/preremove' "$pkg/nfpm.yaml" || fail "the packages lack the preremove"
# Before removal: each user manager stops the engines.
if [ -d /run/systemd/system ] && [ -d "/run/user/$(id -u)" ]; then
  mkdir "$work/fake"
  # Managers answer per $ACTIVE; engines exit $STOPPED.
  printf '#!/bin/sh\ncase $1 in is-active) exit ${ACTIVE:-0} ;; esac\necho "systemctl $*" >>"%s/stopped"\n' "$work" >"$work/fake/systemctl"
  printf '#!/bin/sh\necho "systemd-run $*" >>"%s/stopped"\nexit ${STOPPED:-0}\n' "$work" >"$work/fake/systemd-run"
  # runuser: records the user, runs the rest.
  printf '#!/bin/sh\necho "runuser $1 $2 $3" >>"%s/stopped"\nshift 3\nexec "$@"\n' "$work" >"$work/fake/runuser"
  chmod +x "$work/fake/systemctl" "$work/fake/systemd-run" "$work/fake/runuser"
  me=$(id -nu)
  PATH=$work/fake:$PATH sh "$pkg/preremove" || fail "preremove failed"
  for slug in ely-doc ely-table ely-show; do
    grep -qxF "systemd-run --user --wait --quiet --collect -p RuntimeMaxSec=60 /usr/bin/$slug engine stop" "$work/stopped" ||
      fail "$slug's engine was not stopped: $(cat "$work/stopped")"
  done
  grep -qxF -- "systemctl --user stop ely-engine@*.service" "$work/stopped" || fail "the units were not stopped: $(cat "$work/stopped")"
  [ "$(grep -cxF -- "runuser -u $me --" "$work/stopped")" = 4 ] || fail "not each stop ran as $me: $(cat "$work/stopped")"
  # A stop that fails keeps the files.
  : >"$work/stopped"
  status=0
  STOPPED=29 PATH=$work/fake:$PATH sh "$pkg/preremove" 2>"$work/refused" || status=$?
  [ "$status" = 1 ] || fail "a failed stop let removal go on: $status"
  grep -q "idle engine did not stop; removal stops" "$work/refused" || fail "the failed stop is not named: $(cat "$work/refused")"
  ! grep -q "^systemctl" "$work/stopped" || fail "the units stopped after a failed engine stop"
  # No manager: skipped, said.
  : >"$work/stopped"
  ACTIVE=3 PATH=$work/fake:$PATH sh "$pkg/preremove" 2>"$work/skipped" || fail "a user without a manager failed removal"
  [ ! -s "$work/stopped" ] && grep -q "has no systemd manager; skipped" "$work/skipped" || fail "the skip: $(cat "$work/stopped" "$work/skipped")"
else
  echo "SKIP: no user manager for the preremove's run"
fi
[ "$(cut -f1 "$appdir/apps.tsv" | paste -sd' ')" = "$(awk -F'\t' '$4 == 1 { print $1 }' "$root/packaging/apps.tsv" | paste -sd' ')" ] ||
  fail "the AppDir's apps: $(cat "$appdir/apps.tsv")"
image=$work/same/dist/ElyOffice-9.9.9-linux-$arch.AppImage
sha=$(sha256sum "$image" | cut -d' ' -f1)
for name in ely-office $(cut -f1 "$appdir/apps.tsv"); do
  grep -qF "\"sha256\": \"$sha\"" "$work/same/dist/$name-linux-$arch.json" || fail "$name's manifest does not name the AppImage"
done

plant "$work/differs"
echo other >"$(kernel_of "$work/differs" ely-table ElyTable)/program/libmergedlo.so"
! bundle "$work/differs" || fail "a differing kernel was shared"
grep -q "ely-table's kernel is not ElyShow's" "$work/differs/out.log" || fail "the differing kernel is not named"
[ ! -e "$work/differs/nfpm.log" ] && [ ! -e "$work/differs/pack.log" ] || fail "a package was made after a differing kernel"

# A link to nothing: diff cannot read it.
plant "$work/unread"
library=$(kernel_of "$work/unread" ely-doc ElyDoc)/program/libmergedlo.so
rm "$library"
ln -s /nonexistent/library "$library"
! bundle "$work/unread" || fail "a kernel that could not be read was shared"
grep -q "ely-doc's kernel did not compare" "$work/unread/out.log" || fail "the unread kernel is not named"
[ ! -e "$work/unread/nfpm.log" ] || fail "a package was made after an unread kernel"
echo "linux-bundle-rules: one kernel is shared only when every copy compares equal; the AppImage holds it once"
