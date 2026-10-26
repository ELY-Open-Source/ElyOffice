/* Shared liquid moves for the feature scenes: a ball swells into a panel,
   a panel folds into the next ball, a drop flies on a string. Pure functions of time. */
(function () {
  "use strict";
  var M = LQ.M;

  function rgb(c) { return typeof c === "string" ? M.hex(c).slice(0, 3) : c; }
  function imp(t, t0, freq, damp) {
    var x = t - t0;
    if (x <= 0) return 0;
    return M.smooth(x / 0.05) * Math.exp(-(damp || 7) * x) * Math.cos(2 * Math.PI * (freq || 2.6) * x);
  }
  function box(P, cx, cy, w, h, r, col, k) {
    P.box(cx, cy, w, h, Math.min(r, w / 2, h / 2), col, { k: k || 40 });
  }

  /* o = {r, col, tint, rect: {x, y, w, h, r}, t0, t1, t2, cx, cy, ta}: the ball at (cx, cy),
     (960, 500) unless given, shown from ta, squashes before t0, swells into rect by t1, sinks by t2. */
  function open(P, lt, o) {
    if (lt >= o.t2 || (o.ta != null && lt < o.ta)) return;
    var ox = o.cx == null ? 960 : o.cx, oy = o.cy == null ? 500 : o.cy;
    if (lt < o.t0) {
      var sq = 1 - 0.16 * Math.sin(Math.PI * M.prog(lt, o.t0 - 0.35, o.t0));
      P.ball(ox, oy + o.r * (1 - sq), o.r, o.col, { sx: 1 / sq, sy: sq });
      return;
    }
    var R = o.rect, e = M.outBack(M.prog(lt, o.t0, o.t1), 1.3);
    var s = 1 - M.smooth(M.prog(lt, o.t1, o.t2));
    var jig = M.wobble(lt, o.t1, 2.6, 5, 0.03);
    var col = M.mix3(rgb(o.col), rgb(o.tint || o.col), M.smooth(M.prog(lt, o.t0, o.t1)));
    box(P, M.lerp(ox, R.x + R.w / 2, e), M.lerp(oy, R.y + R.h / 2, e),
      M.lerp(2 * o.r, R.w, e) * s * (1 + jig), M.lerp(2 * o.r, R.h, e) * s * (1 - jig), M.lerp(o.r, R.r, e) * s, col);
  }

  /* o = {rect, from, r, col, ta, t0, t1}: liquid fills rect behind the glass from ta,
     folds into the ball (960, 500, r, col) over [t0, t1], and is exactly that ball at t1. */
  function close(P, lt, o) {
    var ta = o.ta == null ? o.t0 - 0.3 : o.ta;
    if (lt < ta) return;
    if (lt >= o.t1) { P.ball(960, 500, o.r, o.col); return; }
    var R = o.rect;
    if (lt < o.t0) {
      var s = M.outCubic(M.prog(lt, ta, o.t0));
      box(P, R.x + R.w / 2, R.y + R.h / 2, R.w * s, R.h * s, R.r * s, o.from);
      return;
    }
    var u = M.prog(lt, o.t0, o.t1), e = M.outBack(u, 1.2), d = 2 * o.r;
    var jig = M.wobble(lt, o.t0 + (o.t1 - o.t0) * 0.55, 3, 5, 0.08) * (1 - M.smooth(M.prog(u, 0.7, 1)));
    box(P, M.lerp(R.x + R.w / 2, 960, e), M.lerp(R.y + R.h / 2, 500, e),
      M.lerp(R.w, d, e) * (1 + jig), M.lerp(R.h, d, e) * (1 - jig), M.lerp(R.r, o.r, e), M.mix3(rgb(o.from), rgb(o.col), M.smooth(u)));
  }

  /* o = {t0, t1, ax, ay, bx, by, r, col, h}: a drop beads at A, flies an arc of height h
     pulling a string that snaps, lands on B at t1 with a squash, and is absorbed. */
  function drop(P, lt, o) {
    var t0 = o.t0, t1 = o.t1, r = o.r;
    if (lt < t0 - 0.2 || lt > t1 + 0.4) return;
    if (lt < t0) {
      var g = M.outBack(M.prog(lt, t0 - 0.2, t0), 2);
      P.ball(o.ax, o.ay, r * g, o.col, { k: 18 });
      return;
    }
    if (lt < t1) {
      var u = (lt - t0) / (t1 - t0), d = t1 - t0, h = o.h == null ? 140 : o.h;
      var x = M.lerp(o.ax, o.bx, u), y = M.lerp(o.ay, o.by, u) - h * 4 * u * (1 - u);
      var vx = (o.bx - o.ax) / d, vy = (o.by - o.ay) / d - (h * 4 * (1 - 2 * u)) / d;
      var st = 1 + Math.min(Math.sqrt(vx * vx + vy * vy) / 2600, 0.6);
      P.ball(x, y, r, o.col, { sx: st, sy: 1 / st, rot: Math.atan2(vy, vx), k: 18 });
      var neck = r * 0.5 * (1 - M.smooth(M.prog(u, 0.05, 0.4)));
      if (neck > 2.5) P.cap(o.ax, o.ay, x, y, neck * 0.7, neck, o.col, { k: 18 });
      return;
    }
    var sq = 1 - 0.45 * imp(lt, t1, 3.2, 8);
    var rr = r * (1 - M.smooth(M.prog(lt, t1 + 0.12, t1 + 0.38)));
    if (rr > 0.5) P.ball(o.bx, o.by, rr, o.col, { sx: 1 / sq, sy: sq, k: 18 });
  }

  window.FLOW = { open: open, close: close, drop: drop, imp: imp, rgb: rgb };
})();
