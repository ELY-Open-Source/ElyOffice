/* s7: liquid choreography, global 50s to 60s. Rules funnel, vCard bubbles, the MBOX jar,
   the .eml drop, the window sucked into the tray, and the melt into H7. Geometry mirrors scenes/s7.html. */
(function () {
  var M = LQ.M, C = LQ.C;
  var CLEAR = "#F4F6FF";
  function bez(a, c, b, t) { var u = 1 - t; return u * u * a + 2 * u * t * c + t * t * b; }
  /* a ball flying along a quadratic curve, stretched along its velocity */
  function flyer(P, lt, t0, t1, a, c, b, r0, r1, col, ease, k) {
    var e = ease || M.inOutCubic;
    var p = e(M.prog(lt, t0, t1)), p2 = e(M.prog(lt + 0.004, t0, t1));
    var x = bez(a[0], c[0], b[0], p), y = bez(a[1], c[1], b[1], p);
    var vx = bez(a[0], c[0], b[0], p2) - x, vy = bez(a[1], c[1], b[1], p2) - y;
    var r = M.lerp(r0, r1, M.prog(lt, t0, t1));
    var st = M.clamp(1 + Math.sqrt(vx * vx + vy * vy) / 0.004 * 0.00022, 1, r > 36 ? 1.3 : 1.75);
    P.ball(x, y, r, col, { sx: st, sy: 1 / st, rot: Math.atan2(vy, vx), k: k == null ? 30 : k });
    return [x, y];
  }

  /* ---------- A. the rules funnel ---------- */
  var FX = 1260, CHIP_Y = 130, FORK_Y = 520;
  var BUBS = [[1030, 712, "#FF7A2E"], [1490, 712, C.mail]];
  var MELT = [0.8, 1.2, 1.6, 2.0];
  var DCOL = ["#FFB46B", "#FF5FA8", "#FFB46B", "#3ddc84"];
  var DEST = [0, 1, 0, 1];
  var G = 4200, FALL = Math.sqrt((2 * (470 - CHIP_Y)) / G);
  function funnel(lt, P) {
    if (lt < 0.3 || lt >= 3.32) return;
    var g = M.outCubic(M.prog(lt, 0.3, 0.75));
    var out = 1 - M.inCubic(M.prog(lt, 3.0, 3.3));
    var wr = 12 * out * (0.4 + 0.6 * g);
    var walls = [[1080, 250, 1234, 410, 1240, 466], [1440, 250, 1286, 410, 1280, 466]];
    for (var i = 0; i < 2; i++) {
      var W = walls[i];
      var ex = M.lerp(W[0], W[2], g), ey = M.lerp(W[1], W[3], g);
      P.cap(W[0], W[1], ex, ey, wr, wr, CLEAR, { clear: 1, k: 12 });
      var g2 = M.prog(lt, 0.6, 0.8);
      if (g2 > 0) P.cap(W[2], W[3], M.lerp(W[2], W[4], g2), M.lerp(W[3], W[5], g2), wr, wr, CLEAR, { clear: 1, k: 12 });
      P.ball(W[0], W[1], wr * 1.25, CLEAR, { clear: 1, k: 12 });
    }
    var tg = M.outCubic(M.prog(lt, 0.55, 0.95)) * out;
    P.ball(FX, FORK_Y, 13 * tg, CLEAR, { clear: 1, k: 20 });
    for (i = 0; i < 2; i++) {
      var B = BUBS[i];
      var bx = M.lerp(FX, B[0], tg), by = M.lerp(FORK_Y, B[1], tg);
      P.cap(FX, FORK_Y, M.lerp(FX, bx, 0.82), M.lerp(FORK_Y, by, 0.82), 7 * tg, 7 * tg, CLEAR, { clear: 1, k: 22 });
      var land = 0;
      for (var d = 0; d < 4; d++) if (DEST[d] === i) land += M.wobble(lt, MELT[d] + 0.86, 3.6, 5, 0.07);
      var r = 92 * M.outBack(M.prog(lt, 0.62 + 0.06 * i, 1.0 + 0.06 * i), 1.8) * out * (1 + 0.012 * Math.sin(lt * 3.3 + i));
      P.ball(B[0], B[1], r, B[2], { sx: 1 + land, sy: 1 - land, k: 34 });
    }
  }
  function drop(lt, P, i) {
    var x = lt - MELT[i];
    if (x < 0 || x > 0.98) return;
    var col = DCOL[i], B = BUBS[DEST[i]];
    if (x < 0.06) {
      P.ball(FX, CHIP_Y, 26 * M.outBack(x / 0.06, 2), col, { sx: 1.3, sy: 0.8, k: 20 });
      return;
    }
    var f = x - 0.06;
    if (f < FALL) {
      var y = CHIP_Y + 0.5 * G * f * f;
      var v = G * f;
      var st = 1 + Math.min(v * 0.00032, 0.45);
      var sq = M.smooth(M.prog(y, 380, 430)) * (1 - M.smooth(M.prog(y, 455, 470)));
      var sxv = M.lerp(1 / st, 0.5, sq), syv = M.lerp(st, 1.75, sq);
      P.ball(FX, y, 25, col, { sx: sxv, sy: syv, k: 14 });
      return;
    }
    var h = f - FALL;
    if (h < 0.08) {
      P.ball(FX, M.lerp(470, FORK_Y, M.outCubic(h / 0.08)), 23, col, { sx: 0.8, sy: 1.25, k: 24 });
      return;
    }
    var b = M.inOutCubic(M.prog(h, 0.08, 0.4));
    var px = M.lerp(FX, B[0], b), py = M.lerp(FORK_Y, B[1], b);
    var r = h < 0.4 ? M.lerp(22, 17, b) : 17 * (1 - M.prog(h, 0.4, 0.5));
    var ang = Math.atan2(B[1] - FORK_Y, B[0] - FX);
    P.ball(px, py, r, col, { sx: 1.35, sy: 0.78, rot: ang, k: 30 });
  }

  /* ---------- B. contacts, vCard, MBOX, EML ---------- */
  var VC = [210, 350];
  var BUB = [3.5, 3.62, 3.74, 3.86];
  var CCOL = ["#d9468f", "#2e9e5b", "#e07a2a", "#1e8fe0"];
  function avatars(lt, P) {
    if (lt < 3.5 || lt >= 5.86) return;
    var out = M.prog(lt, 5.56, 5.82);
    for (var i = 0; i < 4; i++) {
      var t0 = BUB[i];
      if (lt < t0) continue;
      var tx = 380, ty = 336 + i * 88;
      var p = M.prog(lt, t0, t0 + 0.42);
      var r = 30 * M.outBack(M.prog(lt, t0, t0 + 0.3), 2.2);
      var x, y;
      if (p < 1) {
        var e = M.inOutCubic(p);
        x = bez(VC[0], 300, tx, e); y = bez(VC[1], 290 + i * 70, ty, e);
        if (lt < t0 + 0.1) P.cap(VC[0], VC[1] + 6, x, y, 9 * (1 - (lt - t0) / 0.1), r * 0.5, CCOL[i], { k: 20 });
        P.ball(x, y, r, CCOL[i], { sx: 0.9, sy: 1.1, k: 20 });
      } else {
        var wb = M.wobble(lt, t0 + 0.42, 4.5, 6, 0.16);
        var rr = r * (1 + 0.015 * Math.sin(lt * 4 + i)) * (1 - M.inCubic(out));
        P.ball(tx, ty + 60 * M.inCubic(out), rr, CCOL[i], { sx: 1 + wb, sy: 1 - wb, k: 20 });
      }
    }
  }
  var JX = 1000, JBOT = 590, MOUTH = 304;
  var BEAD = ["#1E8FE0", "#7B5CFF", "#FF5FA8", "#2FD8E8", "#1E8FE0", "#FFB46B", "#7B5CFF"];
  function jar(lt, P) {
    if (lt < 3.72 || lt >= 5.86) return;
    var out = 1 - M.inCubic(M.prog(lt, 5.56, 5.8));
    /* the pour: a gooey stream that slurps beads into the jar */
    var head = M.lerp(-30, MOUTH, M.inCubic(M.prog(lt, 3.75, 3.95)));
    var tail = M.lerp(-30, MOUTH, M.inCubic(M.prog(lt, 4.38, 4.56)));
    if (lt < 4.56 && head > tail + 2) P.cap(JX, tail, JX, head, 5, 6.5, C.mail, { k: 22 });
    for (var j = 0; j < BEAD.length; j++) {
      var bt = 3.8 + j * 0.085;
      var q = M.prog(lt, bt, bt + 0.24);
      if (q <= 0 || q >= 1) continue;
      var by = M.lerp(-30, MOUTH + 18, M.inCubic(q));
      var br = 13 * (1 - M.smooth(M.prog(by, MOUTH - 20, MOUTH + 18)));
      P.ball(JX, by, br, BEAD[j], { sx: 0.82, sy: 1.25, k: 22 });
    }
    /* the fill, seen through the frosted jar */
    var h = 196 * M.smooth(M.prog(lt, 3.92, 4.55)) * out;
    if (h > 2) {
      var wv = M.wobble(lt, 4.55, 3, 5, 6);
      P.box(JX, JBOT - h / 2, 176, h, Math.min(34, h / 2), C.mail, { k: 18 });
      P.ball(JX + wv, JBOT - h + 6, Math.min(36, h * 0.4), C.mail, { sx: 2.1, sy: 0.4, k: 26 });
    }
  }
  /* one message pops out of the jar as an .eml drop and opens as a card */
  var EX = 1520, EY = 485;
  function eml(lt, P) {
    if (lt < 4.55 || lt >= 5.42) return;
    if (lt < 4.62) {
      var s = M.outBack(M.prog(lt, 4.55, 4.62), 2.4);
      P.ball(JX, MOUTH - 8, 22 * s, C.mail, { sx: 0.85, sy: 1.2, k: 24 });
      P.cap(JX, MOUTH + 20, JX, MOUTH - 8, 8, 10, C.mail, { k: 20 });
      return;
    }
    if (lt < 4.9) {
      var neck = 1 - M.prog(lt, 4.62, 4.7);
      var pos = flyer(P, lt, 4.62, 4.9, [JX, MOUTH - 8], [1300, 60], [EX, EY], 22, 30, C.mail, M.inOutCubic, 24);
      if (neck > 0) P.cap(JX, MOUTH + 16, pos[0], pos[1], 6 * neck, 8 * neck, C.mail, { k: 20 });
      return;
    }
    var g = M.outBack(M.prog(lt, 4.92, 5.14), 1.6);
    var shrink = 1 - M.inCubic(M.prog(lt, 5.16, 5.4));
    var sq = M.wobble(lt, 4.9, 4, 6, 0.3) * (1 - g);
    var bw = M.lerp(64, 500, g) * shrink, bh = M.lerp(64, 290, g) * shrink;
    P.box(EX, EY, bw * (1 + sq), bh * (1 - sq), M.lerp(30, 24, g), C.mail, { k: 30 });
  }

  /* ---------- C. close: the window becomes a drop and flies into the tray ---------- */
  var SLOT = [1646, 28];
  function closeToTray(lt, P) {
    if (lt >= 6.42 && lt < 6.72) {
      var rp = M.prog(lt, 6.42, 6.72);
      P.layer(1);
      P.ring(229, 136, 8 + 30 * M.outCubic(rp), 3.5 * (1 - rp) + 0.2, "#ff8b8b", { k: 8 });
      P.layer(0);
    }
    if (lt < 6.64 || lt >= 9.3) return;
    if (lt < 7.0) {
      var p = M.prog(lt, 6.5, 6.86);
      var e = p * p * p;
      var sxW = 1 - 0.7 * e, syW = 1 - 0.88 * e;
      var c = M.smooth(M.prog(lt, 6.86, 7.0));
      var bw = M.lerp(1520 * sxW, 92, c), bh = M.lerp(780 * syW, 92, c);
      var cy = 572 + (494 - 572) * syW;
      P.box(960, cy, bw, bh, M.lerp(Math.min(30 + 60 * e, bh / 2), 46, c), C.mail, { k: 40 });
      return;
    }
    if (lt < 7.36) {
      flyer(P, lt, 7.0, 7.36, [960, 563], [1560, 420], SLOT, 46, 15, C.mail, M.inOutCubic, 30);
      return;
    }
    /* resting in the tray: lands, breathes, swallows three new-mail droplets */
    var land = M.wobble(lt, 7.36, 4, 6, 0.35);
    var blip = 0;
    /* three new mails sweep across the desktop and drop into the tray */
    var IN = [7.56, 7.7, 7.84], FLY = 0.52;
    var NCOL = ["#FF5FA8", "#3ddc84", "#2FD8E8"];
    for (var i = 0; i < 3; i++) {
      var L = IN[i] + FLY;
      blip += M.wobble(lt, L, 5, 8, 0.24);
      var q = M.prog(lt, IN[i], L);
      if (q > 0 && q < 1) {
        var A0 = [-40, 640 + 70 * i], C0 = [760 + 90 * i, 240 + 30 * i];
        var pos = flyer(P, lt, IN[i], L, A0, C0, SLOT, 22, 10, NCOL[i], M.inOutCubic, 26);
        var tq = M.prog(lt - 0.022, IN[i], L);
        if (tq > 0) {
          var tp = M.inOutCubic(tq);
          P.cap(bez(A0[0], C0[0], SLOT[0], tp), bez(A0[1], C0[1], SLOT[1], tp), pos[0], pos[1], 4, 14, NCOL[i], { k: 16 });
        }
      }
      var rr = M.prog(lt, L - 0.02, L + 0.36);
      if (rr > 0 && rr < 1) P.ring(SLOT[0], SLOT[1], 18 + 26 * M.outCubic(rr), 2.4 * (1 - rr) + 0.2, "#9fd8ff", { k: 8 });
    }
    var r = 15 * (1 + 0.02 * Math.sin(lt * 4.2));
    var drip = M.prog(lt, 9.08, 9.26);
    if (drip > 0) {
      /* the tray drop drips down into the melting notification */
      var dy = M.lerp(SLOT[1], 150, M.inCubic(drip));
      P.cap(SLOT[0], SLOT[1], SLOT[0] - 10 * drip, dy, 6 * (1 - drip), 12, C.mail, { k: 26 });
      P.ball(SLOT[0] - 10 * drip, dy, r, C.mail, { k: 30 });
      return;
    }
    P.ball(SLOT[0], SLOT[1], r, C.mail, { sx: 1 + land + blip, sy: 1 - land - blip, k: 30 });
  }

  /* ---------- D. the notification melts and everything falls into H7 ---------- */
  var NC = [1636, 158];
  function finale(lt, P) {
    if (lt < 9.0) return;
    if (lt >= 9.95) {
      P.ball(960, 500, 120, C.mail);
      return;
    }
    if (lt < 9.28) {
      var g = M.outBack(M.prog(lt, 9.0, 9.24), 1.6);
      P.ball(NC[0], NC[1], 60 * g, C.mail, { k: 40 });
      return;
    }
    if (lt < 9.62) {
      flyer(P, lt, 9.28, 9.62, [NC[0], NC[1]], [1320, 110], [960, 500], 62, 104, C.mail, M.inOutCubic, 40);
      return;
    }
    var settle = 1 - M.smooth(M.prog(lt, 9.72, 9.95));
    var grow = M.lerp(104, 120, M.outCubic(M.prog(lt, 9.62, 9.76)));
    var w = M.wobble(lt, 9.62, 2.6, 5.5, 0.2) * settle;
    P.ball(960, 500 + 18 * w, grow, C.mail, { sx: 1 + w, sy: 1 - w });
  }

  LQ.add({ id: "s7", start: 50, end: 60, build: function (lt, P) {
    /* a richer body for the small coloured drops; back to the default before the H7 ball */
    var rich = 1 - M.smooth(M.prog(lt, 5.6, 6.0));
    if (rich > 0) P.material({ body: 0.42 + 0.2 * rich });
    funnel(lt, P);
    for (var i = 0; i < 4; i++) drop(lt, P, i);
    avatars(lt, P);
    jar(lt, P);
    eml(lt, P);
    closeToTray(lt, P);
    finale(lt, P);
  } });
})();
