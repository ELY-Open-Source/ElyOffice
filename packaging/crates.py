"""Notices of every crate built into packages. Usage: crates.py <package>..."""
import json
import pathlib
import subprocess
import sys

NAMES = ("licen", "copying", "notice", "copyright", "unlicense")

root = pathlib.Path(__file__).resolve().parent.parent
standard = root / "packaging" / "licenses"
host = next(line.split()[1] for line in subprocess.run(
    ["rustc", "-vV"], check=True, capture_output=True, text=True).stdout.splitlines() if line.startswith("host:"))
meta = json.loads(subprocess.run(
    ["cargo", "metadata", "--format-version", "1", "--locked",
     "--manifest-path", str(root / "Cargo.toml")],
    check=True, capture_output=True, text=True).stdout)
by_id = {package["id"]: package for package in meta["packages"]}
own = set(meta["workspace_members"])
members = [pid for pid in own if by_id[pid]["name"] in sys.argv[1:]]
if len(members) != len(sys.argv) - 1:
    sys.exit(f"crates.py: unknown package among {sys.argv[1:]}")
# Cargo decides which normal edges link on this target.
tree = subprocess.run(
    ["cargo", "tree", "--locked", "-e", "normal", "--target", host, "--prefix", "none", "-f", "{p}",
     "--manifest-path", str(root / "Cargo.toml"), *[arg for pid in members for arg in ("-p", pid)]],
    check=True, capture_output=True, text=True).stdout
linked = {tuple(line.split()[:2]) for line in tree.splitlines() if line.strip()}
wanted = {pid for pid, package in by_id.items() if (package["name"], "v" + package["version"]) in linked}

out, missing = [], []
for pid in sorted(wanted - own, key=lambda pid: (by_id[pid]["name"], by_id[pid]["version"])):
    package = by_id[pid]
    folder = pathlib.Path(package["manifest_path"]).parent
    files = sorted(path for path in folder.iterdir()
                   if path.is_file() and path.name.lower().startswith(NAMES))
    if package.get("license_file"):
        files.append(folder / package["license_file"])
    head = f"{package['name']} {package['version']} — {package['license'] or 'no licence field'}"
    out.append("=" * 78 + f"\n{head}\n" + "=" * 78)
    if files:
        out += [f"--- {path.name}\n{path.read_text(errors='replace')}" for path in dict.fromkeys(files)]
    else:
        # No file shipped: the expression's standard text, with its authors.
        ids = (package["license"] or "").replace("(", " ").replace(")", " ").replace("/", " ").split()
        text = next((standard / f"{spdx}.txt" for spdx in ids if (standard / f"{spdx}.txt").is_file()), None)
        if text is None:
            missing.append(head)
            continue
        authors = ", ".join(package["authors"]) or package["name"] + " developers"
        out.append(f"Copyright (c) {authors}\n--- {text.stem}\n{text.read_text()}")
if missing:
    sys.exit("crates.py: no licence text for:\n  " + "\n  ".join(missing))
print("\n".join(out))
