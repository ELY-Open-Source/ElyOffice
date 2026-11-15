#!/usr/bin/env bash
# Usage: release.sh [batch]; one .pkg of the batch's apps.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
batch=${1:-1}
dist=${ELY_DIST:-$root/dist}
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
arch=$(uname -m)
prefix=ely.zacharyzhang.office.pkg
[ -z "${ELY_NOTARY_PROFILE:-}" ] || { [ -n "${ELY_SIGN_IDENTITY:-}" ] && [ -n "${ELY_INSTALLER_IDENTITY:-}" ]; } || {
  echo "release.sh: notarizing needs ELY_SIGN_IDENTITY and ELY_INSTALLER_IDENTITY" >&2
  exit 1
}
"$root/packaging/source.sh" "$dist"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/pkgs"
# The kernel's choice follows the apps that use it.
outline= choices= refs= users=
while IFS=$'\t' read -r slug name about; do
  "$root/packaging/macos/bundle.sh" "$slug" release
  codesign --verify --deep --strict "$dist/$name.app"
  mkdir "$work/$slug"
  ditto "$dist/$name.app" "$work/$slug/$name.app"
  # Relocatable would follow a stray copy elsewhere.
  pkgbuild --quiet --analyze --root "$work/$slug" "$work/$slug.plist"
  plutil -replace 0.BundleIsRelocatable -bool NO "$work/$slug.plist"
  pkgbuild --quiet --root "$work/$slug" --component-plist "$work/$slug.plist" --install-location /Applications \
    --identifier "$prefix.${slug#ely-}" --version "$version" "$work/pkgs/$slug.pkg"
  outline+="    <line choice=\"$slug\"/>"$'\n'
  choices+="  <choice id=\"$slug\" title=\"$name\" description=\"$about\"><pkg-ref id=\"$prefix.${slug#ely-}\"/></choice>"$'\n'
  refs+="  <pkg-ref id=\"$prefix.${slug#ely-}\" version=\"$version\">$slug.pkg</pkg-ref>"$'\n'
  if grep -q '^ely-lok' "$root/apps/$slug/Cargo.toml"; then users+="${users:+ || }choices['$slug'].selected"; fi
done < <(awk -F'\t' -v batch="$batch" '$4 == batch { print $1 "\t" $2 "\t" $6 }' "$root/packaging/apps.tsv")
[ -n "$users" ] || {
  echo "release.sh: no app of batch $batch uses the kernel" >&2
  exit 1
}
kernel="/Library/Application Support/Ely Office/Kernel"
login="/Library/Application Support/Ely Office/ely-login"
label=ely.zacharyzhang.office.login
"$root/packaging/macos/kernel.sh" "$dist/Kernel"
# The console user's engines; none at the login window.
user='user=$(stat -f %Su /dev/console); [ "$user" != root ] || exit 0; uid=$(id -u "$user")
as() { launchctl asuser "$uid" sudo -u "$user" -H "$@"; }'
# Engines stop before a new kernel replaces the old.
mkdir "$work/scripts" "$work/login-scripts"
cat >"$work/scripts/preinstall" <<SH
#!/bin/sh
if [ -x "$login" ]; then (
  $user
  as "$login" stop || exit 2
) || exit 2; fi
rm -rf "$kernel"
SH
chmod +x "$work/scripts/preinstall"
pkgbuild --quiet --root "$dist/Kernel" --scripts "$work/scripts" --install-location "$kernel" \
  --identifier "$prefix.kernel" --version "$version" "$work/pkgs/kernel.pkg"
# Login starts the engines; so does the install's end.
build=${CARGO_TARGET_DIR:-$root/target}/release
cargo build --manifest-path "$root/Cargo.toml" --release -p ely-login
mkdir -p "$work/login/Library/LaunchAgents" "$work/login/Library/Application Support/Ely Office"
cp "$build/ely-login" "$work/login$login"
if [ -n "${ELY_SIGN_IDENTITY:-}" ]; then
  codesign --force --options runtime --timestamp --sign "$ELY_SIGN_IDENTITY" "$work/login$login"
else
  codesign --force --sign - "$work/login$login"
fi
cat >"$work/login/Library/LaunchAgents/$label.plist" <<XML
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>$label</string>
  <key>ProgramArguments</key><array><string>$login</string></array>
  <key>RunAtLoad</key><true/>
  <key>ProcessType</key><string>Background</string>
  <key>LimitLoadToSessionType</key><string>Aqua</string>
  <key>AbandonProcessGroup</key><true/>
</dict>
</plist>
XML
cat >"$work/login-scripts/postinstall" <<SH
#!/bin/sh
$user
as "$login" || exit 1
if launchctl print "gui/\$uid/$label" >/dev/null 2>&1; then launchctl bootout "gui/\$uid/$label" || exit 1; fi
launchctl bootstrap "gui/\$uid" "/Library/LaunchAgents/$label.plist" || exit 1
SH
chmod +x "$work/login-scripts/postinstall"
pkgbuild --quiet --root "$work/login" --scripts "$work/login-scripts" --install-location / \
  --identifier "$prefix.login" --version "$version" "$work/pkgs/login.pkg"
cat >"$work/distribution.xml" <<XML
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
  <title>Ely Office</title>
  <options customize="always" require-scripts="true" rootVolumeOnly="true" hostArchitectures="$arch"/>
  <domains enable_localSystem="true"/>
  <allowed-os-versions><os-version min="14.0"/></allowed-os-versions>
  <choices-outline>
$outline    <line choice="kernel"/>
  </choices-outline>
$choices  <choice id="kernel" visible="false" enabled="false" selected="$users"><pkg-ref id="$prefix.kernel"/><pkg-ref id="$prefix.login"/></choice>
$refs  <pkg-ref id="$prefix.kernel" version="$version">kernel.pkg</pkg-ref>
  <pkg-ref id="$prefix.login" version="$version">login.pkg</pkg-ref>
</installer-gui-script>
XML
out=$dist/ElyOffice-$version-$arch.pkg
sign=()
[ -z "${ELY_INSTALLER_IDENTITY:-}" ] || sign=(--sign "$ELY_INSTALLER_IDENTITY" --timestamp)
productbuild --quiet --distribution "$work/distribution.xml" --package-path "$work/pkgs" ${sign[@]+"${sign[@]}"} "$out"
if [ -n "${ELY_NOTARY_PROFILE:-}" ]; then
  xcrun notarytool submit "$out" --keychain-profile "$ELY_NOTARY_PROFILE" --wait
  xcrun stapler staple "$out"
else
  echo "release.sh: $out is not notarized (no ELY_NOTARY_PROFILE)" >&2
fi
# The update manifest names this package.
case $arch in arm64) cpu=aarch64 ;; *) cpu=$arch ;; esac
"$root/packaging/manifest.sh" ely-office "macos-$cpu" "$out"
echo "$out"
rm -rf "$root/target/release/incremental"
