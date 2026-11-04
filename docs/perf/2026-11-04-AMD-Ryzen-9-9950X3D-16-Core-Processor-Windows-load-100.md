# PERF-10 load 2026-11-04, 100 documents per app

Format: docs/perf/README.md.

```json
{
  "date": "2026-11-04",
  "machine": "AMD-Ryzen-9-9950X3D-16-Core-Processor-Windows",
  "cpu": "AMD Ryzen 9 9950X3D 16-Core Processor",
  "cores": "32",
  "memory_gb": 93,
  "gpu": "GameViewer Virtual Display Adapter, AMD Radeon(TM) Graphics, NVIDIA GeForce RTX 4090",
  "system": "Microsoft Windows 11 Pro for Workstations 10.0.26300",
  "fonts": 895,
  "docs": 100,
  "rounds": 3,
  "keys": 30,
  "builds": {
    "ely-doc": "8062efb130475a12",
    "ely-table": "e403e76310001803",
    "ely-show": "8a9ee3da9bb733d3",
    "ely-mark": "7226d73e2b9855f7",
    "ely-lok-worker": "7550f92b89bad857"
  },
  "lok": "C:/Users/zhang/Documents/Github/ElyOffice/engines/lo-src/instdir/program"
}
```

| App | Open all ms | Open one more ms | Memory MB open / edited | Free GB, lowest | Keys idle: P50 / P95 / max ms | Frames idle: over 8.3 ms / all, dirty to shown | Stalls idle | Keys editing: P50 / P95 / max ms | Frames editing | Stalls editing | Command edit P50 / P95 ms | Save P50 / max ms | Stalls whole run |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ElyDoc | 16956 | 187 | 691 / 787 | 59 | 30: 22 / 28 / 28 | 45/46, p95 ≤22.0, max 23.6 | 0 | 30: 23 / 27 / 29 | 111/115, p95 ≤20.4, max 26.4 | 0 | 156 / 293 | 1296 / 2388 | 0, max 0 |
| ElyTable | 18089 | 252 | 928 / 1009 | 59 | 30: 17 / 25 / 26 | 31/36, p95 ≤17.3, max 23.0 | 0 | 30: 19 / 27 / 27 | 79/86, p95 ≤21.4, max 23.3 | 0 | 187 / 312 | 5295 / 9551 | 0, max 0 |
| ElyShow | 38791 | 447 | 1469 / 1859 | 59 | 30: 25 / 32 / 33 | 46/47, p95 ≤22.9, max 23.5 | 0 | 30: 27 / 67 / 108 | 245/251, p95 ≤21.9, max 28.4 | 0 | 667 / 1435 | 11714 / 23097 | 0, max 0 |
| ElyMark | 1141 | 79 | 227 / 299 | 60 | 30: 10 / 19 / 19 | 157/213, p95 ≤20.2, max 23.1 | 0 | 30: 12 / 18 / 20 | 200/257, p95 ≤22.1, max 27.8 | 0 | 114 / 138 | 131 / 270 | 0, max 0 |

Problems:

- ElyShow: PERF-02: 1 of 30 keys over 100 ms editing, max 108 ms

The one slow key (edit 35, 108 ms to tile) waited 61 ms for a single background `text.replace` (tab 20, about 62 ms in the worker), the one request R15.36 lets a key wait for. Engine lo-src, patches 0001 to 0088; branch r15-31 from main 3e2fbef; run L173744, not voided. Linked documents and big workbooks are checked against the values their files saved: nobody pressed Update links (PERF-10, LOK-11).
