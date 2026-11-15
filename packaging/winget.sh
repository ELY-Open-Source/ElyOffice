#!/usr/bin/env bash
# winget.sh <tag>: the ZacharyZhang.ElyOffice manifest for that release.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
tag=${1:?the release tag}
# ELY_WINGET_DOWNLOAD replaces the release URL; tests use it.
download=${ELY_WINGET_DOWNLOAD:-https://github.com/ELY-Open-Source/ElyOffice/releases/download}/$tag
out=${ELY_DIST:-$root/dist}/winget
manifest=$(mktemp)
trap 'rm -f "$manifest"' EXIT
curl -fsSL -o "$manifest" "$download/ely-office-windows-x86_64.json"
rm -rf "$out"
python3 - "$tag" "$manifest" "$out" "$root/packaging/apps.tsv" <<'EOF'
import json, os, sys
tag, manifest, out, apps = sys.argv[1:]
schema = "1.10.0"
manifest = json.load(open(manifest))
if "v" + manifest["version"] != tag:
    sys.exit(f"winget.sh: the manifest is {manifest['version']}, not {tag}")
version, asset = manifest["version"], manifest["assets"]["windows-x86_64"]
licenses = []
for row in open(apps).read().splitlines():
    cols = row.split("\t")
    if cols[3] == "1":
        licenses += [l for l in cols[4].split(" AND ") if l not in licenses]
package = "ZacharyZhang.ElyOffice"
ids = f"PackageIdentifier: {package}\nPackageVersion: {version}\n"
head = lambda kind: f"# yaml-language-server: $schema=https://aka.ms/winget-manifest.{kind}.{schema}.schema.json\n" + ids
tail = f"ManifestVersion: {schema}\n"
files = {
    f"{package}.yaml": head("version") + "DefaultLocale: en-US\nManifestType: version\n" + tail,
    f"{package}.installer.yaml": head("installer")
    + "InstallerType: nullsoft\nScope: user\nUpgradeBehavior: install\n"
    + "ProductCode: Ely Office\n"
    + "Installers:\n- Architecture: x64\n"
    + f"  InstallerUrl: {asset['url']}\n  InstallerSha256: {asset['sha256'].upper()}\n"
    + "ManifestType: installer\n" + tail,
    f"{package}.locale.en-US.yaml": head("defaultLocale")
    + "PackageLocale: en-US\nPublisher: ZacharyZhang\n"
    + "PackageName: Ely Office\nPackageUrl: https://office.elygpui.com\n"
    + f"License: {' AND '.join(licenses)}\n"
    + "ShortDescription: Documents, spreadsheets, presentations, PDF, mail, Git and Markdown\n"
    + "ManifestType: defaultLocale\n" + tail,
}
at = os.path.join(out, "manifests", "z", "ZacharyZhang", "ElyOffice", version)
os.makedirs(at)
for file, text in files.items():
    open(os.path.join(at, file), "w", newline="\n").write(text)
print(at)
EOF
