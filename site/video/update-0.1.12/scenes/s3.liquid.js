/* s3 ElyPDF horizontal scrolling, global 26–32 s: the H2 ball swells into the
   reader; on the 30.0 hit the pages land in a row; the reader folds into the
   ElyMark ball (H3). */
(function () {
  "use strict";
  var M = LQ.M, C = LQ.C, F = window.FLOW;
  var W = { x: 160, y: 110, w: 1600, h: 700, r: 28 };

  LQ.add({
    id: "s3",
    start: 26,
    end: 32,
    build: function (lt, P) {
      F.open(P, lt, { r: 90, col: C.pdf, tint: "#F6D6DC", rect: W, t0: 0, t1: 0.8, t2: 1.35 });
      /* the row lands on the beat: a ripple under the pages */
      var pr = M.prog(lt, 4.0, 4.8);
      if (pr > 0 && pr < 0.98) P.ring(960, 480, 300 + 900 * M.outCubic(pr), (7 * Math.pow(1 - pr, 1.5) + 0.6) * M.smooth((lt - 4.0) / 0.05), C.white, { k: 12 });
      F.close(P, lt, { rect: W, from: "#F8E4E8", r: 90, col: C.mark, ta: 4.92, t0: 5.15, t1: 6.0 });
    },
  });

  window.__s3 = { W: W };
})();
