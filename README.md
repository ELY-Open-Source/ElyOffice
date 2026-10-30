# Ely Office

Offline-first desktop apps in Rust and GPUI. Files stay in your folders; no account, no cloud service.

Download from [office.elygpui.com](https://office.elygpui.com) or the [latest release](https://github.com/ELY-Open-Source/ElyOffice/releases/latest): Windows, macOS (Apple silicon and Intel) and Linux (AppImage, deb, rpm, Arch, Nix; Debian 11 and Ubuntu 20.04 or newer).

| App | Does | Engine |
| --- | --- | --- |
| ElyDoc | Word processing (DOCX, ODT) | LibreOfficeKit |
| ElyTable | Spreadsheets (XLSX, ODS, CSV) | LibreOfficeKit |
| ElyShow | Presentations (PPTX, ODP) | LibreOfficeKit, system media |
| ElyPDF | PDF reading and editing, OCR, signatures | MuPDF, Tesseract |
| ElyMail | Mail over IMAP, POP3 and SMTP; follows the system proxy | own IMAP and POP3, mail-send, SQLite |
| ElyTree | Git client | Git |
| ElyMark | Markdown | markdown-rs, RaTeX, mermaid-rs-renderer |
| ElyAgent | Second release | Kigi CLI |

ElyMail signs in to Microsoft with OAuth, and to Gmail and iCloud with an app password.

## Build

Install Rust with [rustup](https://rustup.rs) (`rust-toolchain.toml` pins the version), then the system packages:

- macOS: Xcode with its Metal toolchain (`xcodebuild -downloadComponent MetalToolchain`); gpui compiles its shaders with `xcrun metal`.
- Windows: Visual Studio Build Tools with the C++ workload (MSVC and a Windows SDK); gpui compiles its shaders with the SDK's `fxc.exe`.
- Linux, one line per family:

```sh
sudo apt install build-essential pkg-config libdbus-1-dev libfontconfig-dev libxcb1-dev libxkbcommon-x11-dev
sudo dnf install gcc make pkgconf-pkg-config dbus-devel fontconfig-devel libxcb-devel libxkbcommon-x11-devel
sudo pacman -S --needed base-devel dbus fontconfig libxcb libxkbcommon-x11
```

`scripts/doctor.sh` checks each of these, and the tools the engine scripts below use.

```sh
scripts/doctor.sh                 # toolchain versions
scripts/fetch-references.sh       # upstream sources at pinned commits
scripts/build-libreoffice.sh      # patched LibreOffice 26.2.6.3 (about an hour)
scripts/build-mupdf.sh            # patched MuPDF 1.28.5
scripts/build-git.sh && scripts/fetch-git-lfs.sh
scripts/fetch-tessdata.sh
cargo build -p ely-lok --bins -p ely-pdf-worker
cargo run -p ely-doc              # or ely-table, ely-show, ely-pdf, ely-mail, ely-tree, ely-mark
```

ElyMail needs a Microsoft client ID at build time: put `ELY_MICROSOFT_CLIENT_ID=<id>` in `cred.txt` at the repository root (git ignores it).

The scripts run on macOS, on Windows (LibreOffice under Cygwin, MuPDF with MSBuild) and on Linux inside `packaging/linux/container.sh`, an AlmaLinux 8 image that keeps the binaries on glibc 2.28.

## Release

Pushing a `v*` tag runs `.github/workflows/release.yml`. Self-hosted runners build macOS (arm64, and Intel under Rosetta), Windows and Linux with `scripts/ci.sh` and upload each app's packages, update manifests and the source archives to the GitHub release. macOS builds are signed and notarized.

## Tests

`cargo test --workspace`; the script tests in `tests/scripts` (the window tests need macOS Accessibility permission; Linux tests run in containers); the benchmarks `tests/perf/bench.py` and `tests/perf/folder-search.rs`.

## Documents

| | |
| --- | --- |
| `PRD.md` | Requirements |
| `agents.md` | Stack, rules and commands; the single source of truth |
| `tasks/TASKS.md` | Progress |
| `docs/decisions.md` | Decision log, by date |
| `docs/help.md` | Help: opening and saving, panels, recovery, offline limits, repositories, mail |
| `docs/compatibility.md` | Compatibility list for the current release |
| `docs/accessibility.md` | Accessibility and language review |
| `docs/perf/` | Benchmark records |
| `docs/licenses.md` | Licences and sources |
| `CONTRIBUTING.md` | Opening a pull request |

## Licence

MIT (`LICENSE`). Third-party components keep their own licences; see `docs/licenses.md`.
