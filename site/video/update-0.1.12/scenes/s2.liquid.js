/* s2 ElyTable from the command line, global 12–26 s: the H1 ball swells into the
   sheet and flings a drop that becomes the terminal; drops carry the write to D4
   and the answer back; both panels fold into the ElyPDF ball (H2). */
(function () {
  "use strict";
  var M = LQ.M, C = LQ.C, F = window.FLOW;
  /* Layout, shared with scenes/s2.html. */
  var G = {
    term: { x: 120, y: 150, w: 760, h: 520, r: 28 },
    sheet: { x: 940, y: 120, w: 860, h: 640, r: 28 },
    gridX: 960, gridY: 244, rowH: 50, headH: 36, rowHead: 52,
    cols: [200, 130, 150, 170, 120],
    lineX: 148, lineY: 230, lineH: 34, charW: 12.6,
  };
  function cell(col, row) {
    var x = G.gridX + G.rowHead;
    for (var i = 0; i < col; i++) x += G.cols[i];
    return { x: x + G.cols[col] / 2, y: G.gridY + G.headH + (row - 1) * G.rowH + G.rowH / 2 };
  }
  function lineEnd(line, chars) { return { x: G.lineX + chars * G.charW + 10, y: G.lineY + line * G.lineH + G.lineH / 2 }; }
  var D4 = cell(3, 4);
  var SENT = lineEnd(1, 49), BACK = { x: G.lineX + 40, y: G.lineY + 3 * G.lineH + G.lineH / 2 };
  var ALL = { x: 120, y: 120, w: 1680, h: 640, r: 28 };

  LQ.add({
    id: "s2",
    start: 12,
    end: 26,
    build: function (lt, P) {
      F.open(P, lt, { r: 120, col: C.table, tint: "#CFF2DD", rect: G.sheet, t0: 0, t1: 0.8, t2: 1.35 });
      F.drop(P, lt, { t0: 0.35, t1: 0.75, ax: 1180, ay: 430, bx: 500, by: 410, r: 46, col: C.table, h: 160 });
      F.open(P, lt, { r: 40, col: C.table, tint: "#1B3B30", rect: G.term, ta: 0.75, t0: 0.78, t1: 1.3, t2: 1.8, cx: 500, cy: 410 });
      P.layer(1);
      F.drop(P, lt, { t0: 5.8, t1: 6.4, ax: SENT.x, ay: SENT.y, bx: D4.x, by: D4.y, r: 20, col: C.table, h: 170 });
      F.drop(P, lt, { t0: 7.7, t1: 8.2, ax: D4.x, ay: D4.y, bx: BACK.x, by: BACK.y, r: 16, col: C.cyan, h: 120 });
      P.layer(0);
      F.close(P, lt, { rect: ALL, from: "#F1F7F3", r: 90, col: C.pdf, ta: 12.92, t0: 13.15, t1: 14.0 });
    },
  });

  window.__s2 = { G: G, cell: cell };
})();
