# Benchmark 2026-10-28

Format: docs/perf/README.md.

```json
{
  "date": "2026-10-28",
  "machine": "AMD-Ryzen-9-9950X3D-16-Core-Processor-Windows",
  "cpu": "AMD Ryzen 9 9950X3D 16-Core Processor",
  "cores": "32",
  "memory_gb": 93,
  "gpu": "GameViewer Virtual Display Adapter, AMD Radeon(TM) Graphics, NVIDIA GeForce RTX 4090",
  "system": "Microsoft Windows 11 Pro for Workstations 10.0.26300",
  "fonts": 700,
  "builds": {
    "ely-doc": "fcdf421a13590284",
    "ely-table": "3e008d6800f7ca78",
    "ely-show": "cd8e4c3a7d27f866",
    "ely-pdf": "bf1315738c4f1561",
    "ely-tree": "f8bc6bcd1637f4c3",
    "ely-mark": "85b2c9c00940963f"
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
| PERF-01 | ely-doc first run | 1 | 809 | 809 ✓ | 3000 |  |  |
| PERF-01 | ely-doc daily start | 30 | 415 | 431 ✓ | 3000 |  |  |
| PERF-01 | ely-table first run | 1 | 701 | 701 ✓ | 3000 |  |  |
| PERF-01 | ely-table daily start | 30 | 430 | 481 ✓ | 3000 |  |  |
| PERF-01 | ely-show first run | 1 | 716 | 716 ✓ | 3000 |  |  |
| PERF-01 | ely-show daily start | 30 | 432 | 452 ✓ | 3000 |  |  |
| PERF-01 | ely-pdf first run | 1 | 544 | 544 ✓ | 3000 |  |  |
| PERF-01 | ely-pdf daily start | 30 | 355 | 409 ✓ | 3000 |  |  |
| PERF-01 | ely-tree first run | 1 | 648 | 648 ✓ | 3000 |  |  |
| PERF-01 | ely-tree daily start | 30 | 387 | 474 ✓ | 3000 |  |  |
| PERF-01 | ely-mark first run | 1 | 621 | 621 ✓ | 3000 |  |  |
| PERF-01 | ely-mark daily start | 30 | 251 | 360 ✓ | 3000 |  |  |
| PERF-01 | ely-doc blank document ready | 30 | 552 | 566 ✓ | 3000 |  |  |
| PERF-01 | ely-table blank document ready | 30 | 739 | 849 ✓ | 3000 |  |  |
| PERF-01 | ely-show blank document ready | 30 | 923 | 996 ✓ | 3000 |  |  |
| PERF-02 | ElyDoc key to new tile | 30 | 8 | 26 ✓ | 100 |  |  |
| PERF-02 | main thread busy >100 ms: 0 times | 1 | 0 | 0 | 100 |  |  |
| PERF-03 | ElyDoc doc.docx | 30 | 1119 | 1156 ✓ | 5000 | 375 |  |
| PERF-04 | ElyTable sheet.xlsx | 30 | 939 | 1686 ✓ | 5000 | 292 |  |
| PERF-05 | ElyPDF pages.pdf | 30 | 330 | 364 ✓ | 3000 | 102 |  |
| PERF-07 | ElyMark notes.md | 30 | 311 | 369 ✓ | 2000 | 139 ✓ | 400 |
| PERF-06 | ElyTree history, press to first screen | 30 | 758 | 846 ✓ | 1000 |  |  |
| PERF-06 | ElyTree query 'part 42', first results | 30 | 719 | 816 ✓ | 1000 |  |  |

PERF-08, ElyDoc creating and closing a document 40 times (total memory MB / open files / kernel documents):

before: 248/1423/1

261/1428/1 261/1428/1 254/1428/1 255/1430/1 254/1430/1 261/1430/1 260/1431/1 261/1431/1 255/1431/1 256/1432/1 255/1432/1 262/1432/1 260/1432/1 262/1432/1 255/1423/1 256/1423/1 255/1423/1 262/1423/1 260/1423/1 262/1423/1 255/1423/1 256/1423/1 260/1423/1 261/1423/1 260/1423/1 256/1423/1 255/1423/1 256/1423/1 260/1423/1 262/1423/1 254/1415/1 256/1407/1 254/1407/1 261/1399/1 260/1399/1 261/1399/1 261/1399/1 262/1399/1 262/1399/1 255/1399/1

PERF-09 (Agent sessions) belongs to the second release.
