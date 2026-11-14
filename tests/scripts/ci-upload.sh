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
case "$* " in *" -X DELETE "*) method=DELETE ;; *" -d "*) method=POST ;; *) method=GET ;; esac
case $method:$url in
  GET:*/actions/workflows/pr.yml/runs\?head_sha=*\&status=success) echo "{\"total_count\": ${PR_PASSED:-1}}" ;;
  GET:*/releases/tags/*) [ -s "$STATE.made" ] || exit 22; echo '{"id": 1}' ;;
  POST:*/releases)
    for arg; do [ "${prev:-}" != -d ] || echo "$arg" >>"$STATE.made"; prev=$arg; done ;;
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
[ "$1" != metadata ] || { echo '{"target_directory": "/built"}'; exit 0; }
[ "$1" != --version ] || { echo "cargo 1.101.0 (${CARGO_IS:-huacnlee/cargo 0.102.0-9})"; exit 0; }
echo "$* | engines=${ELY_REQUIRE_ENGINES:-} incremental=${CARGO_INCREMENTAL:-} workers=${ELY_LOK_WORKER:-},${ELY_PDF_WORKER:-}" >>"$CARGO_LOG"
[ "$1" != "${CARGO_FAILS:-}" ] || { echo "cargo $1 failed"; exit 101; }
EOF
chmod +x "$work/bin/cargo"

# A clone writing one package and the three source files.
clone() { # os
  local at=$work/$1 source='d=$1; touch "$d/ElyOffice-V-c0ffee-source.tar.gz" "$d/ElyOffice-V-c0ffee-upstream-source.tar.gz" "$d/ElyOffice-V-c0ffee-libreoffice-external-source.tar"'
  mkdir -p "$at/scripts" "$at/packaging/macos" "$at/packaging/linux"
  cp "$root/scripts/ci.sh" "$root/scripts/release-tests.sh" "$at/scripts/"
  # The gates' own rules are release-gate.sh's to test.
  mkdir -p "$at/tests/perf"
  printf 'import os, sys\nsys.exit(1 if os.environ.get("GATE_FAILS") == sys.argv[0].rsplit("/", 1)[1] else 0)\n' |
    tee "$at/scripts/compatibility.py" "$at/scripts/release-gate.py" >"$at/tests/perf/baseline.py"
  git -C "$at" init -q && git -C "$at" -c user.name=t -c user.email=t@t -c commit.gpgsign=false commit -q --allow-empty -m t
  cp "$root/Cargo.toml" "$root/rust-toolchain.toml" "$at/"
  cp "$root/packaging/apps.tsv" "$at/packaging/"
  write() { printf '#!/usr/bin/env bash\n%s\n' "$2" >"$at/$1" && chmod +x "$at/$1"; }
  write packaging/source.sh "$source"
  mkdir -p "$at/tests/scripts"
  write tests/scripts/engine-arch.sh :
  write tests/scripts/linux-glibc.sh '[ -z "${GLIBC_FAILS:-}" ]'
  write packaging/sign.sh 'for f; do echo signed >"$f.sig"; done'
  write packaging/macos/release.sh "mkdir -p dist; touch dist/App-$1.dmg dist/app-$1.json; packaging/source.sh dist"
  # The container's Cargo may differ from the host's.
  write packaging/linux/container.sh 'CARGO_IS=${INSIDE_IS:-${CARGO_IS:-}} exec "$@"'
  write packaging/linux/package.sh "mkdir -p dist; touch dist/\$1-$1.AppImage dist/\$1-$1.json"
  write packaging/linux/bundle.sh "touch dist/ely-office-$1.deb"
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

# The gate: both workers, then every test with them named.
gate="build -p ely-lok --bins -p ely-pdf-worker | engines=1 incremental=0 workers=,
test --workspace | engines=1 incremental=0 workers=/built/debug/ely-lok-worker,/built/debug/ely-pdf-worker"
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
# A failed gate, or pr.yml not passed, builds nothing.
for check in compatibility.py release-gate.py baseline.py pr; do
  clone linux
  : >"$CARGO_LOG"
  case $check in
    pr) ! PR_PASSED=0 go linux || fail "linux released a commit pr.yml did not pass" ;;
    *) ! GATE_FAILS=$check go linux || fail "linux released past a failing $check" ;;
  esac
  [ ! -s "$CARGO_LOG" ] || fail "linux built past a failing $check: $(cat "$CARGO_LOG")"
  [ ! -s "$STATE" ] || fail "linux uploaded past a failing $check: $(cat "$STATE")"
  rm -rf "${work:?}/linux"
