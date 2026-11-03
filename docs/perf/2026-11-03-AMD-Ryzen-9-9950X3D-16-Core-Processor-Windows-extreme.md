# PERF-10 extreme cases 2026-11-03

Format: docs/perf/README.md. Main at c3db6bd (R15.33, R15.34) merged; harness after review round 3; engine lo-snap (frozen, without patch 0080). One run, E055808, no case voided. Other jobs were building on the machine: opening times are about twice the round-2 run's.

| Case | Result | Measured | Run |
| --- | --- | --- | --- |
| ElyDoc: opens a file whose link sources are missing | FAIL | opened in 9312 ms, 5 keys max 25 ms, saved; lost on save: ['OLE link kept']; warnings: none | E055808 |
| ElyTable: opens a file whose link sources are missing | pass | opened in 6073 ms, 5 keys max 29 ms, saved; lost on save: nothing; warnings: none | E055808 |
| ElyTable: an external reference to a closed source shows the source's value | FAIL | Links!A1 shows 1.01; the cache holds 1.01, the source 2.01 | E055808 |
| ElyTable: the reference follows its source edited and saved in the same app | FAIL | Links!A1 shows 1.01 after Rates!B1 became 9.99 and was saved | E055808 |
| ElyDoc: INCLUDETEXT shows its source's text on opening | FAIL | shows the cached text | E055808 |
| ElyDoc: INCLUDETEXT follows its source changed while open | FAIL | kept the old text | E055808 |
| ElyDoc: INCLUDETEXT saved with the changed text | FAIL | read in the saved file after the sources changed | E055808 |
| ElyDoc: the linked picture still links its file | pass | read in the saved file after the sources changed | E055808 |
| ElyDoc: the OLE link stays | FAIL | read in the saved file after the sources changed; the link is gone | E055808 |
| ElyDoc: three open, tabs switched while typing and while all are edited | FAIL | keys stay in the document after 0 of 9 tab switches; 18 keys, 0 over 100 ms; keys lost (saved, typed): none; edits saved {'a.docx': [3], 'b.docx': [3], 'c.docx': [3]}; command failures none | E055808 |
| ElyShow: three open, tabs switched while typing and while all are edited | FAIL | keys stay in the document after 0 of 9 tab switches; 18 keys, 0 over 100 ms; keys lost (saved, typed): none; edits saved {'a.pptx': [3], 'b.pptx': [3], 'c.pptx': [3]}; command failures none | E055808 |
| ElyDoc: saved 100 times while 40 keys went in | FAIL | keys P95 641 max 867 ms, 2 over 100; every key saved: True; save failures: none | E055808 |
| ElyTable: saved 87 times while 40 keys went in | FAIL | keys P95 103 max 231 ms, 2 over 100; every key saved: False; save failures: none | E055808 |
| ElyShow: saved 76 times while 40 keys went in | FAIL | keys P95 335 max 444 ms, 12 over 100; every key saved: True; save failures: none | E055808 |
| ElyMark: saved 167 times while 40 keys went in | pass | keys P95 16 max 16 ms, 0 over 100; every key saved: True; save failures: none | E055808 |
| ElyDoc: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E055808 |
| ElyTable: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E055808 |
| ElyShow: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E055808 |
| ElyMark: closing an autosaved document | pass | closed at once; the keys on disk True | E055808 |
| ElyDoc: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'other1.docx': (0, [1]), 'other0.docx': (0, [1]), 'front.docx': (11, [0])}: front keeps 11 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E055808 |
| ElyTable: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'other0.xlsx': (0, '1'), 'other1.xlsx': (0, '1'), 'front.xlsx': (10, None)}: front keeps 10 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E055808 |
| ElyShow: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'other0.pptx': (0, [1]), 'other1.pptx': (0, [1]), 'front.pptx': (14, [0])}: front keeps 14 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E055808 |
| ElyDoc: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; zoom not as asked: none; screenshots in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\wf_38652c95-7df-4\.work\E055808\zoom\ely-doc | E055808 |
| ElyTable: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; zoom not as asked: none; screenshots in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\wf_38652c95-7df-4\.work\E055808\zoom\ely-table | E055808 |
| ElyShow: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; zoom not as asked: none; screenshots in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\wf_38652c95-7df-4\.work\E055808\zoom\ely-show | E055808 |
| ElyDoc: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(3163, 17540), (3333, 34727), (6027, 17901)]; memory MB [268, 338, 343, 346] | E055808 |
| ElyTable: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(3833, 17062), (3348, 17125), (3695, 16381)]; memory MB [294, 325, 337, 340] | E055808 |
| ElyShow: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(9159, 27845), (8469, 16702), (14818, 17449)]; memory MB [273, 360, 407, 424] | E055808 |
| ElyMark: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(1612, 25928), (434, 17791), (270, 21378)]; memory MB [78, 83, 84, 84] | E055808 |
| ElyDoc: a 5 MB file | pass | open 2553 ms; keys P50 27 P95 37 max 37 ms; frames over 8.3 ms 57/61, dirty to shown max 21.8 ms; edit 210 ms; save 299 ms; 338 MB; problems none | E055808 |
| ElyTable: a 2 MB file | pass | open 3912 ms; keys P50 31 P95 48 max 48 ms; frames over 8.3 ms 55/63, dirty to shown max 23.6 ms; edit 438 ms; save 1977 ms; 366 MB; problems none | E055808 |
| ElyShow: a 94 MB file | FAIL | open 7975 ms; keys P50 36 P95 964 max 964 ms; frames over 8.3 ms 72/85, dirty to shown max 25.6 ms; edit 46 ms; save 1683 ms; 856 MB; problems none | E055808 |
| ElyMark: a 7 MB file | pass | open 2146 ms; keys P50 49 P95 84 max 84 ms; frames over 8.3 ms 196/196, dirty to shown max 68.3 ms; edit 114 ms; save 106 ms; 228 MB; problems none | E055808 |

Known and filed: slow keys while saving are R15.35 (ElyShow, and this run ElyDoc's 2 keys). The ElyTable save-while-typing row is a data loss: a save ends the cell edit in progress, later keys replace the cell. The three-switching rows: after a tab switch, by UI Automation or a real mouse click, keys reach no document until the page is clicked. Zoom steps are read back from the view's log, each from the 25 % floor. After review round 5 the OLE row checks only the kept link (re-read from the same saved file, still FAIL): what a renewed preview or linked picture shows needs a screen, and is not verified.
