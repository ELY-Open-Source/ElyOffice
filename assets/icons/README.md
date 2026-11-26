# Ely Office icons

One schema for all eight apps, after the Papirus icon theme: each app is one large flat object on Papirus's 192-unit grid (a page, a board, an envelope, a terminal). The body is one saturated colour; a folded corner is a lighter tint of it; content is white, with at most one accent colour. Each body and its content cast Papirus's soft shadow (the shape, 4 or 2 units down, blurred, 20 % black). No gradients, outlines or glass.

| App | Object | Colour | Accent |
| --- | --- | --- | --- |
| ElyDoc | page of lines | `#3D7FE5` | |
| ElyTable | page with a grid | `#2E9E5B` | selected cell `#FFD54F` |
| ElyShow | chart board on a stand | `#F07B2A` | pie `#5D7FC9` |
| ElyPDF | page with a picture | `#E0344F` | |
| ElyMail | envelope | `#1E8FE0` | |
| ElyTree | branch with commits | `#7E57C2` | merge commit `#F2A900` |
| ElyMark | notepad with M and arrow | `#ECECEC`, `#3E4A5C` | arrow `#F2A900` |
| ElyAgent | terminal prompt | `#4F4F4F` | spark `#FF6B8B` |

## Files

- `packaging/icons/masters.py` writes every SVG master. Change the design there, never in the SVGs.
- `<slug>.svg` is the shapeless master for Linux and Windows: the object alone on a transparent background.
- `<slug>-macos.svg` is the macOS master: the same object at 86 % on a light Apple squircle (an 824/1024 body drawn as a superellipse with n = 5).
- `<slug>-small.svg` and `<slug>-small-macos.svg` are the cut for sizes under 48 px: no shadows, no small details.
- `<slug>-document-macos.svg` (and its `-small` cut) is Finder's icon for the files an app opens: a white page with a folded corner, the app's object at 62 % on it. Every app but ElyTree, which opens folders, has one; `packaging/macos/documents.sh` names it as `CFBundleTypeIconFile`, and it shows only where that app opens the type. Windows shows the app's `.ico` (the ProgID's `DefaultIcon`); Linux keeps the icon theme's, which draws by MIME type whichever app opens the file.
- `<slug>.icns`, `<slug>-document.icns`, `<slug>.png` (512 px) and `<slug>.ico` (16 to 256 px) are rendered by `packaging/icons/build.sh`. Never edit them by hand. `tests/scripts/icons.sh` fails when a rendered file is older than its master.

## Rules

- Objects fill Papirus's frame: a page is 128 by 160 units, a landscape object 160 wide.
- Never put a container behind a shapeless icon, and never recolour one.
- A new app takes an object, a body colour unlike the others, and at most one accent.
