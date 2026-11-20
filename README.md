# Ely Office

Offline-first desktop apps in Rust and GPUI. Files stay in your folders; no account, no cloud service.

Download from [office.elygpui.com](https://office.elygpui.com) or the [latest release](https://github.com/ELY-Open-Source/ElyOffice/releases/latest): Windows x64, macOS (Apple silicon and Intel) and Linux x86_64 (AppImage, deb, rpm, Arch, Nix; Debian 11 and Ubuntu 20.04 or newer). The latest release, 0.1.16, has an installer for each app (macOS `.dmg`, Windows setup, Linux AppImage, deb, rpm and Arch package), an ElyOffice installer with all of them (macOS `.dmg`, Windows setup) and an `ely-office` deb, rpm and Arch package; each app updates on its own. The tree now builds one installer per platform with every app (macOS `.pkg`), updated as a set; no release is built that way yet.

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

- macOS: Xcode (gpui compiles its shaders when the app starts, so the separate Metal toolchain is not needed); then `brew install cmake ninja pkg-config openssl make gnu-sed bison flex gettext autoconf automake node pnpm bash` for LibreOffice, MuPDF and the checks, run from a shell with Homebrew on its `PATH` (`build-libreoffice.sh` puts Homebrew's GNU make, sed, bison, flex and gettext first; the scripts need bash 5, since macOS's own bash 3.2 exits 0 after a `set -u` abort). Apple silicon links MuPDF against Homebrew's OpenSSL, found through `pkg-config`; Intel uses `scripts/build-openssl.sh`, as Linux does. The Intel release runs under Rosetta with an x86_64 bash 5 that `scripts/build-bash.sh` builds into `~/ely-share/bash-x86_64`.
- Windows: Visual Studio 2022 with the C++ workload (MSVC, a Windows SDK and CMake with Ninja) and clang-cl, from its "C++ Clang tools" component or LLVM in `C:\Program Files\LLVM` (LibreOffice builds Skia with it); gpui compiles its shaders with the SDK's `fxc.exe`. Git for Windows supplies bash and curl; run the scripts from its bash. MuPDF links a static OpenSSL from vcpkg in `engines/vcpkg`: `git clone --depth 1 --branch 2026.07.29 https://github.com/microsoft/vcpkg.git engines/vcpkg`, then `engines/vcpkg/bootstrap-vcpkg.bat -disableMetrics` and `engines/vcpkg/vcpkg install openssl:x64-windows-static`. `scripts/build-git.sh` fetches MinGit on Windows rather than building Git. LibreOffice builds under Cygwin in `C:\cygwin64` (`ELY_CYGWIN` elsewhere) with autoconf, automake and LibreOffice's GNU make in `/opt/lo/bin`, as its [Windows build guide](https://wiki.documentfoundation.org/Development/BuildingOnWindows) sets up.
- Linux, one line per family, then pnpm. The engines build inside `packaging/linux/container.sh`, hence Docker. Ubuntu's own Node.js is too old, so Node.js 22 comes from NodeSource:

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
scripts/build-openssl.sh          # Linux and Intel macOS: static OpenSSL for MuPDF
scripts/build-mupdf.sh            # patched MuPDF 1.28.5
scripts/build-git.sh && scripts/fetch-git-lfs.sh
scripts/fetch-tessdata.sh
scripts/build-libreoffice.sh      # patched LibreOffice 26.2.6.3 (21 minutes on 16 cores)
export PKG_CONFIG_PATH=$PWD/engines/openssl/lib/pkgconfig   # Linux and Intel macOS
cargo build -p ely-lok --bins -p ely-pdf-worker
cargo run -p ely-doc              # or ely-table, ely-show, ely-pdf, ely-mail, ely-tree, ely-mark
```

On Linux, run each script after `packaging/linux/container.sh` (as in `packaging/linux/container.sh scripts/build-mupdf.sh`, which is how `scripts/ci.sh` runs them): an AlmaLinux 8 image that keeps the binaries on glibc 2.28. Cargo runs on the host. On Windows, LibreOffice builds under Cygwin and MuPDF with MSBuild.

Microsoft sign-in in ElyMail needs a client ID at build time: put `ELY_MICROSOFT_CLIENT_ID=<id>` in `cred.txt` at the repository root (git ignores it), or set the variable. Without it ElyMail still builds and says Microsoft sign-in is not in this build; `scripts/ci.sh` refuses to build a release without it.

## Experimental R macros

ElyTable can run local R macros on a selected range, including values, header
formatting and column widths. See [docs/r-macros.md](docs/r-macros.md) for
setup, the small API and its limits. Requires R (Rscript) and a LOK engine
built with patch 0081.

## Release

Pushing a `v*` tag runs `.github/workflows/release.yml`, four jobs that wait for no other, all on our own machines. Each job first checks the gates and builds nothing if one fails: the release notes hold only New and Fixes, and the release record `docs/releases/<version>.md` states how many fixes were not reviewed again and why a release comes less than seven days after the last (`scripts/release-gate.py`); `docs/perf/` holds this version's benchmark for the system and its first opens after install and after reboot, with a reason for each case over 30% slower (`tests/perf/baseline.py`); `pr.yml` passed on the tagged commit; the host's cargo, and the container's on Linux, is [huacnlee/cargo](https://huacnlee.github.io/cargo/). Self-hosted runners build macOS (arm64, and Intel under Rosetta), Windows x64 and Linux x86_64 (in the AlmaLinux 8 container on an Omarchy host) with `scripts/ci.sh`; there is no arm64 release for Linux or Windows. Each job uploads its packages and update manifests; x86_64 Linux alone uploads the source archives. The Linux job also builds `ely-office`, one deb, rpm and Arch package that holds every app and one copy of the LibreOffice kernel for ElyDoc, ElyTable and ElyShow (`packaging/linux/bundle.sh`). macOS builds are signed and notarized.

## Tests

The test report is [docs/qa.md](docs/qa.md): what proves each of PRD's QA-01 to QA-28 today, and what is missing. [docs/test-map.md](docs/test-map.md) lists each PRD item's tasks and the tests that cover it. Known limitations are in [docs/compatibility.md](docs/compatibility.md#已知限制).

`cargo test --workspace`; the script tests in `tests/scripts`, listed in its README (the window tests need macOS Accessibility permission; Linux tests run in containers); the benchmarks `tests/perf/bench.py` and `tests/perf/folder-search.rs`; the first opens of the installed apps `tests/perf/firsts.py install` and `reboot` (on Windows it records Microsoft Office's beside them, ungated); the idle engines `tests/perf/idle.py` (no CPU in ten minutes, memory within a tenth of `tests/perf/idle.json`; part of `scripts/release-tests.sh`); the load tests `tests/perf/load.py` (many large documents open and edited at once, the command line editing the one being typed in) and `tests/perf/pdfs.py`, on Linux through `tests/perf/linux.sh`.

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
| `docs/compatibility.md` | Compatibility list and known limitations: each app section last verified for 0.1.11; Office on the web opened 30 Ely-saved files and 18 large, odd and extreme ones at 0.1.15 |
| `docs/qa.md` | Test report: the evidence for each of QA-01 to QA-28 |
| `docs/test-map.md` | Each PRD item's tasks and tests (`scripts/test-map.py`) |
| `docs/gaps.md` | What the component library lacks and where Ely builds it |
| `scripts/README.md` | Every script: build, check, package, release |
| `docs/accessibility.md` | Accessibility and language review |
| `docs/perf/` | Benchmark records |
| `docs/licenses.md` | Licences and sources |
| `docs/acceptance.md` | Audit acceptance: each item, its task and its last review |
| `CONTRIBUTING.md` | Opening a pull request |

## Licence

MIT (`LICENSE`). Third-party components keep their own licences; see `docs/licenses.md`.
