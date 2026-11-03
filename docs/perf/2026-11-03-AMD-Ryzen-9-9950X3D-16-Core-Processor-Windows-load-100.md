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
| ElyDoc | 19585 | 7198 | 675 / 779 | 40 | 30: 21 / 30 / 33 | 60/61, p95 ≤26.5, max 36.7 | 0 | 30: 24 / 184 / 266 | 209/213, p95 ≤25.9, max 30.2 | 0 | 375 / 653 | 2836 / 5478 | 5, max 13652 |
| ElyTable | 23225 | 748 | 939 / 1036 | 36 | 30: 21 / 46 / 77 | 45/45, p95 ≤31.0, max 31.4 | 0 | 30: 25 / 1545 / 2790 | 253/253, p95 ≤25.5, max 31.1 | 0 | 1145 / 1777 | 6619 / 11584 | 89, max 662 |
| ElyShow | 48415 | 1990 | 1527 / 1977 | 45 | 30: 27 / 36 / 184 | 56/57, p95 ≤23.3, max 25.3 | 0 | 30: 36 / 1502 / 3810 | 464/465, p95 ≤24.8, max 27.6 | 0 | 1131 / 2433 | 14215 / 27218 | 100, max 1550 |
| ElyMark | 1494 | 88 | 223 / 267 | 50 | 30: 15 / 21 / 22 | 155/192, p95 ≤20.7, max 24.1 | 0 | 30: 13 / 22 / 25 | 208/255, p95 ≤22.6, max 29.7 | 0 | 106 / 164 | 725 / 1160 | 1, max 719 |

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
- ElyDoc: PERF-02: 5 of 30 keys over 100 ms editing, max 266 ms
- ElyDoc: PERF-02: main thread busy over 100 ms 5 times, max 13652 ms
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
- ElyTable: PERF-02: 10 of 30 keys over 100 ms editing, max 2790 ms
- ElyTable: PERF-02: main thread busy over 100 ms 89 times, max 662 ms
- ElyShow: PERF-02: 1 of 30 keys over 100 ms idle, max 184 ms
- ElyShow: PERF-02: 12 of 30 keys over 100 ms editing, max 3810 ms
- ElyShow: PERF-02: main thread busy over 100 ms 100 times, max 1550 ms
- ElyMark: PERF-02: main thread busy over 100 ms 1 times, max 719 ms
