#!/usr/bin/env bash
# First-batch bundles open Finder's files on their own engines.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || {
  echo "SKIP release-macos: macOS only"
  exit 0
}
# Real path: the apps log theirs.
dist=$(cd "${ELY_DIST:-$root/dist}" && pwd -P)
work=$(mktemp -d)
home=$work/home
trap 'pkill -f "$dist/Ely[A-Za-z]*.app/Contents/MacOS/" 2>/dev/null || true; sleep 1; rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
apps=$(awk -F'\t' '$4 == 1 { print $1 "\t" $2 }' "$root/packaging/apps.tsv")
[ "$(echo "$apps" | wc -l | tr -d ' ')" = 7 ] || fail "the first batch is not seven apps"
while IFS=$'\t' read -r slug name; do
  [ -d "$dist/$name.app" ] || fail "no $dist/$name.app; run packaging/macos/release.sh"
done <<<"$apps"
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
commit=$(git -C "$root" rev-parse --short=12 HEAD)
source=$dist/ElyOffice-$version-$commit-source.tar.gz
[ -s "$source" ] || fail "no $source; run packaging/macos/release.sh"
listed=$(tar -tzf "$source")
for part in LICENSE Cargo.lock patches/libreoffice/ patches/mupdf/ scripts/fetch-references.sh scripts/build-git.sh; do
  grep -qx "ElyOffice-$version/$part" <<<"$listed" || fail "the source archive lacks $part"
done
for part in agents.md PRD.md tasks/; do
  ! grep -qx "ElyOffice-$version/$part" <<<"$listed" || fail "the source archive has the internal $part"
done
# Copyleft upstream as built: MuPDF with our patches, and Git.
upstream=$dist/ElyOffice-$version-$commit-upstream-source.tar.gz
[ -s "$upstream" ] || fail "no $upstream; run packaging/macos/release.sh"
for part in mupdf-src/COPYING git/COPYING git/GIT-VERSION-GEN; do
  tar -tzf "$upstream" "$part" >/dev/null 2>&1 || fail "the upstream archive lacks $part"
done
tar -xzOf "$upstream" mupdf-src/include/mupdf/pdf/form.h | grep -q pdf_signature_set_value_said ||
  fail "the MuPDF source is not the patched one"

# Samples: fixtures, so no LibreOffice app ever runs here.
files=$work/files
mkdir -p "$files/repo"
cp "$root"/tests/fixtures/office/t.* "$files/"
printf '# Bundle\n' >"$files/t.md"
cp "$root/tests/fixtures/mail/sample.eml" "$files/t.eml"
for kind in docx xlsx pptx pdf md eml; do
  [ -s "$files/t.$kind" ] || fail "no t.$kind sample"
  cp "$files/t.$kind" "$files/u.$kind"
done
git -C "$files/repo" init -q
mkdir -p "$files/second" && git -C "$files/second" init -q

