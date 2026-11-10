# PERF-10 load 2026-11-10, 20 documents per app

Format: docs/perf/README.md.

ElyDoc, ElyTable, ElyShow, ElyMark and ElyPDF ran at the same time, each on its own Xvfb display and in its own `load.py` (ElyPDF: `pdfs.py`, its own record). Every document is the big sample (`--big-every 1`). The command line edits all twenty documents of an app at once, the one being typed in too, three rounds; then saves them all.

```json
{
  "date": "2026-11-10",
  "machine": "AMD-Ryzen-9-9950X3D-16-Core-Processor-Linux",
  "cpu": "AMD Ryzen 9 9950X3D 16-Core Processor",
  "cores": "32",
  "memory_gb": 91,
  "gpu": "Mesa lavapipe under Xvfb",
  "system": "Omarchy, kernel 7.2.15-1-omarchy",
  "fonts": 917,
  "docs": 20,
  "big every": 1,
  "rounds": 3,
  "keys": 30,
  "builds": {
    "ely-doc": "df0b49d08094ae4a",
    "ely-lok-worker": "62874cd879aadfbf",
    "ely-table": "d3248ff4a3808a08",
    "ely-show": "db2eb2495f082d1f",
    "ely-mark": "3e7d3357e8d8ea8c"
  },
  "lok": "engines/"
}
```

| App | Open all ms | Open one more ms | Memory MB open / edited | Free GB, lowest | Keys idle: P50 / P95 / max ms | Frames idle: over 8.3 ms / all, dirty to shown | Stalls idle | Keys editing: P50 / P95 / max ms | Frames editing | Stalls editing | Command edit P50 / P95 ms | Save P50 / max ms | Stalls whole run |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ElyDoc | 809 | 156 | 486 / 986 | 55 | 30: 17 / 37 / 51 | 62/62, p95 ≤39.9, max 43.5 | 0 | 30: 17 / 385 / 449 | 148/148, p95 ≤36.7, max 44.7 | 0 | 94 / 6767 | 622 / 1049 | 0, max 0 |
| ElyTable | 833 | 106 | 504 / 1046 | 49 | 30: 14 / 19 / 31 | 58/58, p95 ≤34.7, max 36.6 | 0 | 30: 13 / 179 / 182 | 78/78, p95 ≤34.5, max 36.0 | 0 | 54 / 3243 | 3435 / 6541 | 0, max 0 |
| ElyShow | 981 | 332 | 612 / 1813 | 53 | 30: 16 / 38 / 39 | 65/65, p95 ≤35.5, max 37.0 | 0 | 30: 86 / 168 / 234 | 110/110, p95 ≤34.9, max 34.9 | 0 | 199 / 8697 | 2753 / 5056 | 0, max 0 |
| ElyMark | 732 | 50 | 566 / 936 | 49 | 30: 25 / 34 / 35 | 178/178, p95 ≤33.0, max 36.6 | 0 | 30: 28 / 32 / 32 | 187/187, p95 ≤52.8, max 57.3 | 0 | 64 / 74 | 97 / 102 | 0, max 0 |

Problems:

- ElyDoc: PERF-02: 6 of 30 keys over 100 ms editing, max 449 ms
- ElyTable: PERF-02: 4 of 30 keys over 100 ms editing, max 182 ms
- ElyShow: PERF-02: 13 of 30 keys over 100 ms editing, max 234 ms

What the problems are: a key that arrives while the engine loads a big document for the command line, or replaces or recalculates across one, waits until that is done. No key was lost, every edit and every key was read back from the saved files, the main thread never stalled, no worker ended.
