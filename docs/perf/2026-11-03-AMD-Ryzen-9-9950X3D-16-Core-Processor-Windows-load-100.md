# PERF-10 load 2026-11-03, 100 documents per app

Format: docs/perf/README.md.

```json
{
  "date": "2026-11-03",
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
    "ely-doc": "54cbf8b674c3ae12",
    "ely-table": "6feb214fe62ff029",
    "ely-show": "dae517dcc5ba782b",
    "ely-mark": "0e3b3c978a56b1c8",
    "ely-lok-worker": "9a18160fbf621292"
  },
  "lok": "C:/Users/zhang/Documents/Github/ElyOffice/engines/lo-snap/instdir/program"
}
```

| App | Open all ms | Open one more ms | Memory MB open / edited | Free GB, lowest | Keys idle: P50 / P95 / max ms | Frames idle: over 8.3 ms / all, dirty to shown | Stalls idle | Keys editing: P50 / P95 / max ms | Frames editing | Stalls editing | Command edit P50 / P95 ms | Save P50 / max ms | Stalls whole run |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ElyDoc | 16835 | 7022 | 676 / 778 | 40 | 30: 23 / 29 / 29 | 64/65, p95 ≤25.2, max 25.2 | 0 | 30: 26 / 308 / 311 | 215/220, p95 ≤27.3, max 28.6 | 0 | 842 / 4990 | 3013 / 5515 | 5, max 10348 |
| ElyTable | 21973 | 644 | 934 / 1029 | 45 | 30: 19 / 26 / 27 | 36/36, p95 ≤24.7, max 25.9 | 0 | 30: 24 / 395 / 423 | 225/232, p95 ≤24.6, max 28.5 | 0 | 592 / 1067 | 5281 / 10059 | 81, max 464 |
| ElyShow | 46864 | 2025 | 1580 / 2041 | 45 | 30: 26 / 39 / 426 | 57/57, p95 ≤21.8, max 25.5 | 0 | 30: 36 / 1302 / 1402 | 406/408, p95 ≤25.2, max 27.8 | 0 | 1007 / 1843 | 13397 / 26509 | 103, max 6292 |
| ElyMark | 2062 | 84 | 223 / 266 | 39 | 30: 15 / 23 / 23 | 173/208, p95 ≤22.7, max 545.2 | 1 | 30: 16 / 21 / 22 | 259/293, p95 ≤25.5, max 736.5 | 6 | 341 / 1486 | 1630 / 1886 | 11, max 1029 |

Problems:

- ElyDoc: doc-001-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-001-linked.docx: OLE link kept failed
- ElyDoc: doc-011-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-011-linked.docx: OLE link kept failed
- ElyDoc: doc-021-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-021-linked.docx: OLE link kept failed
- ElyDoc: doc-031-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-031-linked.docx: OLE link kept failed
- ElyDoc: doc-041-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-041-linked.docx: OLE link kept failed
- ElyDoc: doc-051-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-051-linked.docx: OLE link kept failed
- ElyDoc: doc-061-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-061-linked.docx: OLE link kept failed
- ElyDoc: doc-071-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-071-linked.docx: OLE link kept failed
- ElyDoc: doc-081-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-081-linked.docx: OLE link kept failed
- ElyDoc: doc-091-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-091-linked.docx: OLE link kept failed
- ElyDoc: PERF-02: 3 of 30 keys over 100 ms editing, max 311 ms
- ElyDoc: PERF-02: main thread busy over 100 ms 5 times, max 10348 ms
- ElyTable: table-000-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-010-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-020-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-030-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-040-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-050-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-060-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-070-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-080-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-090-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: PERF-02: 6 of 30 keys over 100 ms editing, max 423 ms
- ElyTable: PERF-02: main thread busy over 100 ms 81 times, max 464 ms
- ElyShow: PERF-02: 1 of 30 keys over 100 ms idle, max 426 ms
- ElyShow: PERF-02: 12 of 30 keys over 100 ms editing, max 1402 ms
- ElyShow: PERF-02: main thread busy over 100 ms 103 times, max 6292 ms
- ElyMark: PERF-02: a frame took 545 ms from dirty to shown idle
- ElyMark: PERF-02: a frame took 736 ms from dirty to shown editing
- ElyMark: PERF-02: main thread busy over 100 ms 11 times, max 1029 ms