log_of() { echo "$home/Library/Application Support/ElyOffice/$1/ely.log"; }
waits() { # slug, text
  for _ in $(seq 240); do
    grep -qF "$2" "$(log_of "$1")" 2>/dev/null && return 0
    sleep 0.25
  done
  fail "$1 never logged '$2': $(tail -n 5 "$(log_of "$1")" 2>/dev/null)"
}
while IFS=$'\t' read -r slug name; do
  app=$dist/$name.app
  codesign --verify --deep --strict "$app" || fail "$name's signature does not verify"
  icon=$(/usr/libexec/PlistBuddy -c "Print :CFBundleIconFile" "$app/Contents/Info.plist")
  [ -s "$app/Contents/Resources/$icon.icns" ] || fail "$name has no icon"
  # Licences travel with the app (§21.3).
  licenses=$app/Contents/Resources/Licenses
  case $slug in
    ely-doc | ely-table | ely-show) engines=(LibreOffice) ;;
    ely-pdf) engines=(MuPDF MuPDF/fonts/urw MuPDF/fonts/sil MuPDF/fonts/noto MuPDF/fonts/droid MuPDF/fonts/han OpenSSL tessdata GreatVibes) ;;
    ely-tree) engines=(Git git-lfs) ;;
    ely-mark) engines=(KaTeX MuPDF MuPDF/fonts/noto OpenSSL GreatVibes) ;;
    ely-mail) engines=() ;;
  esac
  for part in LICENSE rust-crates.txt "${engines[@]}"; do
    [ -n "$(find "$licenses/$part" -type f -size +0 2>/dev/null)" ] || fail "$name lacks the licences of $part"
  done
  # Each font family needs a licence, not just a README.
  for family in "$licenses"/MuPDF/fonts/*; do
    [ -d "$family" ] || continue
    [ -n "$(find "$family" -type f -size +0 \( -iname 'licen[cs]e*' -o -iname 'copying*' -o -iname 'notice*' -o -iname 'ofl*' \))" ] ||
      fail "$name lacks the licence of the ${family##*/} fonts"
  done
  # Nothing inside registers with macOS.
  extra=$(find "$app/Contents" \( -name '*.app' -o -name '*.appex' -o -name '*.mdimporter' -o -name '*.qlgenerator' \))
  [ -z "$extra" ] || fail "$name carries bundles macOS registers: $extra"
  # No binary may need a newer macOS than the app.
  least=$(/usr/libexec/PlistBuddy -c "Print :LSMinimumSystemVersion" "$app/Contents/Info.plist")
  while IFS= read -r binary; do
    file -b "$binary" | grep -q '^Mach-O' || continue
    # vtool: otool reads "name (x)" as an archive member.
    minos=$(vtool -show-build "$binary" | awk '/minos/ && !v { v = $2 } END { print v }')
    [ -z "$minos" ] || [ "$(printf '%s\n%s\n' "$minos" "$least" | sort -V | tail -n 1)" = "$least" ] ||
      fail "${binary#"$app"/} needs macOS $minos; $name says $least"
    "$root/tests/scripts/rpath.sh" "$app" "$binary"
  done < <(find "$app" -type f \( -perm -111 -o -name '*.dylib' \))
  case $slug in
    ely-doc) first=t.docx second=u.docx ;;
    ely-table) first=t.xlsx second=u.xlsx ;;
    ely-show) first=t.pptx second=u.pptx ;;
    ely-pdf) first=t.pdf second=u.pdf ;;
    ely-mark) first=t.md second=u.md ;;
    ely-tree) first=repo second=second ;;
    ely-mail) first=t.eml second=u.eml ;;
  esac
  # Finder's launch: no arguments, an open request.
  open -n -a "$app" --env "HOME=$home" --env ELY_LOG=info --env ELY_NO_DIALOG=1 "$files/$first"
  case $slug in
    ely-tree) opened() { echo "tree: opened $(cd "$files/$1" && pwd -P)"; } ;;
    # No file in its log: count the second open.
    ely-mail) opened() { echo "mail: a message file opened"; } ;;
    *) opened() { echo "$slug: opened $1 as tab"; } ;;
  esac
  waits "$slug" "$(opened "$first")"
  log=$(log_of "$slug")
  case $slug in
    ely-doc | ely-table | ely-show)
      grep -qF "lok: worker from $app/Contents/MacOS/ely-lok-worker" "$log" || fail "$name used another LOK worker"
      grep -qF "worker from $app/Contents/MacOS/../Frameworks" "$log" ||
        fail "$name used another kernel: $(grep 'worker from' "$log")"
      ! grep -q "opened Untitled" "$log" || fail "$name opened a blank document beside the file"
      ;;
    ely-pdf) grep -qF "pdf: worker from $app/Contents/MacOS/ely-pdf-worker" "$log" || fail "ElyPDF used another worker" ;;
    ely-tree)
      grep -qF "at $app/Contents/MacOS/../Resources/git/bin/git (bundled)" "$log" || fail "ElyTree used another Git"
      # Built-ins stay links to one binary, not copies.
      [ "$(du -sm "$app/Contents/Resources/git" | cut -f1)" -lt 100 ] || fail "ElyTree's Git copies its built-ins"
      ;;
  esac
  # A running app takes Finder's next file.
  open -a "$app" "$files/$second"
  if [ "$slug" = ely-mail ]; then
    seen=0
    for _ in $(seq 240); do
      seen=$(grep -cF "$(opened "$second")" "$(log_of "$slug")" || true)
      [ "$seen" -ge 2 ] && break
      sleep 0.25
    done
    [ "$seen" -ge 2 ] || fail "ElyMail did not take the second file"
  else
    waits "$slug" "$(opened "$second")"
  fi
  pkill -f "$app/Contents/MacOS/$slug" || true
  echo "ok: $name"
done <<<"$apps"
# The app menu starts another installed app (§16).
opens=$(grep -c "ElyDoc window open" "$(log_of ely-doc)")
open -n -a "$dist/ElyDoc.app" --env "HOME=$home" --env ELY_LOCALE=en --env ELY_NO_DIALOG=1
for _ in $(seq 120); do [ "$(grep -c "ElyDoc window open" "$(log_of ely-doc)")" -gt "$opens" ] && break; sleep 0.25; done
doc=$(pgrep -f "$dist/ElyDoc.app/Contents/MacOS/ely-doc" | head -n 1)
[ -n "$doc" ] || fail "ElyDoc did not start"
osascript -e "tell application \"System Events\" to tell (first process whose unix id is $doc)" \
  -e 'set frontmost to true' -e 'delay 1' \
  -e 'click menu item "ElyMark" of menu 1 of menu item "Open Ely App" of menu 1 of menu bar item 2 of menu bar 1' \
  -e 'end tell' >/dev/null || fail "ElyDoc's app menu has no ElyMark"
for _ in $(seq 40); do pgrep -f "$dist/ElyMark.app/Contents/MacOS/ely-mark" >/dev/null && break; sleep 0.25; done
pgrep -f "$dist/ElyMark.app/Contents/MacOS/ely-mark" >/dev/null || fail "the menu did not start ElyMark"
pkill -f "$dist/Ely[A-Za-z]*.app/Contents/MacOS/" || true
echo "release-macos: 6 bundles open files on their own engines and start each other"