done
# Standard Cargo, on a host or inside, builds nothing.
while read -r os host inside where; do
  clone "$os"
  : >"$CARGO_LOG"
  ! CARGO_IS=${host#-} INSIDE_IS=${inside#-} go "$os" || fail "$os released with standard Cargo in $where"
  grep -q "${where//-/ } has cargo .*, not huacnlee/cargo" "$work/$os.out" || fail "$os did not name $where: $(cat "$work/$os.out")"
  [ ! -s "$CARGO_LOG" ] || fail "$os built with standard Cargo in $where: $(cat "$CARGO_LOG")"
  [ ! -s "$STATE" ] || fail "$os uploaded with standard Cargo in $where: $(cat "$STATE")"
  rm -rf "${work:?}/$os"
done <<'EOF'
macos rust-lang - this-host
linux rust-lang - this-host
linux - rust-lang packaging/linux/container.sh
EOF
# Linux packages past the glibc ceiling upload nothing.
clone linux
! GLIBC_FAILS=1 go linux || fail "linux released past the glibc check"
[ ! -s "$STATE" ] || fail "linux uploaded past the glibc check: $(cat "$STATE")"
rm -rf "${work:?}/linux"
: >"$CARGO_LOG"
# macOS first: it builds the source and uploads none.
run macos
[ "$(cat "$CARGO_LOG")" = "$gate" ] || fail "macOS gate ran: $(cat "$CARGO_LOG")"
[ "$(uploads 'source' macos)" -eq 0 ] || fail "macOS uploaded source: $(cat "$STATE")"
[ "$(uploads 'App-macos.dmg' macos)" -eq 1 ] || fail "macOS skipped its dmg"
[ "$(uploads 'app-macos.json.sig' macos)" -eq 1 ] || fail "macOS left its manifest unsigned: $(cat "$STATE")"
# Made once, as a pre-release.
[ "$(wc -l <"$STATE.made")" -eq 1 ] || fail "releases made: $(cat "$STATE.made")"
python3 -c 'import json, sys; assert json.load(open(sys.argv[1]))["prerelease"] is True' "$STATE.made" ||
  fail "a public release from one platform: $(cat "$STATE.made")"
: >"$CARGO_LOG"
run linux
[ "$(cat "$CARGO_LOG")" = "$gate" ] || fail "Linux gate ran: $(cat "$CARGO_LOG")"
[ "$(uploads '-source.tar' linux)" -eq 3 ] || fail "Linux did not upload the source: $(cat "$STATE")"
[ "$(uploads 'ely-doc-linux.json.sig' linux)" -eq 1 ] || fail "Linux left a manifest unsigned: $(cat "$STATE")"
[ "$(uploads 'ely-office-linux.deb' linux)" -eq 1 ] || fail "Linux uploaded no suite package: $(cat "$STATE")"
# A rerun replaces its own files once and keeps macOS's.
mac=$(grep ' macos$' "$STATE")
ids=$(awk '$3 == "linux" { print $1 }' "$STATE" | sort)
rm -rf "$work/linux"
run linux
grep -q "replacing ElyOffice-V-c0ffee-source.tar.gz" "$work/linux.out" || fail "rerun: $(cat "$work/linux.out")"
[ "$(awk '{ print $2 }' "$STATE" | sort | uniq -d)" = "" ] || fail "a name twice: $(cat "$STATE")"
[ "$(grep ' macos$' "$STATE")" = "$mac" ] || fail "the rerun touched macOS: $(cat "$STATE")"
[ "$(uploads '-source.tar' linux)" -eq 3 ] || fail "the rerun lost the source: $(cat "$STATE")"
[ -z "$(comm -12 <(echo "$ids") <(awk '$3 == "linux" { print $1 }' "$STATE" | sort))" ] ||
  fail "an old upload stayed: $(cat "$STATE")"
echo "ci-upload: strict gate first; x86_64 Linux alone uploads source; reruns replace their own"
