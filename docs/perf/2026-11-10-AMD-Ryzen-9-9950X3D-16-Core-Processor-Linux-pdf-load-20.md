# PERF-10 ElyPDF load 2026-11-10, 20 PDFs of 1000 pages

AMD Ryzen 9 9950X3D 16-Core Processor, Omarchy, kernel 7.2.15-1-omarchy. Each PDF was paged, given a blank page, saved and read back.

| Open all ms | Memory MB | Free GB, lowest | Keys in front: P50 / P95 / max ms | First key in each other tab | Stalls |
| --- | --- | --- | --- | --- | --- |
| 3661 | 553 | 57 | 30: 27 / 33 / 46 | 19: 26 / 37 / 37 | 0, max 0 |

Problems:

- none
