#!/usr/bin/env bash
# The one Windows setup builds, keeps data, static runtime.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
nsi=$root/packaging/windows/installer.nsi
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

strings=$root/packaging/windows/strings.nsh
mapfile -t langs < <(sed -n 's/^!insertmacro MUI_LANGUAGE "\(.*\)"$/\1/p' "$strings")
[ "${langs[0]}" = English ] || fail "English is not first, so it is not the fallback"
[ "${langs[*]}" = "English SimpChinese SpanishInternational French German Japanese Korean Vietnamese" ] ||
  fail "the setup's languages are not the apps' eight: ${langs[*]}"
for key in $(awk '$1 == "LangString" { print $2 }' "$strings" | sort -u); do
  for lang in "${langs[@]}"; do
    [ "$(grep -c "^LangString $key \${LANG_${lang^^}} " "$strings")" -eq 1 ] || fail "$key has no single $lang text"
  done
done
for key in $(grep -ohP '\$\(\K[A-Z]\w+(?=\))' "$root"/packaging/windows/*.ns?); do
  grep -q "^LangString $key " "$strings" || fail "$key is used but has no text"
done

command -v makensis >/dev/null || {
  echo "SKIP installer-windows: makensis is not installed"
  exit 0
}
stages=$work/stages apps=$work/apps.nsh
echo '!macro ElyAll M' >"$apps"
count=0
while IFS=$'\t' read -r slug name _ batch _; do
  [ "$batch" = 1 ] || continue
  mkdir -p "$stages/$slug"
  printf 'MZ' >"$stages/$slug/$slug.exe"
  "$root/packaging/windows/types.sh" "$slug" >"$stages/$slug-types.nsh"
  cp "$root/assets/icons/$slug.ico" "$stages/$slug/"
  "$root/packaging/windows/files.sh" "$stages/$slug" >"$stages/$slug-files.txt"
  printf '  !insertmacro ${M} %s %s 1\n' "$name" "$slug" >>"$apps"
  count=$((count + 1))
done <"$root/packaging/apps.tsv"
echo '!macroend' >>"$apps"
mkdir -p "$stages/kernel"
echo kernel >"$stages/kernel/kernel.txt"
"$root/packaging/windows/files.sh" "$stages/kernel" >"$stages/kernel-files.txt"
[ "$count" -eq 7 ] || fail "the first batch has $count apps, expected 7"
out=$work/setup.exe
makensis -V1 -INPUTCHARSET UTF8 -DVERSION=0.1.0 -DAPPS="$apps" -DSTAGES="$stages" \
  -DOUTFILE="$out" "$nsi" >"$work/makensis.log" 2>&1 || fail "$(tail -n 5 "$work/makensis.log")"
[ "$(head -c 2 "$out")" = MZ ] || fail "the setup is not a Windows executable"

[ "$(grep -c 'RMDir /r' "$nsi")" -eq 1 ] || fail "the uninstaller removes more than one tree"
grep -qx '      RMDir /r "$LOCALAPPDATA\\ElyOffice\\${SLUG}"' "$nsi" || fail "the removed tree is not the app's own data directory"
grep -qx '      MessageBox MB_YESNO|MB_DEFBUTTON2 "$(DeleteData)" /SD IDNO IDNO +2' "$nsi" ||
  fail "data removal is not confirmed with No as the default"
if rustup target add x86_64-pc-windows-msvc >/dev/null 2>&1; then
  mkdir -p "$work/probe/src"
  printf '[package]\nname = "probe"\nversion = "0.0.0"\nedition = "2021"\n' >"$work/probe/Cargo.toml"
  printf 'fn main() {}\n' >"$work/probe/src/main.rs"
  cfg=$(cd "$root" && cargo rustc -q --manifest-path "$work/probe/Cargo.toml" --target x86_64-pc-windows-msvc -- --print cfg 2>&1) ||
    fail "cargo rustc for the msvc target failed: $cfg"
  grep -qx 'target_feature="crt-static"' <<<"$cfg" || fail "the msvc build links the C runtime dynamically"
  echo "installer-windows: msvc target links the C runtime statically"
else
  echo "SKIP installer-windows: msvc target is not installable, so the runtime link was not checked"
fi
if makensis -V1 -DVERSION=1 -DSTAGES="$stages" -DOUTFILE="$work/x.exe" "$nsi" >/dev/null 2>&1; then
  fail "a setup without APPS built"
fi
echo "installer-windows: one setup with the first batch builds; uninstall keeps documents and asks before data"
