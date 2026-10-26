---
workflow: general-video
flow: automation
message: "Ely Office 0.1.12: other programs drive ElyTable, plus horizontal PDFs, copy paths, your own data folder and fixes"
aspect: 1920x1080
fps: 60
language: zh-CN + en (bilingual captions)
length: 60s
angle: release update
---

## Intent

The 0.1.11 → 0.1.12 update film, made like the 0.1.10 film: Liquid Motion, aurora and glass, bilingual captions, music. ElyTable's command line (GitHub #19) is the headline; ElyPDF horizontal scrolling (#39), ElyMark's copy name and path (#41), the data folder (#43) and the fixes follow.

## Facts

Only what the 0.1.12 release notes say shipped. The terminal shows the real commands and the real `cell get` answer; the Settings row shows the app's own wording for a pending move.

## Kit

`lib/liquid.js`, `lib/ui.css`, `lib/captions.js` (as a track) and the audio tools come from `../update-0.1.10`. `lib/flow.js` holds the three liquid moves the feature scenes share. The music is the 0.1.10 score cut to 60 s (`tools/audio/cut.py`).
