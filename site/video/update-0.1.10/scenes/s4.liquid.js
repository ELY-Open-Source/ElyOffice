/* s4 reading, 26–34 s; window.S4X shares the camera with s4.html. */
(function () {
  "use strict";
  var M = LQ.M, C = LQ.C;
  var X = (window.S4X = {});

  /* Camera: the wrapper's transform, p' = p*s + t. */
  var ID = { s: 1, x: 0, y: 0 };
  var A = { s: 1.25, x: -290, y: -150 };
  var B = { s: 1.2, x: -806, y: -150 };
  function q4(u) {
    u = M.clamp(u, 0, 1);
    return u < 0.5 ? 8 * u * u * u * u : 1 - Math.pow(-2 * u + 2, 4) / 2;
  }
  var KEYS = [[0.5, 1.5, ID, A], [3.85, 4.75, A, B], [7.1, 7.95, B, ID]];
  X.cam = function (lt) {
    var c = ID;
    for (var i = 0; i < KEYS.length; i++) {
      var k = KEYS[i];
      if (lt < k[0]) break;
      var u = q4((lt - k[0]) / (k[1] - k[0]));
      c = { s: M.lerp(k[2].s, k[3].s, u), x: M.lerp(k[2].x, k[3].x, u), y: M.lerp(k[2].y, k[3].y, u) };
    }
    return c;
  };
  X.rest = function (lt) {
    return lt < 0.5 || lt >= 7.95;
  };
  /* identity-screen point -> screen under the camera at lt */
  X.sc = function (lt, x, y) {
    var c = X.cam(lt);
    return [x * c.s + c.x, y * c.s + c.y, c.s];
  };
  /* decaying cosine: 1 at T, used for impact squash */
  X.jelly = function (lt, T, f, d) {
    var x = lt - T;
    if (x < 0) return 0;
    return Math.exp(-(d || 5) * x) * Math.cos(2 * Math.PI * (f || 4) * x);
  };

  /* ---------- thread (reader overlay; origin identity (1005, 251)) ---------- */
  X.O = [1005, 251];
  X.T = [1.0, 1.42, 1.84, 2.26]; // card landings
  X.EXP = 2.7; // newest card opens
  X.cardTop = function (i) {
    return 84 + 76 * i;
  };
  X.card = function (i, lt) {
    var T = X.T[i], t0 = T - 0.3;
    var p = M.prog(lt, t0, T);
    var dy = -36 * (1 - p * p);
    var o = M.smooth(M.prog(lt, t0, t0 + 0.2));
    var w = M.wobble(lt, T, 3.4, 5.5, 1);
    var sy = 1 - 0.13 * w, sx = 1 + 0.05 * w;
    for (var j = i + 1; j < 4; j++) {
      var wj = M.wobble(lt, X.T[j] + 0.02, 3.8, 6.5, 1) * (j === i + 1 ? 1 : 0.45);
      dy += 4 * wj;
      sy -= 0.02 * wj;
    }
    if (i < 3) {
      var we = M.wobble(lt, X.EXP + 0.05, 3.2, 6, 1) * 0.4;
      dy -= 2.5 * we;
    }
    return { dy: dy, o: o, sx: sx, sy: sy };
  };
  X.h3 = function (lt) {
    return 64 + 132 * M.spring(lt, X.EXP, 2.4, 0.5);
  };
  X.bead = function (i, lt) {
    var cd = X.card(i, lt);
    return X.sc(lt, X.O[0] + 22, X.O[1] + X.cardTop(i) + 32 + cd.dy);
  };

  /* Newsletter, shield and drops, in screen space under camera B. */
  X.SH = { x: 1282, y: 300, w: 96, h: 360 }; // glass shield (screen)
  X.HITS = [5.4, 5.85, 6.72];
  X.LOAD = 6.0; // "载入图片" tap
  X.BTN = [1115, 437]; // its identity-screen centre
  X.SLOT = [[807, 554], [598, 768], [1015, 768]]; // hero, thumb 1, thumb 2 (screen under B)
  X.Q = [[1500, 606], [1596, 650], [1690, 600]];
  X.PR = [30, 25, 27];
  X.PC = [C.pink, C.peach, C.cyan];
  X.PA = [4.8, 4.95, 5.1];
  X.PD = [6.08, 6.18, 6.28];
  X.FLY = 0.38;
  X.land = function (j) {
    return X.PD[j] + X.FLY;
  };
  X.bloom = function (j, lt) {
    return M.outCubic(M.prog(lt, X.land(j) - 0.02, X.land(j) + 0.5));
  };
  X.openness = function (lt) {
    return M.smooth(M.prog(lt, 6.03, 6.12)) * (1 - M.smooth(M.prog(lt, 6.52, 6.64)));
  };
  X.shield = function (lt) {
    if (lt < 4.5 || lt > 7.25) return null;
    var s = M.spring(lt, 4.5, 2.4, 0.42) * (1 - M.inCubic(M.prog(lt, 6.98, 7.22)));
    var dx = 0, sq = 0;
    for (var i = 0; i < X.HITS.length; i++) {
      dx -= 9 * M.wobble(lt, X.HITS[i], 4.2, 7, 1);
      sq += 0.06 * M.wobble(lt, X.HITS[i], 4.2, 7, 1);
    }
    var op = X.openness(lt);
    return { s: s, dx: dx, sx: 1 - sq - 0.18 * op, sy: 1 + sq * 0.6, o: 1 - 0.62 * op, open: op };
  };

  var C1 = [1392, 404], C2 = [1392, 440], C3 = [1392, 516];
  X.CY = [C1[1], C2[1], C3[1]];
  /* The tracking pixel: a tiny dark drop with comedic timing. */
  X.dark = function (lt) {
    if (lt < 5.0 || lt > 7.1) return null;
    var x, y, sx = 1, sy = 1, rot = 0, p, w, r = 19;
    if (lt < 5.4) {
      p = M.prog(lt, 5.0, 5.4);
      var e = p * p;
      x = M.lerp(2060, C1[0], e);
      y = M.lerp(352, C1[1], e) + 7 * Math.sin(p * 10);
      sx = 1 + 0.9 * p;
      sy = 1 / Math.sqrt(sx);
    } else if (lt < 5.75) {
      p = M.outCubic(M.prog(lt, 5.4, 5.75));
      x = M.lerp(C1[0], 1592, p);
      y = M.lerp(C1[1], 352, p) - 46 * Math.sin(Math.PI * p);
      w = X.jelly(lt, 5.4, 4.5, 6);
      sx = 1 - 0.45 * w;
      sy = 1 + 0.4 * w;
    } else if (lt < 5.79) {
      p = M.smooth(M.prog(lt, 5.75, 5.79));
      x = M.lerp(1592, 1652, p);
      y = 352 + 6 * p;
      sx = 0.86;
      sy = 1.12;
    } else if (lt < 5.85) {
      p = M.prog(lt, 5.79, 5.85);
      x = M.lerp(1652, C2[0], p * p);
      y = M.lerp(358, C2[1], p);
      sx = 1 + 1.3 * p;
      sy = 1 / Math.sqrt(sx);
    } else if (lt < 6.25) {
      p = M.outCubic(M.prog(lt, 5.85, 6.25));
      x = M.lerp(C2[0], 1740, p);
      y = M.lerp(C2[1], 470, p) - 80 * Math.sin(Math.PI * p);
      w = X.jelly(lt, 5.85, 4.2, 5);
      sx = 1 - 0.55 * w;
      sy = 1 + 0.5 * w;
      rot = 6.3 * p;
    } else if (lt < 6.55) {
      var d = lt - 6.25;
      x = 1740 + 9 * Math.cos(d * 22);
      y = 470 + 5 * Math.sin(d * 22);
      sx = 1 + 0.08 * Math.sin(d * 30);
      sy = 1 - 0.08 * Math.sin(d * 30);
    } else if (lt < 6.72) {
      p = M.prog(lt, 6.55, 6.72);
      var x0 = 1740 + 9 * Math.cos(0.3 * 22), y0 = 470 + 5 * Math.sin(0.3 * 22);
      x = M.lerp(x0, C3[0], p * p);
      y = M.lerp(y0, C3[1], p);
      sx = 1 + 1.1 * p;
      sy = 1 / Math.sqrt(sx);
      rot = Math.atan2(C3[1] - y0, C3[0] - x0) - Math.PI;
    } else {
      var t = lt - 6.72;
      var vx = 1150, vy = -2600, g = 2400;
      x = C3[0] + vx * t;
      y = C3[1] + vy * t + 0.5 * g * t * t;
      w = X.jelly(lt, 6.72, 4.5, 6);
      var spd = Math.sqrt(vx * vx + (vy + g * t) * (vy + g * t));
      rot = Math.atan2(vy + g * t, vx);
      sx = 1 + Math.min(spd / 3200, 0.9) * M.prog(lt, 6.74, 6.8) - 0.4 * w * (1 - M.prog(lt, 6.72, 6.76));
      sy = 1 / Math.sqrt(Math.max(sx, 0.4));
      if (y < -60) return null;
    }
    return { x: x, y: y, r: r, sx: sx, sy: sy, rot: rot };
  };

  function pastel(j, lt) {
    var q = X.Q[j], r = X.PR[j], a = X.PA[j], D = X.PD[j], L = X.land(j);
    if (lt < a - 0.5 || lt > L + 0.2) return null;
    var x, y, sx = 1, sy = 1, rot = 0, p;
    if (lt < D) {
      p = M.prog(lt, a - 0.5, a);
      var e = M.outBack(p, 1.3);
      x = M.lerp(2010, q[0], e);
      y = q[1] + 4 * Math.sin(2 * Math.PI * 1.1 * lt + j * 1.7);
      sx = 1 + 0.5 * (1 - p) * (p < 1 ? 1 : 0);
      sy = 1 / sx;
      var w = M.wobble(lt, a, 3.4, 5, 1) * 0.16;
      for (var h = 0; h < 2; h++) w += M.wobble(lt, X.HITS[h] + 0.04 + 0.03 * j, 5, 7, 1) * 0.12;
      w += M.wobble(lt, X.LOAD + 0.02, 4, 6, 1) * 0.14; // perk up at the click
      sx *= 1 + w;
      sy *= 1 - w;
      sx *= 1 + 0.025 * Math.sin(lt * 5 + j);
      sy *= 1 - 0.025 * Math.sin(lt * 5 + j);
      // anticipation squat
      var an = M.prog(lt, D - 0.1, D);
      sy *= 1 - 0.18 * Math.sin(Math.PI * an);
      sx *= 1 + 0.14 * Math.sin(Math.PI * an);
    } else if (lt < L) {
      var u = M.prog(lt, D, L);
      var s = X.SLOT[j];
      var pos = function (uu) {
        return [M.lerp(q[0], s[0], uu), M.lerp(q[1], s[1], uu) - 150 * 4 * uu * (1 - uu)];
      };
      var a0 = pos(u), a1 = pos(Math.min(u + 0.02, 1));
      x = a0[0];
      y = a0[1];
      rot = Math.atan2(a1[1] - a0[1], a1[0] - a0[0]);
      sx = 1 + 0.32 * Math.sin(Math.PI * u);
      sy = 1 / sx;
      r *= 1 - 0.1 * u;
    } else {
      p = M.prog(lt, L, L + 0.18);
      x = X.SLOT[j][0];
      y = X.SLOT[j][1];
      r *= 0.85 * (1 - M.smooth(p));
      sx = 1.45 + 0.4 * p;
      sy = 0.6 - 0.2 * p;
    }
    return { x: x, y: y, r: r, sx: sx, sy: sy, rot: rot };
  }
  X.pastel = pastel;

  /* The liquid fingertip: a glass drop that taps the UI. */
  X.tap = function (P, lt, T, x, y) {
    var t0 = T - 0.26;
    if (lt < t0 || lt > T + 0.44) return;
    P.layer(1);
    if (lt < T) {
      var p = M.prog(lt, t0, T);
      var a = M.outBack(M.prog(lt, t0, t0 + 0.12), 2);
      P.ball(x, y - 46 * (1 - p * p), 10 * a, C.white, { sx: 1 - 0.18 * p, sy: 1 + 0.32 * p, clear: 1, k: 12 });
    } else {
      var d = lt - T;
      var s = M.prog(d, 0, 0.2);
      P.ball(x, y + 3, 10 * (1 - M.inCubic(s)), C.white, { sx: 1.65 - 0.65 * s, sy: 0.55 + 0.45 * s, clear: 1, k: 12 });
      var rp = M.prog(d, 0, 0.44);
      var hw = 1.8 * (1 - rp);
      if (hw > 0.25) P.ring(x, y, 8 + 22 * M.outCubic(rp), hw, C.white, { k: 6, clear: 1 });
    }
    P.layer(0);
  };

  /* ---------- beads on a string ---------- */
  var BEAD = ["#F59A45", C.mail, "#F2B620", "#F59A45"];
  function beads(P, lt) {
    if (lt < 0.68 || lt > 3.8) return;
    var cs = X.cam(lt).s;
    var R = 11 * cs, SR = 2.8 * cs;
    var pos = [0, 1, 2, 3].map(function (i) { return X.bead(i, lt); });
    P.layer(1);
    for (var i = 0; i < 4; i++) {
      var T = X.T[i];
      var E = 3.47 + 0.06 * i; // pop time on exit
      var pop = lt < E ? 1 : lt < E + 0.07 ? 1 + 3 * (lt - E) : Math.max(0, 1.21 * (1 - (lt - E - 0.07) / 0.09));
      var br = 1 + 0.03 * Math.sin(lt * 4.4 + i * 1.3);
      if (i === 0) {
        var r0 = R * M.spring(lt, T - 0.06, 2.8, 0.36) * pop * br;
        P.ball(pos[0][0], pos[0][1], r0, BEAD[0], { k: 16 });
        continue;
      }
      var S = 3.42 + 0.04 * i; // string snaps
      var snap = 1 - M.prog(lt, S, S + 0.08);
      if (lt < T) {
        var p = M.inOutCubic(M.prog(lt, T - 0.32, T));
        if (p <= 0) continue;
        var tx = M.lerp(pos[i - 1][0], pos[i][0], p), ty = M.lerp(pos[i - 1][1], pos[i][1], p);
        P.cap(pos[i - 1][0], pos[i - 1][1], tx, ty, SR * 1.1, SR * 1.7, C.white, { k: 10 });
        P.ball(tx, ty, R * 0.5 * M.smooth(p * 3), BEAD[i], { sy: 1.25, sx: 0.85, k: 12 });
      } else {
        if (snap > 0.02) P.cap(pos[i - 1][0], pos[i - 1][1], pos[i][0], pos[i][1], SR * snap, SR * snap, C.white, { k: 10 });
        var rr = R * (0.5 + 0.5 * M.spring(lt, T, 3, 0.35)) * pop * br;
        var w = X.jelly(lt, T, 3.6, 6);
        P.ball(pos[i][0], pos[i][1], rr, BEAD[i], { sx: 1 + 0.25 * w, sy: 1 - 0.25 * w, k: 16 });
      }
      // a plip falls from each popped bead
      if (lt > E + 0.1 && lt < E + 0.5) {
        var fd = lt - E - 0.1;
        P.ball(pos[i][0], pos[i][1] + 600 * fd * fd + 30 * fd, 4 * cs * (1 - fd / 0.4), BEAD[i], { sy: 1.3, k: 6 });
      }
    }
    P.layer(0);
  }

  function news(P, lt) {
    P.layer(1);
    // tracking pixel
    var d = X.dark(lt);
    if (d) {
      if (lt < 5.4 && lt > 5.12) {
        var tp = M.prog(lt, 5.0, 5.4);
        P.cap(d.x + 18 + 60 * (1 - tp), d.y - 2, d.x + 8, d.y, 1.5, 6, "#2A2240", { k: 10 });
      }
      P.ball(d.x, d.y, d.r, "#1A1230", { sx: d.sx, sy: d.sy, rot: d.rot, k: 12 });
    }
    // the polite remote images
    for (var j = 0; j < 3; j++) {
      var p = pastel(j, lt);
      if (p) P.ball(p.x, p.y, p.r, X.PC[j], { sx: p.sx, sy: p.sy, rot: p.rot, k: 14 });
    }
    // Dark spit sprays where the pixel bonks the shield.
    for (var h = 0; h < 3; h++) {
      var ft = lt - X.HITS[h];
      if (ft <= 0 || ft > 0.45) continue;
      var rg = M.rng(77 + h);
      for (var q = 0; q < 3; q++) {
        var ang = -0.9 + 0.9 * q + (rg() - 0.5) * 0.4, sp = 260 + 160 * rg();
        var sx0 = X.SH.x + X.SH.w + 6, sy0 = X.CY[h];
        P.ball(sx0 + Math.cos(ang) * sp * ft, sy0 + Math.sin(ang) * sp * ft + 900 * ft * ft, 5.5 * (1 - ft / 0.45), "#2A2046", { k: 4 });
      }
    }
    P.layer(0);
  }

  LQ.add({
    id: "s4",
    start: 26,
    end: 34,
    build: function (lt, P) {
      X.tap(P, lt, 0.32, 720, 399); // row 2: Re: Lisbon offsite photos
      var r6 = X.sc(lt, 720, 795); // row 6: Design Weekly
      X.tap(P, lt, 3.75, r6[0], r6[1]);
      var b = X.sc(lt, X.BTN[0], X.BTN[1]);
      X.tap(P, lt, X.LOAD, b[0], b[1]);
      beads(P, lt);
      news(P, lt);
    },
  });
})();
