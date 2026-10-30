#!/usr/bin/env bash
# ci.sh uploads: Linux alone sends source; reruns replace their own.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
command -v python3 >/dev/null || {
  echo "SKIP: no python3"
  exit 0
}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
mkdir -p "$work/share/engines/openssl/lib/pkgconfig" "$work/share/reference" "$work/bin"
# Intel Macs check for the pinned OpenSSL.
: >"$work/share/engines/openssl/lib/pkgconfig/libcrypto.pc"
export STATE=$work/uploads
: >"$STATE"

# Fake release API: $STATE holds "id name os" assets.
cat >"$work/bin/curl" <<'EOF'
#!/usr/bin/env bash
url=${!#}
case "$* " in *" -X DELETE "*) method=DELETE ;; *) method=GET ;; esac
case $method:$url in
  GET:*/releases/tags/*) echo '{"id": 1}' ;;
  GET:*/releases/1/assets\?per_page=100\&page=1)
    python3 -c 'import json, sys; print(json.dumps([{"id": int(l.split()[0]), "name": l.split()[1]} for l in open(sys.argv[1]) if l.strip()]))' "$STATE" ;;
  GET:*/releases/1/assets\?per_page=100\&page=*) echo '[]' ;;
  DELETE:*/releases/assets/*)
    id=${url##*/}
    grep -q "^$id " "$STATE" || { echo "curl stub: no asset $id" >&2; exit 22; }
    grep -v "^$id " "$STATE" >"$STATE.new" || true
    mv "$STATE.new" "$STATE" ;;
  GET:https://uploads.github.com/*/releases/1/assets\?name=*)
    echo "$(($(wc -l <"$STATE") + RANDOM * 100)) ${url##*name=} $CI_OS" >>"$STATE" ;;
  *) echo "curl stub: unexpected $method $url" >&2; exit 22 ;;
esac
EOF
chmod +x "$work/bin/curl"
# Fake cargo: logs each call; $CARGO_FAILS names the failing one.
export CARGO_LOG=$work/cargo.log
cat >"$work/bin/cargo" <<'EOF'
#!/usr/bin/env bash
echo "$* | engines=${ELY_REQUIRE_ENGINES:-} incremental=${CARGO_INCREMENTAL:-}" >>"$CARGO_LOG"
[ "$1" != "${CARGO_FAILS:-}" ] || { echo "cargo $1 failed"; exit 101; }
EOF
chmod +x "$work/bin/cargo"

# A clone writing one package and the source pair.
clone() { # os
  local at=$work/$1 source='d=$1; touch "$d/ElyOffice-V-c0ffee-source.tar.gz" "$d/ElyOffice-V-c0ffee-upstream-source.tar.gz"'
  mkdir -p "$at/scripts" "$at/packaging/macos" "$at/packaging/linux"
  cp "$root/scripts/ci.sh" "$root/scripts/release-tests.sh" "$at/scripts/"
  cp "$root/Cargo.toml" "$root/rust-toolchain.toml" "$at/"
  cp "$root/packaging/apps.tsv" "$at/packaging/"
  write() { printf '#!/usr/bin/env bash\n%s\n' "$2" >"$at/$1" && chmod +x "$at/$1"; }
  write packaging/source.sh "$source"
  mkdir -p "$at/tests/scripts"
  write tests/scripts/engine-arch.sh :
  write packaging/sign.sh 'for f; do echo signed >"$f.sig"; done'
  write packaging/macos/release.sh "mkdir -p dist; touch dist/App-$1.dmg dist/app-$1.json; packaging/source.sh dist"
  write packaging/linux/container.sh 'exec "$@"'
  write packaging/linux/package.sh "mkdir -p dist; touch dist/\$1-$1.AppImage dist/\$1-$1.json"
}
go() { # os
  CI_OS=$1 PATH=$work/bin:$PATH ELY_SHARE=$work/share ELY_RELEASE_TOKEN=unused \
    ELY_MICROSOFT_CLIENT_ID=unused ELY_UPDATE_KEY=unused \
    ELY_SIGN_IDENTITY=unused ELY_NOTARY_PROFILE=unused \
    "$work/$1/scripts/ci.sh" "$1" "v$version" >"$work/$1.out" 2>&1
}
run() { # os
  clone "$1"
  go "$1" || fail "$1: $(cat "$work/$1.out")"
}
uploads() { # name pattern, os
  awk -v n="$1" -v o="$2" '$2 ~ n && $3 == o' "$STATE" | wc -l
}

# The gate: both workers, then every test, engines required.
gate="build -p ely-lok --bins -p ely-pdf-worker | engines=1 incremental=0
test --workspace | engines=1 incremental=0"
# A failed build or test uploads nothing.
for os in macos linux; do
  for step in build test; do
    clone "$os"
    : >"$CARGO_LOG"
    ! CARGO_FAILS=$step go "$os" || fail "$os released past a failing $step"
    grep -q "cargo $step failed" "$work/$os.out" || fail "$os did not run cargo $step: $(cat "$work/$os.out")"
    [ "$step" = test ] || ! grep -q "^test " "$CARGO_LOG" || fail "$os tested past a failed build"
    [ ! -s "$STATE" ] || fail "$os uploaded past a failing $step: $(cat "$STATE")"
    rm -rf "${work:?}/$os"
  done
done
: >"$CARGO_LOG"
# macOS first: it builds the pair and uploads neither.
run macos
[ "$(cat "$CARGO_LOG")" = "$gate" ] || fail "macOS gate ran: $(cat "$CARGO_LOG")"
[ "$(uploads 'source' macos)" -eq 0 ] || fail "macOS uploaded source: $(cat "$STATE")"
[ "$(uploads 'App-macos.dmg' macos)" -eq 1 ] || fail "macOS skipped its dmg"
[ "$(uploads 'app-macos.json.sig' macos)" -eq 1 ] || fail "macOS left its manifest unsigned: $(cat "$STATE")"
: >"$CARGO_LOG"
run linux
[ "$(cat "$CARGO_LOG")" = "$gate" ] || fail "Linux gate ran: $(cat "$CARGO_LOG")"
[ "$(uploads '-source.tar.gz' linux)" -eq 2 ] || fail "Linux did not upload the pair: $(cat "$STATE")"
[ "$(uploads 'ely-doc-linux.json.sig' linux)" -eq 1 ] || fail "Linux left a manifest unsigned: $(cat "$STATE")"
# A rerun replaces its own files once and keeps macOS's.
mac=$(grep ' macos$' "$STATE")
ids=$(awk '$3 == "linux" { print $1 }' "$STATE" | sort)
rm -rf "$work/linux"
run linux
grep -q "replacing ElyOffice-V-c0ffee-source.tar.gz" "$work/linux.out" || fail "rerun: $(cat "$work/linux.out")"
[ "$(awk '{ print $2 }' "$STATE" | sort | uniq -d)" = "" ] || fail "a name twice: $(cat "$STATE")"
[ "$(grep ' macos$' "$STATE")" = "$mac" ] || fail "the rerun touched macOS: $(cat "$STATE")"
[ "$(uploads '-source.tar.gz' linux)" -eq 2 ] || fail "the rerun lost the pair: $(cat "$STATE")"
[ -z "$(comm -12 <(echo "$ids") <(awk '$3 == "linux" { print $1 }' "$STATE" | sort))" ] ||
  fail "an old upload stayed: $(cat "$STATE")"
echo "ci-upload: strict gate first; Linux alone uploads source; reruns replace their own"
