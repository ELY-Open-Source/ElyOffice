# Benchmark 2026-10-28

Format: docs/perf/README.md.

```json
{
  "date": "2026-10-28",
  "machine": "AMD-Ryzen-9-9950X3D-16-Core-Processor-Linux",
  "cpu": "AMD Ryzen 9 9950X3D 16-Core Processor",
  "cores": "32",
  "memory_gb": 45,
  "gpu": "Mesa lavapipe under Xvfb",
  "system": "Ubuntu 26.04.1 LTS, kernel 6.18.54.1-microsoft-standard-WSL2",
  "fonts": 311,
  "builds": {
    "ely-doc": "4c1bde9b3abb9737",
    "ely-table": "21f42407cd13f76e",
    "ely-show": "2c554bfdada34b3c",
    "ely-pdf": "e623292007bcdcb7",
    "ely-tree": "51fdd82f29104222",
    "ely-mark": "9f0bf659353d2f58"
  },
  "samples": {
    "doc.docx": "e994ed07147811d57231bfdf44dc711bd1cf0ff74a6b76f4bf2564637893e994",
    "sheet.xlsx": "9e2cbe2526a4da1c641db1fb0f36ba4d5cc5c4377eb6a933770a5115c977e864",
    "pages.pdf": "d9852531dbbd3699642398c2b454246a361c92ddad5f7dc7483af371f9ce40cb",
    "notes.md": "28e15bd4ecd655e20f01f7eb93fe84a5f60cf7bb298e3744756d670db762d95c",
    "repo": "1ad0662eea314739d0b85ae5ae9258620d271c75"
  }
}
```

| PERF | Case | Runs | P50 ms | P95 ms | Target ms | Total memory MB | Budget MB |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PERF-01 | ely-doc first run | 1 | 452 | 452 ✓ | 3000 |  |  |
| PERF-01 | ely-doc daily start | 30 | 247 | 258 ✓ | 3000 |  |  |
| PERF-01 | ely-table first run | 1 | 230 | 230 ✓ | 3000 |  |  |
| PERF-01 | ely-table daily start | 30 | 243 | 255 ✓ | 3000 |  |  |
| PERF-01 | ely-show first run | 1 | 246 | 246 ✓ | 3000 |  |  |
| PERF-01 | ely-show daily start | 30 | 242 | 251 ✓ | 3000 |  |  |
| PERF-01 | ely-pdf first run | 1 | 245 | 245 ✓ | 3000 |  |  |
| PERF-01 | ely-pdf daily start | 30 | 242 | 258 ✓ | 3000 |  |  |
| PERF-01 | ely-tree first run | 1 | 247 | 247 ✓ | 3000 |  |  |
| PERF-01 | ely-tree daily start | 30 | 242 | 253 ✓ | 3000 |  |  |
| PERF-01 | ely-mark first run | 1 | 237 | 237 ✓ | 3000 |  |  |
| PERF-01 | ely-mark daily start | 30 | 247 | 258 ✓ | 3000 |  |  |
| PERF-01 | ely-doc blank document ready | 30 | 351 | 372 ✓ | 3000 |  |  |
| PERF-01 | ely-table blank document ready | 30 | 355 | 375 ✓ | 3000 |  |  |
| PERF-01 | ely-show blank document ready | 30 | 415 | 433 ✓ | 3000 |  |  |
| PERF-02 | ElyDoc key to new tile | 30 | 13 | 20 ✓ | 100 |  |  |
| PERF-02 | main thread busy >100 ms: 0 times | 1 | 0 | 0 | 100 |  |  |
| PERF-03 | ElyDoc doc.docx | 30 | 877 | 1011 ✓ | 5000 | 423 |  |
| PERF-04 | ElyTable sheet.xlsx | 30 | 476 | 486 ✓ | 5000 | 417 |  |
| PERF-05 | ElyPDF pages.pdf | 30 | 1876 | 1924 ✓ | 3000 | 208 |  |
| PERF-07 | ElyMark notes.md | 30 | 302 | 323 ✓ | 2000 | 206 ✓ | 400 |
| PERF-06 | ElyTree history, press to first screen | 30 | 787 | 807 ✓ | 1000 |  |  |
| PERF-06 | ElyTree query 'part 42', first results | 30 | 768 | 784 ✓ | 1000 |  |  |

PERF-08, ElyDoc creating and closing a document 40 times (total memory MB / open files / kernel documents):

before: 373/45/1

381/45/1 382/45/1 383/45/1 383/45/1 383/45/1 383/45/1 384/45/1 384/45/1 384/45/1 384/45/1 384/45/1 384/45/1 384/45/1 384/45/1 384/45/1 385/45/1 385/45/1 385/45/1 385/45/1 386/45/1 386/45/1 387/45/1 387/45/1 387/45/1 387/45/1 387/45/1 387/45/1 388/45/1 388/45/1 388/45/1 388/45/1 388/45/1 388/45/1 388/45/1 388/45/1 388/45/1 389/45/1 389/45/1 389/45/1 389/45/1

PERF-09 (Agent sessions) belongs to the second release.
