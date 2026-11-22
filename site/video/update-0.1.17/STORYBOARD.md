# Ely Office 0.1.17 update film: One Line

120 s, 1920×1080, 60 fps. 120 BPM: a beat is 0.5 s, a bar is 2 s, 60 bars. Every cut, hit and handoff lands on the grid.

## The idea

One shot. One line. The pen touches the paper once, at 1.0 s, and lifts once, at 118.5 s. Everything in between, every window, icon, glyph, cursor and chart, is that same unbroken line. Nothing is ever cut. When an object leaves, its stretch of line draws itself back into the line around it. When an object moves, the line moves with it.

The camera never cuts either. Each chapter is drawn on its own panel of one infinite sheet of paper, and the camera flies from panel to panel following the pen. At the finale the camera pulls back and the whole film is on the page at once: every chapter, side by side, joined by the one line that drew them. The changelog becomes a map.

## Look

Line art. Warm off-white paper (#F3F0E8) with a barely visible tooth; a deep ink line (#1C1F26). The line has pressure: it thins where the pen moves fast and swells where it slows, tapers in at 1.0 s and out at 118.5 s. No fills, no hatching, no shading, no gradients on objects. Depth comes only from line weight and overlap.

Colour is wet ink. The last moment of freshly drawn line carries the colour of the app being drawn, and it dries to ink in about a second. The pen tip is a small bead of that colour. Nothing else is coloured.

App colours (ink tint while wet): ElyDoc #3B6FE0, ElyTable #1E9E61, ElyShow #E8692E, ElyPDF #D5413A, ElyMail #7A5AF0, ElyTree #C98A1A, ElyMark #2A9BB0, the suite #1C1F26 drying to itself.

Motion is designed, not decorated: anticipation before a drag, overshoot on a snap, springs that settle, nothing linear unless it is a machine. Every interaction is something a user does: the pen is also the pointer, and the line is its tether.

## Rules

- No captions, no labels, no feature names, no taglines, no boxes inside boxes. Let the layout explain.
- The only words are what the apps themselves show (a menu item, a file name, a size, a URL, a status bar line) and, at the end, "Ely Office" and "0.1.17".
- Only what `site/changelog/0.1.17.md` says shipped.
- Deterministic: frame n is a pure function of n.
- Readable at 1080p: anything a viewer must read is at least 28 px tall on screen.

## Chapters (global seconds; each on its own panel)

| Ch | Window | Shows (changelog line) | Key hits |
| --- | --- | --- | --- |
| c01 | 0–8 | Blank paper. The pen lands (1.0) and writes "0.1.16". The 6's loop opens and the same line straightens into a 7 (6.0). | 1.0 touch, 6.0 morph |
| c02 | 8–20 | One installer: seven app icons drawn in a row; the line cinches them into one package (12.0). It opens three ways: a Windows setup with a checkbox per app, ticked one per beat; a macOS `.pkg`; a Linux package and an AppImage. An update swaps the whole set at once (18.0). | 12.0, 14–17.5 ticks, 18.0 |
| c03 | 20–30 | Resident engines: sign in; three small idle engines settle beside ElyDoc, ElyTable, ElyShow. A CPU trace beside them is the line lying perfectly flat. A double-clicked file opens at once (26.0). On a machine with little memory, only one engine stays (28.0). | 26.0, 28.0 |
| c04 | 30–38 | A launch animation plays and holds until the document is drawn (32.0). Reduce Motion on: the next launch is still (34.0). A tab closes and is gone at once; the engine finishes elsewhere (36.0). | 32.0, 34.0, 36.0 |
| c05 | 38–46 | Banners: a banner across a window is peeled away; its words fold into the status bar (40.0), one becomes a dialog (42.0), one an error page (43.0). ElyDoc's missing-font notice alone stays (44.5). | 40.0, 42.0, 43.0, 44.5 |
| c06 | 46–58 | ElyTable's format panel sets a cell's font and size (48.0). ElyPDF's scroll bar is dragged straight through the whole document (51.0). ElyMark: the document text size row in Settings, moved and reset (54.0); a folder found by search opens to its files (56.0). | 48.0, 51.0, 54.0, 56.0 |
| c07 | 58–68 | ElyShow smart guides, the showpiece: a shape is dragged across a slide; guides draw themselves when its edges and centre line up with other shapes and the slide, and it snaps (60.0, 61.0, 62.0); equal spacing snaps (64.0); with Alt held it moves freely and no guide appears (66.0). | 60, 61, 62, 64, 66 |
| c08 | 68–78 | ElyShow fixes: after Insert › shape, the next click selects (70.0); a Linux slideshow turns with the arrow keys and Space (72.0); Ctrl+Z undoes typing in a text box (74.0); links on a rotated shape and on a connector work in the show (75.5); a shape linked to a script starts the show without a crash (77.0). | 70, 72, 74, 75.5, 77 |
| c09 | 78–88 | Text and PDF: a Chinese line whose font is missing is drawn as brush strokes, then the same line settles into a plain serif and sans (80.0); a PDF saved twice keeps its size (82.0); a Chinese PDF export keeps only the glyphs it uses (84.0); Japanese text copies out whole (85.5); stray boxes cached in a Word EQ field drop out of the page (87.0). | 80, 82, 84, 85.5, 87 |
| c10 | 88–98 | Links and safety: a URL with spaces and quotes is encoded as it leaves (90.0); a broken address says so (91.5); a mail link asking to attach a file is refused (93.0); app and engine talk over a channel only this user opens and check versions (94.5); a pasted HTML picture is decoded and checked before the engine gets it (96.5). | 90, 91.5, 93, 94.5, 96.5 |
| c11 | 98–106 | A failed save says which document, why, and offers Save As… (100.0); ElyDoc opens without a working GPU (101.5); after AppImage → package, the engines at sign-in are the package's (103.0); ElyTree stops only Git and its helpers, Explorer stays (104.5); ElyMail's narrow sidebar keeps the unread counts (105.5). | 100, 101.5, 103, 104.5, 105.5 |
| c12 | 106–120 | The camera pulls back over every panel: the whole film on one sheet, joined by one line (106–112). The line writes "Ely Office" and "0.1.17" (114.0). The pen lifts (118.5). Hold. | 112, 114, 118.5 |

## Sound

One original score at 120 BPM, synthesised in code: felt piano, plucked and bowed strings, marimba, a warm sub, brushes and a soft kick; sparse at the touch, full from bar 4, a breakdown at bars 39–43, the tutti at 112, a held chord to the lift. The line itself is audible: a pen-on-paper scratch driven by the pen's speed and pressure, panned by its position on screen. Foley on every interaction: clicks, ticks, snaps, key presses, paper. Hits land on the grid.
