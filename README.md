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
| ElyMark | Markdown | pulldown-cmark, RaTeX, mermaid-rs-renderer; MuPDF for PDF export |
| ElyAgent | Second release | Kigi CLI |

ElyMail signs in to Microsoft with OAuth, to Gmail and iCloud with an app password, and to QQ Mail, 163 and 126 with an authorization code.

## Build

Install Rust with [rustup](https://rustup.rs) (`rust-toolchain.toml` pins the version), then the system packages. Every system also needs Git, Python 3, curl, Node.js 22.18 or newer (the first to run the site's TypeScript tests unflagged) and pnpm (`check.sh` and the site use the last two).

- macOS: Xcode (gpui compiles its shaders when the app starts, so the separate Metal toolchain is not needed); then `brew install cmake ninja pkg-config make gnu-sed bison flex gettext autoconf automake node pnpm bash` for LibreOffice and the checks (`build-libreoffice.sh` puts Homebrew's GNU make, sed, bison, flex and gettext first; the scripts need bash 5, since macOS's own bash 3.2 exits 0 after a `set -u` abort). The Intel release runs under Rosetta with an x86_64 bash 5 that `scripts/build-bash.sh` builds into `~/ely-share/bash-x86_64`.
- Windows: Visual Studio 2022 with the C++ workload (MSVC, a Windows SDK and CMake with Ninja); gpui compiles its shaders with the SDK's `fxc.exe`. Git for Windows supplies bash and curl. LibreOffice builds under Cygwin in `C:\cygwin64` (`ELY_CYGWIN` elsewhere) with autoconf, automake and LibreOffice's GNU make in `/opt/lo/bin`, as its [Windows build guide](https://wiki.documentfoundation.org/Development/BuildingOnWindows) sets up.
- Linux, one line per family, then pnpm. LibreOffice builds inside `packaging/linux/container.sh`, hence Docker. Ubuntu's own Node.js is too old, so Node.js 22 comes from NodeSource:

```sh
sudo apt install build-essential pkg-config git curl python3 docker.io libdbus-1-dev libfontconfig-dev libxcb1-dev libxkbcommon-x11-dev
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash - && sudo apt install nodejs
sudo dnf install gcc make pkgconf-pkg-config git curl python3 nodejs npm moby-engine dbus-devel fontconfig-devel libxcb-devel libxkbcommon-x11-devel
sudo pacman -S --needed base-devel git curl python nodejs npm docker dbus fontconfig libxcb libxkbcommon-x11
sudo npm install -g pnpm
```

`scripts/doctor.sh` checks each of these for the system it runs on.

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

## Experimental R macros

ElyTable can run local R macros on a selected range, including values, header
formatting and column widths. See [docs/r-macros.md](docs/r-macros.md) for
setup, the small API and its limits. Requires R (Rscript) and a LOK engine
built with patch 0081.

## Release

Pushing a `v*` tag runs `.github/workflows/release.yml`, five jobs that wait for no other. Self-hosted runners build macOS (arm64, and Intel under Rosetta), Windows and Linux x86_64 (in WSL) with `scripts/ci.sh`; GitHub's hosted `ubuntu-24.04-arm` runner builds Linux arm64 with the same container. Each job uploads its packages and update manifests; x86_64 Linux alone uploads the source archives. Each Linux job also builds `ely-office`, one deb, rpm and Arch package that holds every app and one copy of the LibreOffice kernel for ElyDoc, ElyTable and ElyShow (`packaging/linux/bundle.sh`). macOS builds are signed and notarized.

## Tests

`cargo test --workspace`; the script tests in `tests/scripts`, listed in its README (the window tests need macOS Accessibility permission; Linux tests run in containers); the benchmarks `tests/perf/bench.py` and `tests/perf/folder-search.rs`.

## Documents

| | |
| --- | --- |
| `PRD.md` | Requirements |
| `agents.md` | Stack, rules and commands; the single source of truth |
| `tests/scripts/README.md` | Each test script: what it checks and what it needs |
| `tests/fixtures/README.md` | Each shared test sample |
| `tasks/TASKS.md` | Progress |
| `docs/decisions.md` | Decision log, by date |
| `docs/help.md` | Help: opening and saving, panels, recovery, offline limits, repositories, mail |
| `patches/UPSTREAM.md` | The patches that fix upstream defects and could go upstream |
| `docs/compatibility.md` | Compatibility list, last verified for 0.1.11 |
| `docs/accessibility.md` | Accessibility and language review |
| `docs/perf/` | Benchmark records |
| `docs/licenses.md` | Licences and sources |
| `docs/acceptance.md` | Audit acceptance: each item, its task and its last review |
| `CONTRIBUTING.md` | Opening a pull request |

## Licence

MIT (`LICENSE`). Third-party components keep their own licences; see `docs/licenses.md`.
