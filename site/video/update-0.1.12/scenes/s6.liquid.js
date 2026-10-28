/* s6 fixes, 44–48 s: four cards melt into seven balls. */
(function () {
  "use strict";
  var M = LQ.M, C = LQ.C, F = window.FLOW;
  var GRID = { x: 344, y: 230, w: 1232, h: 432, r: 28 };
  var CARDS = [
    { x: 344, y: 230 }, { x: 976, y: 230 },
    { x: 344, y: 462 }, { x: 976, y: 462 },
  ];
  var CW = 600, CH = 200;
  var ORDER = ["doc", "table", "show", "mail", "pdf", "tree", "mark"];
  var WHITE = [0.95, 0.96, 1.0];

  /* 3.45–4.0: one drop divides into seven, exact at the end. */
  function split(lt, P) {
    var u = M.prog(lt, 3.45, 4.0), e = M.outBack(u, 1.4);
    var calm = 1 - M.smooth(M.prog(u, 0.6, 1));
    for (var i = 0; i < 7; i++) {
      var x = M.lerp(960, 960 + (i - 3) * 200, e), r = M.lerp(64, 62, M.smooth(u));
      var w = M.wobble(lt, 3.7 + Math.abs(i - 3) * 0.03, 3.2, 6, 0.12) * calm;
      P.ball(x, 500, r, M.mix3(WHITE, F.rgb(C[ORDER[i]]), M.smooth(M.prog(u, 0.1, 0.8))), { sx: 1 + w, sy: 1 - w, k: 26 + 30 * (1 - u) });
      if (i < 6) {
        var nx = M.lerp(960, 960 + (i - 2) * 200, e), neck = 26 * Math.pow(1 - u, 1.5);
        if (neck > 3) P.cap(x, 500, nx, 500, neck, neck, F.rgb(C[ORDER[i]]), { k: 26 });
      }
    }
  }

  LQ.add({
    id: "s6",
    start: 44,
    end: 48,
    build: function (lt, P) {
      if (lt >= 4.0) {
        for (var i = 0; i < 7; i++) P.ball(960 + (i - 3) * 200, 500, 62, C[ORDER[i]]);
        return;
      }
      F.open(P, lt, { r: 90, col: C.violet, tint: "#E7E0FF", rect: GRID, t0: 0, t1: 0.5, t2: 0.75 });
      /* the checks land as candy drops on each card */
      P.layer(1);
      for (var k = 0; k < 4; k++) {
        var t0 = 0.9 + k * 0.25, c = CARDS[k];
        var g = M.outBack(M.prog(lt, t0, t0 + 0.3), 2.6) * (1 - M.smooth(M.prog(lt, 2.85, 3.05)));
        var sq = 1 - 0.3 * F.imp(lt, t0 + 0.12, 3.4, 8);
        if (g > 0.01) P.ball(c.x + 72, c.y + CH / 2, 34 * g, "#3DDC84", { sx: 1 / sq, sy: sq, k: 14 });
      }
      P.layer(0);
      if (lt >= 2.9 && lt < 3.45) F.close(P, lt, { rect: GRID, from: "#ECE8FF", r: 64, col: "#F2F4FF", ta: 2.9, t0: 3.05, t1: 3.45 });
      if (lt >= 3.45) split(lt, P);
    },
  });

  window.__s6 = { CARDS: CARDS, CW: CW, CH: CH };
})();
