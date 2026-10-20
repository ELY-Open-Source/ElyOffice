# Ely Office

Offline-first desktop apps in Rust and GPUI. Files stay in your folders; no account, no cloud service.

| App | Does | Engine |
| --- | --- | --- |
| ElyDoc | Word processing (DOCX, ODT) | LibreOfficeKit |
| ElyTable | Spreadsheets (XLSX, ODS, CSV) | LibreOfficeKit |
| ElyShow | Presentations (PPTX, ODP) | LibreOfficeKit, system media |
| ElyPDF | PDF reading and editing, OCR, signatures | MuPDF, Tesseract |
| ElyTree | Git client | Git |
| ElyMark | Markdown | markdown-rs, RaTeX, mermaid-rs-renderer |
| ElyMail, ElyAgent | Second release | — |

## Build (macOS)

```sh
scripts/doctor.sh                 # toolchain versions
scripts/fetch-references.sh       # upstream sources at pinned commits
scripts/build-libreoffice.sh      # patched LibreOffice 26.2.6.3 (about an hour)
scripts/build-mupdf.sh            # patched MuPDF 1.28.5
scripts/build-git.sh && scripts/fetch-git-lfs.sh
scripts/fetch-tessdata.sh
cargo build -p ely-lok --bins -p ely-pdf-engine --bins
cargo run -p ely-doc              # or ely-table, ely-show, ely-pdf, ely-tree, ely-mark
```

Packages: `packaging/macos/release.sh` writes each app's `.app` and `.dmg`, the source archive and the upstream source archive (patched MuPDF, Git) to `dist/`. Linux and Windows recipes are in `packaging/linux` and `packaging/windows`; they have not been run on those systems.

## Tests

`cargo test --workspace`, the window scripts in `tests/scripts` (macOS, Accessibility permission), and the benchmark `tests/perf/bench.py`.

## Documents

| | |
| --- | --- |
| `docs/help.md` | Help: opening and saving, recovery, offline limits, repositories |
| `docs/compatibility.md` | Versioned compatibility list |
| `docs/accessibility.md` | Accessibility and language review |
| `docs/perf/` | Benchmark records |
| `docs/licenses.md` | Licences and sources |

## Licence

MIT (`LICENSE`). Third-party components keep their own licences; see `docs/licenses.md`.
