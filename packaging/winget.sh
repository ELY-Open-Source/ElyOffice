#!/usr/bin/env bash
# winget.sh <tag>: WinGet manifests from that release's Windows manifests.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
tag=${1:?the release tag}
# ELY_WINGET_DOWNLOAD replaces the release URL; tests use it.
download=${ELY_WINGET_DOWNLOAD:-https://github.com/ELY-Open-Source/ElyOffice/releases/download}/$tag
out=${ELY_DIST:-$root/dist}/winget
rows=$(awk -F'\t' '$4 == 1 { print $1 "\t" $2 "\t" $5 "\t" $6 }' "$root/packaging/apps.tsv")
manifests=$(mktemp -d)
trap 'rm -rf "$manifests"' EXIT
while IFS=$'\t' read -r slug _ _ _; do
  curl -fsSL -o "$manifests/$slug.json" "$download/$slug-windows-x86_64.json"
done <<<"$rows"
printf '%s\n' "$rows" >"$manifests/rows.tsv"
rm -rf "$out"
python3 - "$tag" "$manifests" "$out" <<'EOF'
import json, os, sys
tag, manifests, out = sys.argv[1:]
schema = "1.10.0"
for row in open(f"{manifests}/rows.tsv").read().splitlines():
    slug, name, license, summary = row.split("\t")
    manifest = json.load(open(f"{manifests}/{slug}.json"))
    version = manifest["version"]
    if "v" + version != tag:
        sys.exit(f"winget.sh: {slug}'s manifest is {version}, not {tag}")
    asset = manifest["assets"]["windows-x86_64"]
    package = f"ZacharyZhang.{name}"
    ids = f"PackageIdentifier: {package}\nPackageVersion: {version}\n"
    head = lambda kind: f"# yaml-language-server: $schema=https://aka.ms/winget-manifest.{kind}.{schema}.schema.json\n" + ids
    tail = f"ManifestVersion: {schema}\n"
    files = {
        f"{package}.yaml": head("version") + "DefaultLocale: en-US\nManifestType: version\n" + tail,
        f"{package}.installer.yaml": head("installer")
        + "InstallerType: nullsoft\nScope: user\nUpgradeBehavior: install\n"
        + f"ProductCode: Ely Office {name}\n"
        + "Installers:\n- Architecture: x64\n"
        + f"  InstallerUrl: {asset['url']}\n  InstallerSha256: {asset['sha256'].upper()}\n"
        + "ManifestType: installer\n" + tail,
        f"{package}.locale.en-US.yaml": head("defaultLocale")
        + "PackageLocale: en-US\nPublisher: ZacharyZhang\n"
        + f"PackageName: {name}\nPackageUrl: https://office.elygpui.com\n"
        + f"License: {license}\nShortDescription: {summary}\n"
        + "ManifestType: defaultLocale\n" + tail,
    }
    at = os.path.join(out, "manifests", "z", "ZacharyZhang", name, version)
    os.makedirs(at)
    for file, text in files.items():
        open(os.path.join(at, file), "w", newline="\n").write(text)
    print(at)
EOF
