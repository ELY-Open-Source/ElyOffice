/* s8 montage, 60–78 s: cards melt to drops between vignettes. */
(function () {
  "use strict";
  var M = LQ.M, C = LQ.C;
  var IN = 14; // liquid sits this far inside the glass
  var V = [
    [[616, 356, 320, 240, C.doc], [960, 356, 320, 240, C.table], [1304, 356, 320, 240, C.show],
     [616, 620, 320, 240, C.pdf], [960, 620, 320, 240, C.tree], [1304, 620, 320, 240, C.mark]],
    [[560, 460, 520, 420, C.violet], [1244, 460, 752, 500, C.cyan]],
    [[690, 440, 820, 400, C.lime], [1390, 460, 500, 380, C.peach]],
    [[780, 446, 960, 492, C.table], [1454, 446, 332, 492, C.pink]],
    [[620, 460, 640, 520, C.doc], [1300, 460, 640, 520, C.mark]],
    [[600, 380, 600, 400, C.doc], [1280, 380, 680, 400, C.pdf], [960, 660, 1320, 112, C.lime]],
  ];
  var T = [3.0, 6.5, 9.5, 12.5, 14.75];
  var OUT = 17.1;
  var ROW = [C.doc, C.table, C.show, C.mail, C.pdf, C.tree, C.mark];

  function rgb(h) { return M.hex(h).slice(0, 3); }
  function rr(r) { return { x: r[0], y: r[1], w: r[2] - IN * 2, h: r[3] - IN * 2, c: r[4] }; }
  function ballR(r) { return M.clamp(Math.min(r.w, r.h) * 0.26, 44, 100); }
  /* Impact: starts at full squash, rings down. */
  function imp(t, t0, f, d) {
    var x = t - t0;
    if (x < 0) return 0;
    return Math.exp(-(d || 7) * x) * Math.cos(2 * Math.PI * (f || 4) * x);
  }
  function ease2(p) { return p * p * (1.6 - 0.6 * p); } // accelerates into the end stop

  /* Shared with s8.html (read lazily at seek time). */
  var F = (window.S8F = {
    fill: function (lt, a) {
      return 0.62 * M.inOutCubic(M.prog(lt, a, a + 0.55)) + 0.38 * M.inOutCubic(M.prog(lt, a + 0.7, a + 1.15));
    },
    scroll: function (lt) {
      var f = ease2(M.prog(lt, 12.98, 13.55)) - ease2(M.prog(lt, 13.72, 14.08));
      return { f: f, ib: imp(lt, 13.55, 4, 7), it: imp(lt, 14.08, 4, 7) };
    },
    guide: function (lt) { return 200 + 54 * M.inOutCubic(M.prog(lt, 10.9, 11.2)); },
    colA: function (lt) {
      return 200 + 54 * M.spring(lt, 11.25, 3, 0.5) + 86 * M.spring(lt, 11.5, 2.2, 0.32);
    },
  });

  function path(s, d, m) {
    var dx = d.x - s.x, dy = d.y - s.y;
    var lift = Math.min(140, Math.sqrt(dx * dx + dy * dy) * 0.22);
    return { x: s.x + dx * m, y: s.y + dy * m - Math.sin(Math.PI * m) * lift };
  }

  /* One card-to-card traveller. tau = seconds from the cut T. */
  function trav(P, tau, j, s, d, full, sync) {
    var dl = sync ? 0 : j * 0.04;
    var g = full ? 1 : M.outCubic(M.prog(tau, -0.45, -0.2));
    var mt = function (x) { return M.inOutCubic(M.prog(x, -0.06 + dl, 0.3 + dl)); };
    var c = M.inOutCubic(M.prog(tau, -0.27 + dl * 0.5, 0.0 + dl * 0.5));
    var m = mt(tau);
    var e = M.spring(tau, 0.22 + dl, 2.4, 0.4);
    var hide = M.inCubic(M.prog(tau, 0.62 + dl, 0.92 + dl));
    if (g <= 0.01 || hide >= 0.999) return;
    var Rs = ballR(s), Rd = ballR(d);
    var p = path(s, d, m);
    var col = M.mix3(rgb(s.c), rgb(d.c), M.smooth(M.prog(m, 0.2, 0.9)));
    if (e > 0) {
      var kk = 1 - e;
      var w = M.lerp(d.w, 2 * Rd, kk) * (1 - hide), h = M.lerp(d.h, 2 * Rd, kk) * (1 - hide);
      var rad = M.clamp(M.lerp(20, Rd, kk), 8, Rd);
      P.box(p.x, p.y, w, h, rad * (1 - hide * 0.5), col, { k: M.lerp(60, 14, M.clamp(e, 0, 1)) });
      return;
    }
    var R = M.lerp(Rs, Rd, m);
    if (c < 0.995) {
      P.box(p.x, p.y, M.lerp(s.w, 2 * R, c) * g, M.lerp(s.h, 2 * R, c) * g, M.lerp(20, R, c) * g, col, { k: M.lerp(14, 60, c) });
      return;
    }
    var p2 = path(s, d, mt(tau + 0.012));
    var vx = (p2.x - p.x) / 0.012, vy = (p2.y - p.y) / 0.012;
    var sp = Math.sqrt(vx * vx + vy * vy);
    var st = Math.min(0.36, sp / 6000);
    var a = Math.atan2(vy, vx);
    var an = 0.16 * Math.sin(Math.PI * M.prog(tau, -0.1 + dl, 0.02 + dl));
    P.ball(p.x, p.y + R * an * 0.6, R, col, { sx: (1 + st) * (1 + an), sy: 1 / ((1 + st) * (1 + an)), rot: st > 0.02 ? a : 0, k: 60 });
    if (st > 0.06) {
      var L = R * st * 2.4;
      P.cap(p.x, p.y, p.x - Math.cos(a) * (R + L), p.y - Math.sin(a) * (R + L), R * 0.55, R * 0.25, col, { k: 40 });
    }
  }

  function hop(P, lt, h) {
    var s = V[h], d = V[h + 1], n = Math.max(s.length, d.length);
    // 2 -> 2 hops swap sides; the drops fuse mid-screen.
    var sw = s.length === 2 && d.length === 2 ? 1 : 0;
    for (var j = 0; j < n; j++) trav(P, lt - T[h], j, rr(s[j % s.length]), rr(d[(j + sw) % d.length]), h === 0, sw);
  }

  /* H7 bursts into six drops that grow into the bento. */
  function intro(P, lt) {
    var a0 = 0.17 * M.smooth(M.prog(lt, 0.12, 0.5));
    if (lt < 0.5) {
      P.ball(960, 500 + 120 * a0, 120, C.mail, { sx: 1 + a0, sy: 1 - a0 });
      return;
    }
    var sq = 0.17 * (1 - M.spring(lt, 0.5, 3, 0.35));
    var hubR = 120 * (1 - M.smooth(M.prog(lt, 0.52, 1.0)));
    var hy = 500 + 120 * sq;
    if (hubR > 1) P.ball(960, hy, hubR, C.mail, { sx: 1 + sq, sy: 1 - sq, k: 50 });
    // a satellite droplet hops off the hub as it vanishes
    var sat = M.prog(lt, 0.92, 1.5);
    if (sat > 0 && sat < 1) {
      var v0 = 520, tt = lt - 0.92, y = 470 - (v0 * tt - 1300 * tt * tt);
      P.ball(960, Math.min(y, 520), 14 * (1 - M.inCubic(sat)), C.white, { k: 20 });
    }
    var neck = 26 * (1 - M.prog(lt, 0.55, 0.9));
    var white = rgb(C.mail);
    for (var i = 0; i < 6; i++) {
      var tile = V[0][i], t0 = 0.5 + i * 0.035;
      var qf = function (x) { return M.spring(x, t0, 1.45, 0.5); };
      var q = qf(lt);
      var px = 960 + (tile[0] - 960) * q, py = hy + (tile[1] - hy) * q - Math.sin(Math.PI * M.clamp(q, 0, 1)) * 70;
      var r = 56 * M.outBack(M.prog(lt, t0, t0 + 0.3), 1.6);
      var col = M.mix3(white, rgb(tile[4]), M.smooth(M.prog(lt, t0 + 0.12, t0 + 0.6)));
      var te = 1.32 + i * 0.045;
      var e = M.spring(lt, te, 2.3, 0.42);
      var lock = M.smooth(M.prog(lt, te - 0.05, te + 0.4));
      var cx = M.lerp(px, tile[0], lock), cy = M.lerp(py, tile[1], lock);
      if (e > 0) {
        var kk = 1 - e, R = rr(tile);
        P.box(cx, cy, M.lerp(R.w, 112, kk), M.lerp(R.h, 112, kk), M.clamp(M.lerp(20, 56, kk), 8, 56), col, { k: M.lerp(50, 14, M.clamp(e, 0, 1)) });
      } else if (r > 0.5) {
        var q2 = qf(lt + 0.012);
        var vx = (tile[0] - 960) * (q2 - q) / 0.012, vy = (tile[1] - hy) * (q2 - q) / 0.012;
        var st = Math.min(0.45, Math.sqrt(vx * vx + vy * vy) / 4200);
        P.ball(cx, cy, r, col, { sx: 1 + st, sy: 1 / (1 + st), rot: Math.atan2(vy, vx), k: 50 });
      }
      if (neck > 3 && r > 0.5) P.cap(960, hy, cx, cy, neck * 1.1, neck * 0.6, M.mix3(white, col, 0.5), { k: 30 });
    }
  }

  /* V6 cards melt into three drops; seven roll into H8. */
  var SRC_OF = [0, 0, 2, 2, 1, 1, 1];
  function outro(P, lt) {
    var tau = lt - OUT;
    var S = V[5].map(rr);
    var g = M.outCubic(M.prog(tau, -0.45, -0.2));
    var c = M.inOutCubic(M.prog(tau, -0.27, 0.02));
    var left = [0, 0, 0], tot = [2, 3, 2];
    var kids = [];
    for (var i = 0; i < 7; i++) {
      var td = 0.03 + i * 0.03;
      var u = M.prog(tau, td, td + 0.38);
      left[SRC_OF[i]] += M.smooth(M.prog(u, 0, 0.55));
      kids.push({ i: i, td: td, u: u });
    }
    for (var s = 0; s < 3; s++) {
      var r0 = S[s], R = ballR(r0);
      var rem = 1 - left[s] / tot[s];
      if (rem <= 0.01) continue;
      if (c < 0.995) {
        P.box(r0.x, r0.y, M.lerp(r0.w, 2 * R, c) * g, M.lerp(r0.h, 2 * R, c) * g, M.lerp(20, R, c) * g, r0.c, { k: M.lerp(14, 60, c) });
      } else {
        var j = M.wobble(lt, OUT, 3, 6, 0.08);
        P.ball(r0.x, r0.y, R * Math.sqrt(rem), r0.c, { sx: 1 + j, sy: 1 - j, k: 60 });
      }
    }
    var settle = M.smooth(M.prog(lt, 17.72, 17.975));
    for (var k = 0; k < 7; k++) {
      var K = kids[k];
      if (K.u <= 0) continue;
      var src = S[SRC_OF[k]];
      var tx = 960 + (k - 3) * 200, ty = 500;
      var ue = M.inOutCubic(K.u);
      var lift = 150 + Math.abs(tx - src.x) * 0.1;
      var x = M.lerp(src.x, tx, ue), y = M.lerp(src.y, ty, ue) - Math.sin(Math.PI * ue) * lift;
      var r = M.lerp(ballR(src) * 0.55, 62, M.smooth(M.prog(K.u, 0, 0.6)));
      var col = M.mix3(rgb(src.c), rgb(ROW[k]), M.smooth(M.prog(K.u, 0.05, 0.6)));
      var land = OUT + K.td + 0.38;
      var w = 0.3 * imp(lt, land, 3.4, 7);
      var sx = 1, sy = 1;
      if (K.u >= 1) {
        sx = 1 + w;
        sy = 1 / (1 + w);
        y = ty + 62 * (1 - sy);
      } else {
        var u2 = M.inOutCubic(M.prog(lt + 0.012 - OUT, K.td, K.td + 0.38));
        var vx = (tx - src.x) * (u2 - ue) / 0.012;
        var vy = ((ty - src.y) * (u2 - ue) - (Math.sin(Math.PI * u2) - Math.sin(Math.PI * ue)) * lift) / 0.012;
        var st = Math.min(0.4, Math.sqrt(vx * vx + vy * vy) / 6000);
        P.ball(x, y, r, col, { sx: 1 + st, sy: 1 / (1 + st), rot: Math.atan2(vy, vx), k: 40 });
        if (K.u < 0.35) {
          var nk = r * 0.5 * (1 - K.u / 0.35);
          if (nk > 3) P.cap(src.x, src.y, x, y, nk, nk * 0.7, col, { k: 40 });
        }
        continue;
      }
      x = M.lerp(x, tx, settle);
      y = M.lerp(y, ty, settle);
      sx = M.lerp(sx, 1, settle);
      sy = M.lerp(sy, 1, settle);
      if (settle >= 0.999) P.ball(tx, ty, 62, ROW[k]);
      else P.ball(x, y, 62, col, { sx: sx, sy: sy });
    }
  }

  /* Front layer: a bubble pops, then HTML takes over. */
  function pop(P, lt, x, y, R, t0, col) {
    var x0 = lt - t0;
    if (x0 < 0 || x0 > 0.45) return;
    var r = R * M.spring(lt, t0, 3.2, 0.38) * (1 - M.inCubic(M.prog(lt, t0 + 0.2, t0 + 0.34)));
    if (r > 0.5) P.ball(x, y, r, col, { k: 16 });
    var u = M.prog(lt, t0 + 0.08, t0 + 0.45);
    if (u > 0 && u < 1) P.ring(x, y, R * (1 + 1.3 * M.outCubic(u)), 2.6 * (1 - u), C.white, { k: 4 });
  }
  function bar(P, lt, y, a, col, out) {
    if (lt < a - 0.05) return;
    var L = 1077, Rx = 1443;
    var p = F.fill(lt, a);
    var r = 13 * out * M.outBack(M.prog(lt, a - 0.05, a + 0.15), 2);
    if (r < 0.5) return;
    var x = M.lerp(L, Rx, p);
    var v = (F.fill(lt + 0.01, a) - p) / 0.01 * (Rx - L);
    var b = M.clamp(v / 1400, 0, 0.4) + M.wobble(lt, a + 1.15, 5, 6, 0.2);
    P.cap(L, y, x, y, r, r, col, { k: 12 });
    P.ball(x, y, r * 1.06, col, { sx: 1 + b, sy: 1 / (1 + b), k: 14 });
    // a bubble rides inside the meniscus while it runs
    if (v > 120) P.ball(x - 26 - 10 * Math.sin(lt * 9), y - 3, 3.2, C.white, { k: 6, clear: 1 });
  }
  function fx2(P, lt) {
    if (lt < 3.7 || lt > 6.2) return;
    var out = 1 - M.inCubic(M.prog(lt, 5.95, 6.12));
    bar(P, lt, 474, 3.75, C.cyan, out);
    bar(P, lt, 550, 3.9, C.lime, out);
    pop(P, lt, 736, 454, 28, 3.85, C.table);
    pop(P, lt, 736, 574, 32, 4.3, C.table);
  }
  /* The command spits a mail drop onto the ely-mail package. */
  function fx3(P, lt) {
    var t0 = 8.2;
    if (lt < t0 || lt > 9.0) return;
    var u = M.prog(lt, t0 + 0.1, t0 + 0.52);
    var ux = M.inOutCubic(u);
    var x = M.lerp(1030, 400, ux), y = M.lerp(344, 564, u * u) - Math.sin(Math.PI * u) * 200;
    var r = 22 * M.outBack(M.prog(lt, t0, t0 + 0.12), 2.4) * (1 - M.inCubic(M.prog(lt, t0 + 0.62, t0 + 0.78)));
    if (r < 0.5) return;
    if (u < 1) {
      var u2 = M.prog(lt + 0.01, t0 + 0.1, t0 + 0.52);
      var vx = (M.inOutCubic(u2) - ux) * -630 / 0.01, vy = ((u2 * u2 - u * u) * 220 - (Math.sin(Math.PI * u2) - Math.sin(Math.PI * u)) * 200) / 0.01;
      var st = u > 0 ? Math.min(0.5, Math.sqrt(vx * vx + vy * vy) / 4000) : 0;
      P.ball(x, y, r, C.mail, { sx: 1 + st, sy: 1 / (1 + st), rot: Math.atan2(vy, vx), k: 20 });
    } else {
      var w = 0.45 * imp(lt, t0 + 0.52, 4, 8);
      P.ball(400, 564 + r * w * 0.6, r, C.mail, { sx: 1 + w, sy: 1 / (1 + w), k: 20 });
    }
  }
  /* A cyan drop arcs from the swatch into D2, splashing. */
  function fx4(P, lt) {
    var t0 = 10.1;
    if (lt < t0 || lt > 11.05) return;
    var land = t0 + 0.42, X = 980, Y = 440;
    var u = M.prog(lt, t0 + 0.06, land);
    if (lt < land) {
      var r = 15 * M.outBack(M.prog(lt, t0, t0 + 0.12), 2.2);
      var x = M.lerp(1450, X, M.inOutCubic(u)), y = M.lerp(342, Y, u * u) - Math.sin(Math.PI * u) * 170;
      var st = u > 0.05 && u < 0.95 ? 0.35 : 0;
      var u2 = M.prog(lt + 0.01, t0 + 0.06, land);
      var vx = M.lerp(1450, X, M.inOutCubic(u2)) - x, vy = M.lerp(342, Y, u2 * u2) - Math.sin(Math.PI * u2) * 170 - y;
      P.ball(x, y, r, C.cyan, { sx: 1 + st, sy: 1 / (1 + st), rot: Math.atan2(vy, vx), k: 16 });
      if (u > 0.1) P.cap(x, y, x - vx * 3, y - vy * 3, r * 0.45, 1.5, C.cyan, { k: 12 });
      return;
    }
    var s = lt - land;
    var flat = 1 - M.inCubic(M.prog(s, 0.04, 0.2));
    if (flat > 0.02) P.ball(X, Y + 6, 15 * flat, C.cyan, { sx: 1.9, sy: 0.5, k: 12 });
    var rg = M.rng(48213);
    for (var i = 0; i < 6; i++) {
      var vx0 = (rg() - 0.5) * 520, vy0 = -260 - rg() * 260, rr0 = 4 + rg() * 4;
      var life = 1 - M.prog(s, 0.05, 0.42);
      if (life <= 0) continue;
      P.ball(X + vx0 * s, Y + vy0 * s + 1500 * s * s, rr0 * life, C.cyan, { k: 8 });
    }
    var ru = M.prog(s, 0, 0.35);
    if (ru < 1) P.ring(X, Y, 18 + 46 * M.outCubic(ru), 2.4 * (1 - ru), C.cyan, { k: 4 });
  }
  /* Liquid scrollbar thumb (ElyDoc) and the line-number switch knob (ElyMark). */
  function fx5(P, lt) {
    if (lt < 12.85 || lt > 14.36) return;
    var out = 1 - M.inCubic(M.prog(lt, 14.2, 14.34));
    var app = M.spring(lt, 12.88, 3, 0.5) * out;
    if (app < 0.02) return;
    var S = F.scroll(lt), S2 = F.scroll(lt + 0.01);
    var H = 48, top = 296, bot = 688;
    var v = (S2.f - S.f) / 0.01 * (bot - top - 2 * H);
    var sq = 0.42 * (S.ib + S.it);
    var Hs = H * (1 - sq), vs = Math.min(0.3, Math.abs(v) / 4000);
    var cy = M.lerp(top + H, bot - H, S.f) + (H - Hs) * (S.it !== 0 ? -1 : 1);
    var Hh = Hs * (1 + vs), r = 10 * (1 + sq * 0.9) / (1 + vs * 0.5);
    Hh *= app;
    r *= app;
    P.cap(898, cy - Hh + r, 898, cy + Hh - r, r, r, C.doc, { k: 10 });
    // switch knob
    var q = M.spring(lt, 13.05, 3.2, 0.45);
    var q2 = M.spring(lt + 0.01, 13.05, 3.2, 0.45);
    var sx = 1 + Math.min(0.5, Math.abs(q2 - q) / 0.01 / 12);
    var kc = M.mix3(rgb(C.white), rgb(C.mark), M.smooth(M.prog(q, 0.2, 0.9)));
    P.ball(M.lerp(1542, 1570, q), 248, 13 * app, kc, { sx: sx, sy: 1 / sx, k: 8 });
  }
  /* A red drop hits the edited digit; check bubbles pop. */
  function fx6(P, lt) {
    var t0 = 15.55, land = 15.8, X = 1186, Y = 352;
    if (lt >= t0 && lt < land + 0.2) {
      var r = 13 * M.outBack(M.prog(lt, t0, t0 + 0.1), 2.2);
      if (lt < land) {
        var u = M.prog(lt, t0 + 0.08, land);
        var y = M.lerp(250, Y - 10, u * u);
        var st = 0.5 * u;
        P.ball(X, y, r, C.pdf, { sx: 1 / (1 + st), sy: 1 + st, k: 12 });
        if (u > 0.15) P.cap(X, y, X, y - 20 - 50 * u, r * 0.45, 1.5, C.pdf, { k: 10 });
      } else {
        var s = M.prog(lt, land, land + 0.18);
        P.ball(X, Y - 4, r * (1 - M.inCubic(s)), C.pdf, { sx: 1.8, sy: 0.55, k: 10 });
      }
    }
    pop(P, lt, 360, 660, 28, 15.95, C.table);
    pop(P, lt, 1020, 660, 28, 16.12, C.table);
  }

  LQ.add({ id: "s8", start: 60, end: 78, build: function (lt, P) {
    if (lt < T[0] - 0.45) intro(P, lt);
    for (var h = 0; h < T.length; h++) if (lt >= T[h] - 0.45 && lt < T[h] + 1.1) hop(P, lt, h);
    if (lt >= OUT - 0.45) outro(P, lt);
    P.layer(1);
    fx2(P, lt);
    fx3(P, lt);
    fx4(P, lt);
    fx5(P, lt);
    fx6(P, lt);
    P.layer(0);
  } });
})();
