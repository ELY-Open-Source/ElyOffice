/* s4 ElyMark copies a file's name and path, global 32–38 s: the H3 ball swells
   into the window; drops carry each copy from the menu to the toast; the window
   folds into the data-folder ball (H4). */
(function () {
  "use strict";
  var C = LQ.C, F = window.FLOW;
  /* Layout, shared with scenes/s4.html. */
  var G = {
    win: { x: 240, y: 110, w: 1440, h: 680, r: 28 },
    tab: { x: 789, y: 194 },
    file: { x: 390, y: 312 },
    menuW: 230, itemH: 44,
    toast: { x: 1430, y: 138 },
  };
  function item(at, n) { return { x: at.x + 8 + G.menuW / 2, y: at.y + 8 + 8 + n * G.itemH + G.itemH / 2 }; }
  var PATH = item(G.tab, 1), NAME = item(G.file, 0);

  LQ.add({
    id: "s4",
    start: 32,
    end: 38,
    build: function (lt, P) {
      F.open(P, lt, { r: 90, col: C.mark, tint: "#FFF0C9", rect: G.win, t0: 0, t1: 0.8, t2: 1.35 });
      P.layer(1);
      F.drop(P, lt, { t0: 2.45, t1: 2.9, ax: PATH.x, ay: PATH.y, bx: G.toast.x - 150, by: G.toast.y, r: 16, col: C.mark, h: 150 });
      F.drop(P, lt, { t0: 4.25, t1: 4.7, ax: NAME.x, ay: NAME.y, bx: G.toast.x - 150, by: G.toast.y, r: 16, col: C.mark, h: 200 });
      P.layer(0);
      F.close(P, lt, { rect: G.win, from: "#FFF6E0", r: 90, col: C.cyan, ta: 4.92, t0: 5.15, t1: 6.0 });
    },
  });

  window.__s4 = { G: G, item: item };
})();
