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

# A clone writing one package and the source pair.
clone() { # os
  local at=$work/$1 source='d=$1; touch "$d/ElyOffice-V-c0ffee-source.tar.gz" "$d/ElyOffice-V-c0ffee-upstream-source.tar.gz"'
  mkdir -p "$at/scripts" "$at/packaging/macos" "$at/packaging/linux"
  cp "$root/scripts/ci.sh" "$at/scripts/"
  cp "$root/Cargo.toml" "$root/rust-toolchain.toml" "$at/"
  cp "$root/packaging/apps.tsv" "$at/packaging/"
  write() { printf '#!/usr/bin/env bash\n%s\n' "$2" >"$at/$1" && chmod +x "$at/$1"; }
  write packaging/source.sh "$source"
  mkdir -p "$at/tests/scripts"
  write tests/scripts/engine-arch.sh :
  write packaging/macos/release.sh "mkdir -p dist; touch dist/App-$1.dmg; packaging/source.sh dist"
  write packaging/linux/container.sh 'exec "$@"'
  write packaging/linux/package.sh "mkdir -p dist; touch dist/\$1-$1.AppImage"
}
go() { # os
  CI_OS=$1 PATH=$work/bin:$PATH ELY_SHARE=$work/share ELY_RELEASE_TOKEN=unused \
    ELY_MICROSOFT_CLIENT_ID=unused \
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

# macOS first: it builds the pair and uploads neither.
run macos
[ "$(uploads 'source' macos)" -eq 0 ] || fail "macOS uploaded source: $(cat "$STATE")"
[ "$(uploads 'App-macos.dmg' macos)" -eq 1 ] || fail "macOS skipped its dmg"
run linux
[ "$(uploads '-source.tar.gz' linux)" -eq 2 ] || fail "Linux did not upload the pair: $(cat "$STATE")"
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
echo "ci-upload: Linux alone uploads source; reruns replace their own"
