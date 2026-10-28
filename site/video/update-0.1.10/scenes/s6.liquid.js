/* s6 search, 44–50 s: jelly scope, melting rows, the stat. */
(function () {
  var M = LQ.M;
  /* The HTML layer's search camera: scale 1.3, translate (-250, -101). */
  var S = 1.3, TX = -250, TY = -101;
  function sx(x) { return S * x + TX; }
  function sy(y) { return S * y + TY; }
  var LIST_X = 468, LIST_Y = 168;
  var JY = sy(LIST_Y + 56);
  var CHIP = [[16, 52], [76, 68], [152, 68], [228, 52], [288, 52], [348, 68]].map(function (c) {
    return { x: sx(LIST_X + c[0] + c[1] / 2), w: c[1] * S };
  });
  var AX = sx(LIST_X + 36), AR = 20 * S;
  function AY(slot) { return sy(LIST_Y + 104 + 88 * slot + 33); }
  var HOPS = [[0.72, 0], [1.5, 3], [1.9, 4], [2.3, 5]];
  /* [start, slot, colour] */
  var MELTS = [
    [0.92, 1, "#4b5468"], [0.95, 2, "#e07a2a"], [0.98, 3, "#2e9e5b"], [1.01, 4, "#6a4fd8"], [1.04, 5, "#1e8fe0"], [1.07, 6, "#e0344f"],
    [1.52, 2, "#f07b2a"], [1.56, 3, "#7e57c2"],
    [2.32, 1, "#3d7fe5"], [2.36, 2, "#f07b2a"],
  ];
  var RISES = [[1.22, 1, "#3d7fe5"], [1.28, 2, "#f07b2a"], [1.34, 3, "#7e57c2"], [1.93, 2, "#f07b2a"], [2.5, 1, "#7e57c2"]];
  var JELLY = "#45b4ff";

  /* the stat */
  var NX = 960, NY = 352, SXc = 1050, SYc = 650;
  var COUNT = ["0", "8", "64", "517", "3,906", "12,480", "38,215", "61,907", "88,342", "97,560", "100,000"];
  function specN(t) { return { text: t, font: '900 250px "Inter"', w: 1500, h: 360, blur: 12, align: "center" }; }
  function specS(t) { return { text: t, font: '900 184px "Inter"', w: 1100, h: 280, blur: 10, align: "center" }; }
  var COMMA_X = 963, NUM_BASE = 446, S_TOP = 584;

  function melt(lt, P, m) {
    var x = lt - m[0];
    if (x < 0 || x > 0.75) return;
    var ay = AY(m[1]), c = m[2];
    if (x < 0.12) {
      var s = M.smooth(x / 0.12);
      P.ball(AX, ay + 6 * s, AR, c, { sx: 1 - 0.1 * s, sy: 1 + 0.18 * s, k: 16 });
      return;
    }
    var f = x - 0.12;
    var y = ay + 6 + 0.5 * 6200 * f * f;
    if (y > 1180) return;
    var v = 6200 * f;
    var st = Math.min(1 + v * 0.00045, 1.9);
    var r = AR * (1 - 0.18 * M.prog(f, 0, 0.3));
    P.ball(AX, y, r, c, { sx: 1 / Math.sqrt(st), sy: st, k: 16 });
    var neck = 1 - M.prog(f, 0, 0.14);
    if (neck > 0) {
      P.cap(AX, ay + 4, AX, y - r * st * 0.6, 9 * neck, 6 * neck + 2, c, { k: 14 });
      P.ball(AX, ay + 2, 6 * neck + 1, c, { k: 12 });
    }
  }
  function rise(lt, P, m) {
    var x = lt - m[0];
    if (x < 0 || x > 0.5) return;
    var ay = AY(m[1]) + 36 * (1 - M.outBack(M.prog(x, 0, 0.5), 1.7));
    var r = x < 0.3 ? AR * M.outBack(M.prog(x, 0, 0.26), 2.4) : AR * (1 - M.inCubic(M.prog(x, 0.3, 0.46)));
    var w = M.wobble(lt, m[0] + 0.12, 5, 7, 0.14);
    P.ball(AX, ay, r, m[2], { sx: 1 + w, sy: 1 - w, k: 16 });
  }

  function jelly(lt, P) {
    if (lt < 0.72 || lt >= 2.68) return;
    var i = 0;
    while (i < HOPS.length - 1 && lt >= HOPS[i + 1][0]) i++;
    var cur = CHIP[HOPS[i][1]];
    var th = lt - HOPS[i][0];
    var L = cur.w * 0.6;
    var breathe = 1 + 0.03 * Math.sin(lt * 7.1);
    var r0 = 7.6;
    if (i === 0) {
      /* A bead drops from the search field into the underline. */
      var a = M.prog(lt, 0.72, 0.84);
      var by = M.lerp(JY - 46, JY, M.inCubic(a));
      if (a < 1) {
        P.ball(cur.x, by, 8.5, JELLY, { sx: 0.85, sy: 1.25, k: 18 });
        return;
      }
      var g = M.outBack(M.prog(lt, 0.84, 1.04), 2.2);
      var hl = Math.max(L * 0.5 * g, 0.5);
      var rr = r0 * (1 + 0.9 * (1 - M.prog(lt, 0.84, 1.0)));
      P.cap(cur.x - hl, JY, cur.x + hl, JY, rr * breathe, rr * breathe, JELLY, { k: 18 });
      return;
    }
    var prev = CHIP[HOPS[i - 1][1]];
    var hP = M.outCubic(M.prog(th, 0, 0.22));
    var tP = M.inOutCubic(M.prog(th, 0.06, 0.32));
    var head = M.lerp(prev.x + prev.w * 0.3, cur.x + L / 2, hP);
    var tail = M.lerp(prev.x - prev.w * 0.3, cur.x - L / 2, tP);
    var len = Math.max(head - tail, 1);
    var r = M.clamp(r0 * Math.sqrt(L / len), 3.2, r0);
    var arrive = M.wobble(lt, HOPS[i][0] + 0.22, 5.5, 6, 0.22);
    r *= breathe * (1 + arrive);
    /* departure: contract into a bead */
    var dep = M.prog(lt, 2.56, 2.66);
    if (dep > 0) {
      var c = cur.x;
      head = M.lerp(head, c, M.inCubic(dep));
      tail = M.lerp(tail, c, M.inCubic(dep));
      r = M.lerp(r, 10, dep);
    }
    P.cap(tail, JY, head, JY, r, r, JELLY, { k: 18 });
    var bulge = Math.sin(Math.PI * hP) * (1 - dep);
    if (bulge > 0.02) P.ball(head - 2, JY, r + 4.5 * bulge, JELLY, { sx: 1 + 0.35 * bulge, sy: 1 - 0.15 * bulge, k: 18 });
  }

  function bez(a, c, b, t) { var u = 1 - t; return u * u * a + 2 * u * t * c + t * t * b; }

  /* A jelly bead leaps from 附件名 and seeds the counter. */
  function leap(lt, P) {
    if (lt < 2.66 || lt >= 3.02) return;
    var x0 = CHIP[5].x, y0 = JY;
    var p = M.inOutCubic(M.prog(lt, 2.66, 2.88));
    var x = bez(x0, 820, NX, p), y = bez(y0, -40, NY, p);
    var e = 0.004;
    var p2 = M.inOutCubic(M.prog(lt + e, 2.66, 2.88));
    var vx = bez(x0, 820, NX, p2) - x, vy = bez(y0, -40, NY, p2) - y;
    var sp = Math.sqrt(vx * vx + vy * vy) / e;
    var st = M.clamp(1 + sp * 0.00022, 1, 1.7);
    var r = M.lerp(10, 34, M.prog(lt, 2.66, 2.86));
    if (lt > 2.86) r = 34 * (1 - M.inCubic(M.prog(lt, 2.86, 3.0)));
    P.ball(x, y, r, JELLY, { sx: st, sy: 1 / st, rot: Math.atan2(vy, vx), k: 20 });
  }

  function stat(lt, P) {
    if (lt < 2.78 || lt >= 5.5) return;
    var exitP = M.smooth(M.prog(lt, 5.0, 5.42));
    /* the counter */
    var p = M.prog(lt, 2.78, 3.0);
    var str = COUNT[Math.min(COUNT.length - 1, Math.floor(p * (COUNT.length - 1) + 1e-6))];
    var grow = M.lerp(0.22, 1, M.outCubic(p));
    var hit = 1 - M.wobble(lt, 3.0, 3.1, 5.5, 0.07);
    var breathe = 1 + 0.012 * Math.sin((lt - 3) * 4.4);
    var sc = grow * hit * breathe * (1 - 0.4 * exitP);
    P.text(0, specN(str), M.lerp(NX, 960, exitP), M.lerp(NY, 500, exitP), { scale: sc, color: "#F4F6FF", amount: 1 - exitP });

    /* impact shockwave + crown splash at 3.0 (47.0 s) */
    var rp = M.prog(lt, 3.0, 3.6);
    if (rp > 0 && rp < 1) P.ring(NX, NY, 600 + 560 * M.outCubic(rp), 7 * (1 - rp) + 0.8, "#bfe4ff", { k: 10 });
    if (lt >= 3.0 && lt < 3.9) {
      var R = M.rng(606);
      for (var i = 0; i < 8; i++) {
        var ox = 520 + 880 * (i + 0.5) / 8 + (R() - 0.5) * 60;
        var oy = NY - 92 + (R() - 0.5) * 20;
        var vx = (ox - NX) * (0.7 + R() * 0.5);
        var vy = -(620 + R() * 380);
        var f = lt - 3.0;
        var dr = (6 + R() * 6) * (1 - M.prog(f, 0.45, 0.85));
        P.ball(ox + vx * f, oy + vy * f + 0.5 * 2600 * f * f, dr, i % 3 === 0 ? "#bfe4ff" : "#F4F6FF", { k: 10 });
      }
    }

    /* A drip falls from the comma into 0.15 s. */
    if (lt >= 3.12 && lt < 3.56) {
      var g = M.prog(lt, 3.12, 3.38);
      var dr2 = 13 * M.outCubic(g);
      var hy = NUM_BASE + 16 + 22 * M.smooth(g);
      if (lt < 3.38) {
        P.ball(COMMA_X, hy, dr2, "#F4F6FF", { sx: 0.9, sy: 1.15, k: 14 });
        P.cap(COMMA_X, NUM_BASE - 6, COMMA_X, hy, 6 + 3 * g, dr2 * 0.6, "#F4F6FF", { k: 14 });
      } else {
        var ff = lt - 3.38;
        var fy = hy + 0.5 * 5200 * ff * ff;
        if (fy < S_TOP + 30) P.ball(COMMA_X, fy, 13, "#F4F6FF", { sx: 0.8, sy: 1.3, k: 18 });
        var neck = 1 - M.prog(ff, 0, 0.06);
        if (neck > 0) P.cap(COMMA_X, NUM_BASE - 6, COMMA_X, fy - 10, 4 * neck, 3 * neck, "#F4F6FF", { k: 12 });
      }
    }

    /* "0.15 s" blooms where the drip lands */
    if (lt >= 3.46) {
      var s0 = lt < 3.52 ? "0.00 s" : lt < 3.58 ? "0.04 s" : lt < 3.64 ? "0.09 s" : lt < 3.7 ? "0.13 s" : "0.15 s";
      var am = M.outCubic(M.prog(lt, 3.46, 3.72));
      var ss = (0.82 + 0.18 * M.spring(lt, 3.46, 2.6, 0.32)) * breathe * (1 - 0.45 * exitP);
      P.text(1, specS(s0), M.lerp(SXc, 960, exitP), M.lerp(SYc, 500, exitP), { scale: ss, color: "#2FD8E8", amount: am * (1 - exitP) });
    }
  }

  /* The stat gathers into one drop, flies into the magnifier. */
  function home(lt, P) {
    if (lt < 5.08 || lt >= 5.8) return;
    var g = M.outBack(M.prog(lt, 5.08, 5.38), 1.6);
    if (lt < 5.4) {
      P.ball(960, 500, 48 * g, "#7cc8ff", { k: 40 });
      return;
    }
    var p = M.inOutCubic(M.prog(lt, 5.4, 5.72));
    var tx = 579, ty = 137;
    var x = bez(960, 760, tx, p), y = bez(500, 120, ty, p);
    var p2 = M.inOutCubic(M.prog(lt + 0.004, 5.4, 5.72));
    var vx = bez(960, 760, tx, p2) - x, vy = bez(500, 120, ty, p2) - y;
    var st = M.clamp(1 + Math.sqrt(vx * vx + vy * vy) / 0.004 * 0.0002, 1, 1.8);
    var r = M.lerp(48, 14, p);
    if (lt > 5.72) r = 14 * (1 - M.prog(lt, 5.72, 5.79));
    P.ball(x, y, r, "#7cc8ff", { sx: st, sy: 1 / st, rot: Math.atan2(vy, vx), k: 24 });
  }

  LQ.add({ id: "s6", start: 44, end: 50, build: function (lt, P) {
    P.layer(1);
    for (var i = 0; i < MELTS.length; i++) melt(lt, P, MELTS[i]);
    for (i = 0; i < RISES.length; i++) rise(lt, P, RISES[i]);
    jelly(lt, P);
    leap(lt, P);
    stat(lt, P);
    home(lt, P);
    P.layer(0);
  } });
})();
