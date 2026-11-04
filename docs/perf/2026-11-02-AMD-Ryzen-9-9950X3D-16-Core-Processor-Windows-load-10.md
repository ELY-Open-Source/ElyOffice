# PERF-10 load 2026-11-02, 10 documents per app

Format: docs/perf/README.md.

```json
{
  "date": "2026-11-02",
  "machine": "AMD-Ryzen-9-9950X3D-16-Core-Processor-Windows",
  "cpu": "AMD Ryzen 9 9950X3D 16-Core Processor",
  "cores": "32",
  "memory_gb": 93,
  "gpu": "GameViewer Virtual Display Adapter, AMD Radeon(TM) Graphics, NVIDIA GeForce RTX 4090",
  "system": "Microsoft Windows 11 Pro for Workstations 10.0.26300",
  "fonts": 895,
  "docs": 10,
  "rounds": 3,
  "keys": 30,
  "builds": {
    "ely-doc": "410271249d3dd1ca",
    "ely-table": "66953b1a34eb38a0",
    "ely-show": "05b496c20656cfe1",
    "ely-mark": "2ad1efd5de1deceb",
    "ely-lok-worker": "14d7a52a96dee2d1"
  },
  "lok": "C:/Users/zhang/Documents/Github/ElyOffice/engines/lo-snap/instdir/program"
}
```

| App | Open all ms | Open one more ms | Memory MB open / edited | Keys idle: P50 / P95 / max ms | Frames idle: over 8.3 ms / all, dirty to shown | Stalls idle | Keys editing: P50 / P95 / max ms | Frames editing | Stalls editing | Command edit P50 / P95 ms | Save P50 / max ms | Stalls whole run |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ElyDoc | 6577 | 1327 | 395 / 471 | 30: 22 / 31 / 32 | 61/80, p95 ≤23.4, max 24.1 | 0 | 30: 20 / 30 / 9388 | 104/113, p95 ≤23.2, max 26.8 | 0 | 213 / 425 | 513 / 776 | 4, max 1539 |
| ElyTable | 5591 | 319 | 406 / 471 | 30: 21 / 28 / 28 | 51/117, p95 ≤21.0, max 26.2 | 0 | 30: 25 / 36 / 1242 | 72/96, p95 ≤21.7, max 24.8 | 0 | 82 / 278 | 1038 / 1381 | 5, max 163 |
| ElyShow | 10510 | 795 | 561 / 657 | 30: 32 / 381 / 387 | 64/68, p95 ≤28.0, max 28.0 | 0 | 30: 31 / 487 / 2913 | 139/147, p95 ≤21.7, max 24.3 | 0 | 143 / 198 | 2331 / 3561 | 6, max 897 |
| ElyMark | 1037 | 110 | 99 / 109 | 30: 12 / 17 / 18 | 145/226, p95 ≤21.1, max 22.6 | 0 | 30: 13 / 18 / 19 | 154/232, p95 ≤21.3, max 24.1 | 0 | 54 / 308 | 609 / 620 | 0, max 0 |

Problems:

- ElyDoc: doc-001-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-001-linked.docx: OLE link kept failed
- ElyDoc: PERF-02: 1 of 30 keys over 100 ms editing, max 9388 ms
- ElyDoc: PERF-02: main thread busy over 100 ms 4 times, max 1539 ms
- ElyTable: table-000-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: PERF-02: 1 of 30 keys over 100 ms editing, max 1242 ms
- ElyTable: PERF-02: main thread busy over 100 ms 5 times, max 163 ms
- ElyShow: PERF-02: 3 of 30 keys over 100 ms idle, max 387 ms
- ElyShow: PERF-02: 3 of 30 keys over 100 ms editing, max 2913 ms
- ElyShow: PERF-02: main thread busy over 100 ms 6 times, max 897 ms
