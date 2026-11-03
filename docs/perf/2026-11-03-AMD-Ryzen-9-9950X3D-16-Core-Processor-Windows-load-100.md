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
| ElyDoc | 21812 | 7850 | 919 / 1920 | 37 | 30: 27 / 42 / 57 | 90/91, p95 ≤30.6, max 33.7 | 0 | 30: 22 / 41 / 9433 | 245/249, p95 ≤25.1, max 29.6 | 0 | 522 / 946 | 2779 / 5644 | 6, max 14677 |
| ElyTable | 23275 | 1366 | 1461 / 2076 | 51 | 30: 19 / 27 / 28 | 35/37, p95 ≤19.7, max 22.8 | 0 | 30: 21 / 1862 / 6043 | 269/276, p95 ≤26.5, max 26.8 | 0 | 753 / 9505 | 5811 / 10262 | 87, max 637 |
| ElyMark | 1476 | 286 | 220 / 268 | 47 | 30: 13 / 19 / 21 | 146/190, p95 ≤21.5, max 23.8 | 0 | 30: 13 / 20 / 38 | 194/224, p95 ≤22.1, max 24.0 | 0 | 515 / 874 | 223 / 518 | 1, max 678 |

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
- ElyDoc: PERF-02: 1 of 30 keys over 100 ms editing, max 9433 ms
- ElyDoc: PERF-02: main thread busy over 100 ms 6 times, max 14677 ms
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
- ElyTable: PERF-02: 2 of 30 keys over 100 ms editing, max 6043 ms
- ElyTable: PERF-02: main thread busy over 100 ms 87 times, max 637 ms
- ElyShow: run stopped: ely-show: key 1 showed nothing in 10 s
- ElyMark: PERF-02: main thread busy over 100 ms 1 times, max 678 ms
