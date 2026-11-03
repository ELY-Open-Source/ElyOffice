# PERF-10 load 2026-11-02, 25 documents per app

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
  "docs": 25,
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

| App | Open all ms | Open one more ms | Memory MB open / edited | Free GB, lowest | Keys idle: P50 / P95 / max ms | Frames idle: over 8.3 ms / all, dirty to shown | Stalls idle | Keys editing: P50 / P95 / max ms | Frames editing | Stalls editing | Command edit P50 / P95 ms | Save P50 / max ms | Stalls whole run |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ElyDoc | 7691 | 2573 | 537 / 718 | 43 | 30: 24 / 30 / 33 | 63/70, p95 ≤23.9, max 24.6 | 0 | 30: 22 / 30 / 9228 | 107/125, p95 ≤23.1, max 24.5 | 0 | 248 / 535 | 1652 / 2635 | 4, max 3990 |
| ElyTable | 8803 | 526 | 598 / 780 | 42 | 30: 19 / 28 / 28 | 35/39, p95 ≤21.2, max 21.6 | 0 | 30: 20 / 172 / 1342 | 108/120, p95 ≤20.3, max 21.1 | 0 | 249 / 1585 | 2045 / 3391 | 19, max 451 |
| ElyShow | 19536 | 870 | 771 / 1176 | 36 | 30: 28 / 38 / 40 | 57/59, p95 ≤22.2, max 24.0 | 0 | 30: 31 / 238 / 8772 | 190/197, p95 ≤25.1, max 25.1 | 0 | 292 / 1913 | 4966 / 8958 | 22, max 778 |
| ElyMark | 837 | 103 | 125 / 140 | 35 | 30: 14 / 20 / 21 | 153/216, p95 ≤20.8, max 30.3 | 0 | 30: 14 / 21 / 23 | 219/285, p95 ≤24.3, max 64.3 | 0 | 481 / 673 | 1448 / 2507 | 1, max 225 |

Problems:

- ElyDoc: doc-001-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-001-linked.docx: OLE link kept failed
- ElyDoc: doc-011-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-011-linked.docx: OLE link kept failed
- ElyDoc: doc-021-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-021-linked.docx: OLE link kept failed
- ElyDoc: PERF-02: 1 of 30 keys over 100 ms editing, max 9228 ms
- ElyDoc: PERF-02: main thread busy over 100 ms 4 times, max 3990 ms
- ElyTable: table-000-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-010-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-020-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: PERF-02: 2 of 30 keys over 100 ms editing, max 1342 ms
- ElyTable: PERF-02: main thread busy over 100 ms 19 times, max 451 ms
- ElyShow: PERF-02: 2 of 30 keys over 100 ms editing, max 8772 ms
- ElyShow: PERF-02: main thread busy over 100 ms 22 times, max 778 ms
- ElyMark: PERF-02: main thread busy over 100 ms 1 times, max 225 ms
