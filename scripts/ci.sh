#!/usr/bin/env bash
# A platform's release. Usage: ci.sh <macos|windows|linux> <tag>
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
os=${1:?macos, windows or linux}
tag=${2:?the release tag, such as v0.1.3}
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
[ "$tag" = "v$version" ] || {
  echo "ci.sh: tag $tag is not v$version" >&2
  exit 1
}
: "${ELY_RELEASE_TOKEN:?the GitHub token that uploads}"
: "${ELY_MICROSOFT_CLIENT_ID:?the Microsoft OAuth client ID}"
: "${ELY_UPDATE_KEY:?the release key that signs update manifests}"
if [ "$os" = macos ]; then
  : "${ELY_SIGN_IDENTITY:?the Developer ID Application certificate}"
  : "${ELY_NOTARY_PROFILE:?the notarytool keychain profile}"
fi
[ -z "${ELY_PATH:-}" ] || PATH=$ELY_PATH:$PATH
cd "$root"
# Linux builds and packages in the AlmaLinux 8 container.
inside=()
[ "$os" != linux ] || inside=(packaging/linux/container.sh)

# Intel Macs run under Rosetta: x86_64 Rust, pinned OpenSSL.
intel=
if [ "$os" = macos ] && [ "$(uname -s)-$(uname -m)" = Darwin-x86_64 ]; then
  intel=1
  channel=$(awk -F'"' '$1 ~ /^channel/ { print $2 }' rust-toolchain.toml)
  export RUSTUP_TOOLCHAIN=$channel-x86_64-apple-darwin PKG_CONFIG_PATH=$root/engines/openssl/lib/pkgconfig
fi
# Linux's MuPDF links the pinned static OpenSSL too.
pinned_ssl=
if [ -n "$intel" ] || [ "$os" = linux ]; then
  pinned_ssl=1
fi

# Engines another clone on this host built; else built here.
if [ -n "${ELY_SHARE:-}" ]; then
  for part in engines reference; do
    # Never mix this clone's own build with shared engines.
    if [ "$os" != windows ] && { [ -e "$part" ] || [ -L "$part" ]; }; then
      # A sentinel keeps a trailing newline in the target.
      target=$( (readlink "$part" || true) && printf x)
      target=${target%x}
      [ "$target" = "$ELY_SHARE/$part"$'\n' ] || {
        echo "ci.sh: $root/$part is not a link to $ELY_SHARE/$part; remove it" >&2
        exit 1
      }
    fi
    if [ ! -e "$part" ]; then
      case $os in
        windows) powershell -NoProfile -Command "New-Item -ItemType Junction -Path '$part' -Target '$(cygpath -w "$ELY_SHARE/$part")' | Out-Null" ;;
        *) ln -s "$ELY_SHARE/$part" "$part" ;;
      esac
    fi
  done
else
  scripts/fetch-references.sh
  [ -z "$pinned_ssl" ] || ${inside[@]+"${inside[@]}"} scripts/build-openssl.sh
  ${inside[@]+"${inside[@]}"} scripts/build-mupdf.sh
  ${inside[@]+"${inside[@]}"} scripts/build-git.sh
  ${inside[@]+"${inside[@]}"} scripts/fetch-git-lfs.sh
  ${inside[@]+"${inside[@]}"} scripts/fetch-tessdata.sh
  ${inside[@]+"${inside[@]}"} scripts/build-libreoffice.sh
fi

if [ -n "$pinned_ssl" ]; then
  [ -f "$root/engines/openssl/lib/pkgconfig/libcrypto.pc" ] || {
    echo "ci.sh: no pinned OpenSSL in engines/openssl; run scripts/build-openssl.sh" >&2
    exit 1
  }
fi
[ "$(uname -s)" != Darwin ] || tests/scripts/engine-arch.sh "$(uname -m)"
# One failing test and nothing ships.
scripts/release-tests.sh

dist=$root/dist
rm -rf "$dist"
batch=$(awk -F'\t' '$4 == 1 { print $1 }' packaging/apps.tsv)
case $os in
  macos) packaging/macos/release.sh 1 ;;
  windows)
    for slug in $batch; do packaging/windows/installer.sh "$slug"; done
    packaging/windows/bundle.sh
    ;;
  linux) for slug in $batch; do ${inside[@]+"${inside[@]}"} packaging/linux/package.sh "$slug"; done ;;
  *)
    echo "ci.sh: no release for $os" >&2
    exit 1
    ;;
esac

# Windows checkouts flatten symlinks; Linux and macOS write source.
if [ "$os" = linux ]; then
  packaging/source.sh "$dist"
fi

# The apps take no manifest without its signature.
${inside[@]+"${inside[@]}"} packaging/sign.sh "$dist"/*.json

# The tag's release, made by whichever platform comes first.
repo=ELY-Open-Source/ElyOffice
api=https://api.github.com/repos/$repo
auth=(-H "Authorization: Bearer $ELY_RELEASE_TOKEN" -H "Accept: application/vnd.github+json" -H "X-GitHub-Api-Version: 2022-11-28")
release_id() { curl -fsS "${auth[@]}" "$api/releases/tags/$tag" | python3 -c 'import json, sys; print(json.load(sys.stdin)["id"])' | tr -d '\r'; }
if ! id=$(release_id 2>/dev/null); then
  body="{\"tag_name\": \"$tag\", \"name\": \"Ely Office $version\"}"
  # Another platform may make it first (422); the read decides.
  curl -fsS "${auth[@]}" -H "Content-Type: application/json" -d "$body" "$api/releases" >/dev/null || true
  id=$(release_id)
fi
names=()
for file in "$dist"/*; do
  [ -f "$file" ] || continue
  name=$(basename "$file")
  # Linux alone publishes the source pair.
  if [ "$os" != linux ] && [[ $name == *-source.tar.gz ]]; then
    echo "ci.sh: $name is Linux's to upload"
    continue
  fi
  names+=("$name")
done
# Asset ids and names, 100 per page; strip CRLF.
have=
for ((page = 1; ; page++)); do
  got=$(curl -fsS "${auth[@]}" "$api/releases/$id/assets?per_page=100&page=$page" |
    python3 -c 'import json, sys; print("\n".join("%s\t%s" % (a["id"], a["name"]) for a in json.load(sys.stdin)))' | tr -d '\r')
  [ -n "$got" ] || break
  have+=$got$'\n'
done
# A rerun replaces its own uploads, keeps others'.
cd "$dist"
for name in "${names[@]}"; do
  for old in $(awk -F'\t' -v name="$name" '$2 == name { print $1 }' <<<"$have"); do
    echo "ci.sh: replacing $name"
    curl -fsS "${auth[@]}" -X DELETE "$api/releases/assets/$old" >/dev/null
  done
  echo "ci.sh: uploading $name"
  curl -fsS "${auth[@]}" -H "Content-Type: application/octet-stream" --data-binary "@$name" \
    "https://uploads.github.com/repos/$repo/releases/$id/assets?name=$name" >/dev/null
done
echo "ci.sh: $os uploaded to $tag"
