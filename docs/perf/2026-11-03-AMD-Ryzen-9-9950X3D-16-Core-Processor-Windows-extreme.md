# PERF-10 extreme cases 2026-11-03

Format: docs/perf/README.md. Main at c3db6bd (R15.33, R15.34) merged; harness after review round 2; engine lo-snap (frozen, without patch 0080). Run E044407 is the whole list; rapid-open-close was voided there three times because the zoom case's wheel left the pointer over the next window (the harness now moves it off each new window) and was rerun as E051710.

| Case | Result | Measured | Run |
| --- | --- | --- | --- |
| ElyDoc: opens a file whose link sources are missing | FAIL | opened in 3823 ms, 5 keys max 27 ms, saved; lost on save: ['OLE link kept']; warnings: none | E044407 |
| ElyTable: opens a file whose link sources are missing | pass | opened in 2964 ms, 5 keys max 22 ms, saved; lost on save: nothing; warnings: none | E044407 |
| ElyTable: an external reference to a closed source shows the source's value | FAIL | Links!A1 shows 1.01; the cache holds 1.01, the source 2.01 | E044407 |
| ElyTable: the reference follows its source edited and saved in the same app | FAIL | Links!A1 shows 1.01 after Rates!B1 became 9.99 and was saved | E044407 |
| ElyDoc: INCLUDETEXT shows its source's text on opening | FAIL | shows the cached text | E044407 |
| ElyDoc: INCLUDETEXT follows its source changed while open | FAIL | kept the old text | E044407 |
| ElyDoc: the linked picture stays a link, no stale copy after its source changed | pass | read in the saved file | E044407 |
| ElyDoc: the OLE link stays, its preview renewed after its source changed | FAIL | read in the saved file | E044407 |
| ElyDoc: three open, tabs switched while typing and while all are edited | FAIL | keys stay in the document after 0 of 9 tab switches; 18 keys, 0 over 100 ms; keys lost (saved, typed): none; edits saved {'a.docx': [3], 'b.docx': [3], 'c.docx': [3]}; command failures none | E044407 |
| ElyShow: three open, tabs switched while typing and while all are edited | FAIL | keys stay in the document after 0 of 9 tab switches; 18 keys, 0 over 100 ms; keys lost (saved, typed): none; edits saved {'a.pptx': [3], 'b.pptx': [3], 'c.pptx': [3]}; command failures none | E044407 |
| ElyDoc: saved 93 times while 40 keys went in | pass | keys P95 27 max 28 ms, 0 over 100; every key saved: True; save failures: none | E044407 |
| ElyTable: saved 89 times while 40 keys went in | FAIL | keys P95 78 max 145 ms, 1 over 100; every key saved: False; save failures: none | E044407 |
| ElyShow: saved 83 times while 40 keys went in | FAIL | keys P95 381 max 391 ms, 17 over 100; every key saved: True; save failures: none | E044407 |
| ElyMark: saved 145 times while 40 keys went in | pass | keys P95 17 max 17 ms, 0 over 100; every key saved: True; save failures: none | E044407 |
| ElyDoc: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E044407 |
| ElyTable: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E044407 |
| ElyShow: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E044407 |
| ElyMark: closing an autosaved document | pass | closed at once; the keys on disk True | E044407 |
| ElyDoc: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'front.docx': (13, [0]), 'other0.docx': (0, [1]), 'other1.docx': (0, [1])}: front keeps 13 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E044407 |
| ElyTable: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'front.xlsx': (10, None), 'other0.xlsx': (0, '1'), 'other1.xlsx': (0, '1')}: front keeps 10 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E044407 |
| ElyShow: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'front.pptx': (10, [0]), 'other1.pptx': (0, [1]), 'other0.pptx': (0, [1])}: front keeps 10 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E044407 |
| ElyDoc: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; zoom not as asked: none; screenshots in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\wf_38652c95-7df-4\.work\E044407\zoom\ely-doc | E044407 |
| ElyTable: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; zoom not as asked: none; screenshots in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\wf_38652c95-7df-4\.work\E044407\zoom\ely-table | E044407 |
| ElyShow: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; zoom not as asked: none; screenshots in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\wf_38652c95-7df-4\.work\E044407\zoom\ely-show | E044407 |
| ElyDoc: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(2710, 15234), (2676, 13300), (2410, 14069)]; memory MB [269, 342, 344, 347] | E051710 |
| ElyTable: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(2705, 12530), (2822, 14285), (2820, 13823)]; memory MB [293, 328, 325, 347] | E051710 |
| ElyShow: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(7223, 13121), (7829, 13321), (8422, 16688)]; memory MB [285, 361, 404, 431] | E051710 |
| ElyMark: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(1075, 15370), (370, 14433), (1209, 13031)]; memory MB [78, 83, 83, 84] | E051710 |
| ElyDoc: a 5 MB file | pass | open 2134 ms; keys P50 22 P95 28 max 28 ms; frames over 8.3 ms 39/45, dirty to shown max 21.9 ms; edit 34 ms; save 280 ms; 335 MB; problems none | E044407 |
| ElyTable: a 2 MB file | pass | open 2292 ms; keys P50 31 P95 36 max 36 ms; frames over 8.3 ms 43/54, dirty to shown max 20.1 ms; edit 23 ms; save 1166 ms; 357 MB; problems none | E044407 |
| ElyShow: a 94 MB file | FAIL | open 3880 ms; keys P50 39 P95 1014 max 1014 ms; frames over 8.3 ms 77/80, dirty to shown max 26.8 ms; edit 54 ms; save 1157 ms; 832 MB; problems none | E044407 |
| ElyMark: a 7 MB file | pass | open 1115 ms; keys P50 49 P95 92 max 92 ms; frames over 8.3 ms 160/160, dirty to shown max 69.5 ms; edit 31 ms; save 88 ms; 226 MB; problems none | E044407 |

Known and filed: ElyShow's slow keys while saving are R15.35. The ElyTable save-while-typing row is a data loss: a save ends the cell edit in progress, later keys replace the cell. The three-switching rows: after a tab switch, by UI Automation or a real mouse click, keys reach no document until the page is clicked. Zoom steps are read back from the view's log, each from the 25 % floor.
