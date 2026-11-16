#!/usr/bin/env bash
# packaging/nix.sh writes the release; the flake builds, licenses, runs.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
for tool in curl python3; do
  command -v "$tool" >/dev/null || {
    echo "SKIP: no $tool"
    exit 0
  }
done
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
apps=$root/packaging/apps.tsv

# Fake manifests for every first-batch app.
mkdir -p "$work/root/packaging" "$work/root/nix" "$work/dl/v9.9.9"
cp "$root/packaging/nix.sh" "$apps" "$work/root/packaging/"
awk -F'\t' '$4 == 1 { print $1 }' "$apps" | while read -r slug; do
  sha=$(printf %s "$slug" | sha256sum | cut -d' ' -f1)
  printf '{"version":"9.9.9","assets":{"linux-x86_64":{"url":"https://x/%s.AppImage","sha256":"%s"}}}\n' \
    "$slug" "$sha" >"$work/dl/v9.9.9/$slug-linux-x86_64.json"
done
ELY_NIX_DOWNLOAD="file://$work/dl" "$work/root/packaging/nix.sh" v9.9.9 >/dev/null
python3 - "$apps" "$work/root/nix/release.json" <<'EOF' || fail "nix.sh wrote the wrong release"
import hashlib, json, sys
rows = [r.split("\t") for r in open(sys.argv[1]).read().splitlines()]
release = json.load(open(sys.argv[2]))
assert release["version"] == "9.9.9", release["version"]
first = {r[0]: r for r in rows if r[3] == "1"}
assert set(release["apps"]) == set(first), sorted(release["apps"])
for slug, app in release["apps"].items():
    assert app["name"] == first[slug][1], slug
    assert app["license"] == first[slug][4], (slug, app["license"])
    assert app["url"] == f"https://x/{slug}.AppImage", slug
    assert app["sha256"] == hashlib.sha256(slug.encode()).hexdigest(), slug
EOF
# A manifest of another version refuses and keeps the file.
written=$(sha256sum <"$work/root/nix/release.json")
sed -i 's/9\.9\.9/9.9.8/' "$work/dl/v9.9.9/ely-mark-linux-x86_64.json"
if ELY_NIX_DOWNLOAD="file://$work/dl" "$work/root/packaging/nix.sh" v9.9.9 >/dev/null 2>&1; then
  fail "nix.sh took a manifest of another version"
fi
[ "$(sha256sum <"$work/root/nix/release.json")" = "$written" ] || fail "a refused run rewrote release.json"

# The committed release agrees with the table.
python3 - "$apps" "$root/nix/release.json" <<'EOF' || fail "nix/release.json disagrees with apps.tsv"
import json, sys
rows = [r.split("\t") for r in open(sys.argv[1]).read().splitlines()]
apps = json.load(open(sys.argv[2]))["apps"]
first = {r[0]: r for r in rows if r[3] == "1"}
assert set(apps) == set(first), sorted(apps)
for slug, app in apps.items():
    assert (app["name"], app["license"]) == (first[slug][1], first[slug][4]), slug
EOF
echo "nix.sh: release written and checked"

command -v docker >/dev/null || {
  echo "SKIP: no docker for the flake"
  exit 0
}
mkdir -p "$work/src"
cp -r "$root/flake.nix" "$root/nix" "$work/src/"
cat >"$work/src/run.sh" <<'EOF'
set -eu
nixc() { nix --extra-experimental-features "nix-command flakes" "$@"; }
cp -r /src /tmp/flake && cd /tmp/flake
nixc eval --json path:/tmp/flake#packages.x86_64-linux \
  --apply 'ps: { names = builtins.attrNames ps; licenses = map (l: l.spdxId) ps.ely-office.meta.license; }' >/out/licenses.json
nixc build path:/tmp/flake#ely-office --print-out-paths >/dev/null
cp result/share/applications/ely-mark.desktop /out/
test -s result/share/icons/hicolor/512x512/apps/ely-mark.png
icd=$(nixc build --no-link --print-out-paths nixpkgs#mesa)/share/vulkan/icd.d/lvp_icd.x86_64.json
nixc shell nixpkgs#xvfb-run -c sh -c \
  "VK_ICD_FILENAMES=$icd ELY_SMOKE=1 timeout 300 xvfb-run -a ./result/bin/ely-mark" >/out/smoke.log 2>&1
EOF
mkdir -p "$work/out"
# The FHS wrapper needs namespaces: bwrap.
docker run --rm --privileged -v "$work/src:/src:ro" -v "$work/out:/out" nixos/nix sh /src/run.sh || {
  tail -n 40 "$work/out/smoke.log" >&2 || true
  fail "the flake did not build or run"
}
python3 - "$apps" "$work/out/licenses.json" <<'EOF' || fail "the flake's licenses differ from apps.tsv"
import json, sys
rows = [r.split("\t") for r in open(sys.argv[1]).read().splitlines()]
got = json.load(open(sys.argv[2]))
want = {l for r in rows if r[3] == "1" for l in r[4].split(" AND ")}
assert got["names"] == ["default", "ely-office"], got["names"]
assert sorted(got["licenses"]) == sorted(want), (got["licenses"], want)
EOF
grep -qx 'Exec=ely-mark %F' "$work/out/ely-mark.desktop" || fail "the desktop entry does not run ely-mark"
grep -q 'ElyMark window open' "$work/out/smoke.log" || fail "ElyMark did not open a window"
echo "flake: one ely-office package, licenses match, ElyMark built and drew"
