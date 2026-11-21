#!/usr/bin/env bash
# vendor's crates run their own tests, each on its platform.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root/vendor"
# The apps' versions; only dev-dependencies resolve afresh.
cp "$root/Cargo.lock" Cargo.lock
case $(uname -s) in
  Linux) host=linux ;;
  Darwin) host=macos ;;
  MINGW* | MSYS* | CYGWIN*) host=windows ;;
  *) echo "FAIL: vendor-tests has no list for $(uname -s)"; exit 1 ;;
esac
# gpui-pre is absent: its tests need zed's unpublished assets.
ran=()
# This host's gpui is what the patches touch.
gpui=gpui-pre-$host gpui_ran=
mkdir -p target
# platform, crate, then what its tests need.
while read -r on crate args; do
  case $on in "" | "#"*) continue ;; esac
  [ "$on" = any ] || [ "$on" = "$host" ] || continue
  log=target/vendor-tests-$crate.log
  # shellcheck disable=SC2086
  cargo test --quiet -p "$crate" $args </dev/null 2>&1 | tee "$log" || { echo "FAIL: vendor/$crate's tests"; exit 1; }
  [ "$crate" != "$gpui" ] || gpui_ran=$(grep -oE "test result: ok\. [0-9]+ passed" "$log" | awk '{n += $4} END {print n + 0}')
  ran+=("$crate")
done <<'PLAN'
any accesskit_atspi_common
linux accesskit_unix
linux gpui-pre-linux --features test-support
macos gpui-pre-macos --features test-support
windows gpui-pre-windows --features test-support
any mail-parser --all-features
any pulldown-cmark
# zbus's e2e test needs zbus_xml, from zbus's own repository.
linux zbus --lib --test basic --test builder_message_stream --test connection_closed --test issues
PLAN
[ -n "$gpui_ran" ] || { echo "FAIL: vendor-tests ran no $gpui row on $host"; exit 1; }
[ "$gpui_ran" -gt 0 ] || { echo "FAIL: vendor/$gpui ran no tests on $host"; exit 1; }
echo "vendor-tests: ${ran[*]} pass on $host ($gpui: $gpui_ran tests)"
