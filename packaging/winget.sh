#!/usr/bin/env bash
# winget.sh <tag>: the ZacharyZhang.ElyOffice manifest for that release.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
tag=${1:?the release tag}
# ELY_WINGET_DOWNLOAD replaces the release URL; tests use it.
download=${ELY_WINGET_DOWNLOAD:-https://github.com/ELY-Open-Source/ElyOffice/releases/download}/$tag
out=${ELY_DIST:-$root/dist}/winget
manifests=$(mktemp -d)
trap 'rm -rf "$manifests"' EXIT
# Each first-batch manifest names the one setup.
for slug in $(awk -F'\t' '$4 == 1 { print $1 }' "$root/packaging/apps.tsv"); do
  curl -fsSL -o "$manifests/$slug.json" "$download/$slug-windows-x86_64.json"
done
rm -rf "$out"
python3 - "$tag" "$manifests" "$out" "$root/packaging/apps.tsv" <<'EOF'
import glob, json, os, sys
tag, manifests, out, apps = sys.argv[1:]
schema = "1.10.0"
seen = {}
for path in sorted(glob.glob(f"{manifests}/*.json")):
    manifest = json.load(open(path))
    if "v" + manifest["version"] != tag:
        sys.exit(f"winget.sh: {os.path.basename(path)} is {manifest['version']}, not {tag}")
    seen[path] = (manifest["version"], manifest["assets"]["windows-x86_64"])
if len({json.dumps(s, sort_keys=True) for s in seen.values()}) != 1:
    sys.exit("winget.sh: the apps' manifests name different setups")
version, asset = next(iter(seen.values()))
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
