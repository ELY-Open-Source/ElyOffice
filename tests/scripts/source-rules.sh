#!/usr/bin/env bash
# source.sh packs LibreOffice's tarballs and the fonts' upstream, hash-checked.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
for tool in git shasum tar; do
  command -v "$tool" >/dev/null || {
    echo "SKIP: no $tool"
    exit 0
  }
done
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/bin" "$work/served"
# Fake curl: serves $work/served, else fails like a 404.
cat >"$work/bin/curl" <<'EOF'
#!/usr/bin/env bash
while [ $# -gt 1 ]; do
  [ "$1" != -o ] || out=$2
  shift
done
[ -f "$SERVED/${1##*/}" ] || exit 22
cp "$SERVED/${1##*/}" "$out"
EOF
chmod +x "$work/bin/curl"
export SERVED=$work/served
hash() { printf '%s' "$1" | shasum -a 256 | cut -d ' ' -f 1; }

# One tarball built here, one only on the mirror.
repo=$work/repo
mkdir -p "$repo/packaging/libreoffice/fonts" "$repo/engines/fonts/.upstream" "$repo/engines/mupdf-src" "$repo/reference/git" "$repo/engines/lo-src/external/tarballs"
cp "$root/packaging/source.sh" "$repo/packaging/"
printf '[workspace.package]\nversion = "9.9.9"\n' >"$repo/Cargo.toml"
printf '/engines\n/reference\n' >"$repo/.gitignore"
url=https://dev-www.libreoffice.org/src
printf 'built.tar.gz\t%s\t%s/built.tar.gz\nmirror.tar.xz\t%s\t%s/mirror.tar.xz\n' \
  "$(hash built)" "$url" "$(hash mirror)" "$url" >"$repo/packaging/lo-tarballs.tsv"
: >"$repo/engines/mupdf-src/COPYING"
: >"$repo/reference/git/COPYING"
printf built >"$repo/engines/lo-src/external/tarballs/built.tar.gz"
printf mirror >"$work/served/mirror.tar.xz"
# Fonts: one zip twice, fetched; one file already fetched here.
fonts=https://example.org/fonts
list=$repo/packaging/libreoffice/fonts/SOURCES.tsv
printf 'folder\tfile\turl\tsha256\tupstream_sha256\n' >"$list"
for member in A.ttf B.ttf; do
  printf 'Z\t%s\t%s/z.zip#%s\t00\t%s\n' "$member" "$fonts" "$member" "$(hash zip)" >>"$list"
done
printf 'F\tF.ttf\t%s/F.ttf\t%s\t%s\n' "$fonts" "$(hash font)" "$(hash font)" >>"$list"
printf zip >"$work/served/z.zip"
printf font >"$repo/engines/fonts/.upstream/$(hash font)"
git -C "$repo" init -q
git -C "$repo" add -A
git -C "$repo" -c user.name=t -c user.email=t@t -c commit.gpgsign=false commit -qm t
commit=$(git -C "$repo" rev-parse --short=12 HEAD)
external=ElyOffice-9.9.9-$commit-libreoffice-external-source.tar

pack() { # dist
  PATH=$work/bin:$PATH "$repo/packaging/source.sh" "$work/$1" >"$work/$1.out" 2>&1
}
pack good || fail "source.sh failed: $(cat "$work/good.out")"
[ -f "$work/good/$external" ] || fail "no $external: $(cat "$work/good.out")"
listed=$(tar -tf "$work/good/$external" | sort)
want=$(printf 'libreoffice-external/\nlibreoffice-external/built.tar.gz\nlibreoffice-external/fonts/\nlibreoffice-external/fonts/F.ttf\nlibreoffice-external/fonts/SOURCES.tsv\nlibreoffice-external/fonts/z.zip\nlibreoffice-external/lo-tarballs.tsv\nlibreoffice-external/mirror.tar.xz\n')
[ "$listed" = "$want" ] || fail "archive holds: $listed"
# The built one is packed as bytes, not a link.
[ "$(tar -xOf "$work/good/$external" libreoffice-external/built.tar.gz)" = built ] || fail "built tarball not packed by content"
[ "$(tar -xOf "$work/good/$external" libreoffice-external/mirror.tar.xz)" = mirror ] || fail "mirror tarball not packed"
grep -qx "source.sh: fetching $url/mirror.tar.xz" "$work/good.out" || fail "no fetch line: $(cat "$work/good.out")"
! grep -q "fetching $url/built.tar.gz" "$work/good.out" || fail "fetched what was built"
[ "$(tar -xOf "$work/good/$external" libreoffice-external/fonts/F.ttf)" = font ] || fail "the font here not packed by content"
[ "$(tar -xOf "$work/good/$external" libreoffice-external/fonts/z.zip)" = zip ] || fail "the font zip not packed"
[ "$(grep -c "fetching $fonts/z.zip" "$work/good.out")" = 1 ] || fail "the font zip not fetched once: $(cat "$work/good.out")"
! grep -q "fetching $fonts/F.ttf" "$work/good.out" || fail "fetched a font already here"
printf zap >"$work/served/z.zip"
! pack fonts || fail "a font upstream with a wrong hash passed"
grep -q "z.zip has sha256 $(hash zap), not $(hash zip)" "$work/fonts.out" || fail "fonts: $(cat "$work/fonts.out")"
[ ! -e "$work/fonts/$external" ] || fail "an archive despite a wrong font upstream"
printf zip >"$work/served/z.zip"

# A wrong byte anywhere and no archive.
printf mirrar >"$work/served/mirror.tar.xz"
! pack fetched || fail "a fetched tarball with a wrong hash passed"
grep -q "mirror.tar.xz has sha256 $(hash mirrar), not $(hash mirror)" "$work/fetched.out" || fail "fetched: $(cat "$work/fetched.out")"
[ ! -e "$work/fetched/$external" ] || fail "an archive despite a wrong hash"
rm "$work/served/mirror.tar.xz"
! pack missing || fail "an unfetchable tarball passed"
grep -q "cannot fetch $url/mirror.tar.xz" "$work/missing.out" || fail "missing: $(cat "$work/missing.out")"
[ ! -e "$work/missing/$external" ] || fail "an archive despite a missing tarball"
printf mirror >"$work/served/mirror.tar.xz"
printf bilt >"$repo/engines/lo-src/external/tarballs/built.tar.gz"
! pack local || fail "a built tarball with a wrong hash passed"
grep -q "built.tar.gz has sha256 $(hash bilt), not $(hash built)" "$work/local.out" || fail "local: $(cat "$work/local.out")"
[ ! -e "$work/local/$external" ] || fail "an archive despite a wrong local hash"
echo "source-rules: tarballs and font upstreams packed; wrong or missing ones stop it"
