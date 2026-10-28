/* s5 data folder, 38–44 s: drops flow C: to D:. */
(function () {
  "use strict";
  var C = LQ.C, F = window.FLOW;
  /* Layout, shared with scenes/s5.html. */
  var G = {
    win: { x: 160, y: 130, w: 980, h: 600, r: 28 },
    from: { x: 1300, y: 225, w: 240, h: 150 },
    to: { x: 1520, y: 525, w: 240, h: 150 },
  };
  var DATA = [C.mail, C.doc, C.mark, C.table, C.pdf];
  var ALL = { x: 160, y: 130, w: 1600, h: 600, r: 28 };
  function mid(b) { return { x: b.x + b.w / 2, y: b.y + b.h / 2 }; }

  LQ.add({
    id: "s5",
    start: 38,
    end: 44,
    build: function (lt, P) {
      F.open(P, lt, { r: 90, col: C.cyan, tint: "#D8F6FA", rect: G.win, t0: 0, t1: 0.8, t2: 1.35 });
      P.layer(1);
      var a = mid(G.from), b = mid(G.to);
      for (var i = 0; i < DATA.length; i++) {
        var t0 = 2.9 + i * 0.28;
        F.drop(P, lt, { t0: t0, t1: t0 + 0.45, ax: a.x, ay: a.y, bx: b.x, by: b.y, r: 18, col: DATA[i], h: 120 });
      }
      P.layer(0);
      F.close(P, lt, { rect: ALL, from: "#E6F7FA", r: 90, col: C.violet, ta: 4.92, t0: 5.15, t1: 6.0 });
    },
  });

  window.__s5 = { G: G };
})();
