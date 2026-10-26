#!/usr/bin/env python3
"""agents.md names resolve to public items in the library."""
import re
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[2]
lib = root / "reference/Ely-GPUI-Components"
ITEM = re.compile(r"^pub\s+(?:struct|enum|fn|trait|type|const)\s+([A-Za-z_]\w*)", re.M)
USE = re.compile(r"pub\s+use\s+([^;]+);")
CLAIM = re.compile(r"([a-z_]+)::\{([^}]*)\}")
FACADE = (lib / "src/lib.rs").read_text()
CORE = re.search(r"pub\s+use\s+ely_core::\{([^}]*)\}", FACADE)
CRATE = re.compile(r"pub\s+use\s+(ely_\w+)\s+as\s+(\w+);")


def entry(module: str) -> Path:
    """The file that declares `module`'s public names."""
    if CORE and module in (n.strip() for n in CORE.group(1).split(",")):
        base = lib / "crates/ely-core/src"
        for path in (base / module / "mod.rs", base / f"{module}.rs"):
            if path.is_file():
                return path
    for krate, name in CRATE.findall(FACADE):
        if name == module:
            return lib / "crates" / krate.replace("_", "-") / "src/lib.rs"
    sys.exit(f"FAIL: no module {module} in the component library")


def public_names(module: str) -> set[str]:
    path = entry(module)
    text = path.read_text()
    names = set(ITEM.findall(text))
    for use in USE.findall(text):
        use = " ".join(use.split())
        if use.endswith("::*"):
            names |= glob_names(path.parent, use[:-3], module)
            continue
        group = re.match(r".+::\{(.*)\}$", use)
        parts = group.group(1).split(",") if group else [use]
        names |= {p.split(" as ")[-1].rsplit("::", 1)[-1].strip() for p in parts if p.strip()}
    return names


def glob_names(base: Path, sub: str, module: str) -> set[str]:
    for path in (base / f"{sub}.rs", base / sub / "mod.rs"):
        if path.is_file():
            return set(ITEM.findall(path.read_text()))
    sys.exit(f"FAIL: cannot resolve {module}: pub use {sub}::*")


count = 0
for module, listed in CLAIM.findall((root / "agents.md").read_text()):
    public = public_names(module)
    for name in (n.strip() for n in listed.split(",")):
        if name not in public:
            sys.exit(f"FAIL: {module}::{name} is not public in the component library")
        count += 1

if count == 0:
    sys.exit("FAIL: agents.md names no components")
print(f"agents.md: {count} component names are public")
