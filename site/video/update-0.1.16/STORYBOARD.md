# Ely Office 0.1.16 update film

90 s, 1920×1080, delivered at 24 fps. 120 BPM: a beat is 0.5 s, a bar is 2 s, 45 bars. Every cut, hit and handoff lands on the grid.

## Look

Frame by frame. Every drawing is redrawn: lines boil (each stroke re-jittered per drawing from a seeded noise, never the same twice in a row, never swimming). Drawings change on twos (12 a second); the flame and its embers move on ones (24). Ink and charcoal on warm paper, loose watercolour washes in each app's colour, smudges, overshooting line ends, pressure in the stroke.

The flame is the only light. The film opens on black; a struck match lights a pool of paper around the flame, and the pool moves with it: grain casts tiny shadows away from it, warmth falls off with distance. Everything is drawn inside that light. It widens as the film goes on and floods the page at the finale.

The flame is the one character. It hops, leans, squashes on landing, stretches when it leaps, curls into a question mark, shrinks to an ember when it thinks, roars when a thing it touches comes alive. It never gets a face; it acts with shape alone.

## Rules

- No captions, no labels, no feature names, no taglines, no boxes inside boxes. A window is a few wobbling lines, never a frame around a frame.
- The only words are what the apps themselves show (a file name, a paragraph, a formula, cell values, a paper size) and, at the end, "Ely Office" and "0.1.16".
- Only what `site/changelog/0.1.16.md` says shipped.
- Deterministic: frame n is a pure function of n. Rendering any frame alone gives the same pixels.

## Handoff

At each scene boundary the flame rests at (960, 610), height 140 px, upright, idle, in a light pool of radius 520. A scene may move it anywhere in between, but takes it from and returns it to that rest.

## Scenes (global seconds; bars from 0)

| Scene | Window | Bars | Beats on screen | Ends on |
| --- | --- | --- | --- | --- |
| s1 | 0–8 | 0–3 | Black. A hand-drawn match strikes on the down beat of bar 1 (2.0): sparks, flare, the head catches. The flame drops off the match, lands (squash) on the paper, the light pool opens. It finds "0.1.15" scratched in charcoal, burns the 5 away and draws a 6 with its own trail (hit 6.0). | rest |
| s2 | 8–20 | 4–9 | Files from the desktop: a .docx, a .xlsx, a photo. The flame kicks the .docx onto an ElyDoc window: it opens there. It flings the .xlsx at the same window: the sheet flies on to ElyTable's window and opens there. It drops the photo onto the open page at an exact spot; the picture lands where it fell, ink splash on contact (hit 16.0). | rest |
| s3 | 20–32 | 10–15 | A slide line "微软雅黑 Microsoft" with a visible gap between 雅黑 and Microsoft; the flame wedges in, pushes, the gap snaps shut (hit 24.0). Then "思源宋体" drawn as four tofu boxes with a missing-font mark; the flame breathes on them and each box catches and burns into its real glyph, one per beat (26–28); the mark burns away. | rest |
| s4 | 32–42 | 16–20 | Seven app icons, each with its own little engine flame inside, tumble into one cardboard box; their seven flames pour into one engine in the box (hit 36.0). Then a Windows window drawn empty and grey, waiting; the flame lands on it and the splash blooms at once while the window fills (hit 40.0). | rest |
| s5 | 42–54 | 21–26 | ElyMark page: `` `$$` `` in a code span, then two display formulas. The code-span dollars reach out like hooks for the formula's opening dollars; the flame burns the hook; both formulas render, the second written by the flame's trail: e^{iπ} + 1 = 0 (hit 48.0). An image's alt text "literal $$" tries the same and gets the same answer (50–52). | rest |
| s6 | 54–64 | 27–31 | ElyTable: a block of numbers; a new chart lands beside the data, clear of a drawing already on the sheet, sliding around it on the beat (hit 56.0). A formula cell linked to another workbook by an inked thread; the other book's sheet is torn away; the thread frays, but the saved value stays lit, guarded by the flame, and the source's name is circled in red (60–62). | rest |
| s7 | 64–74 | 32–36 | ElyDoc: a Chinese paragraph; the first line slides in by exactly two characters, two ghost 字 marking the space (hit 66.0). ElyTree: a commit list with its history graph of coloured lanes; the flame snuffs the graph and the titles stretch into its width (70.0); a drag selects three commit lines and they lift as a copied strip. | rest |
| s8 | 74–82 | 37–40 | ElyPDF: a blank page unfolds at A4, refolds at Letter, the paper size in the corner of the sheet (hit 76.0). A text block on a page is dragged by the flame to a new place; its marks stay behind where they were (80.0). | rest |
| s9 | 82–90 | 41–44 | Every drawing of the film flickers back for a frame each, then catches; the fire rises into "Ely Office" written in flame and "0.1.16" in embers (hit 86.0). The fire burns down to the match's charred stick, one last ember winks (88.5), black at 89.9. | black |

## Sound

One original score at 120 BPM, synthesised in code: plucked nylon, kalimba, upright bass pizzicato, brushes and a soft kick, warm and playful; quiet until the match catches, full from bar 4, a breakdown at bars 21–22, the tutti at the finale hit (86.0), a held chord to black. Foley on every drawn action: match strike, ignition whoosh, crackle under the flame scaled by its energy, paper rustle, pencil scratch on each drawn stroke burst, a tiny pop per glyph that catches, a thud on each landing. Hits land on the grid.
