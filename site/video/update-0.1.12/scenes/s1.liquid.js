/* s1 intro, global 0–12 s: a white drop lands, blooms into "0.1.11", melts into
   "0.1.12", splits into six app tiles, the ElyTable drop plunges in and slurps
   them all, settling exactly on H1. Pure function of time.
   window.__s1 shares the same poses with scenes/s1.html (eyes, icons). */
(function () {
  "use strict";
  var M = LQ.M, C = LQ.C;
  var WHITE = "#F4F6FF";
  var FONT = '900 280px "Inter"';
  var FW = 1500, FH = 420, PAD = 40;
  var CANDY = [1.12, 1.14, 1.26];
  var SA = { text: "0.1.11", font: FONT, w: FW, h: FH, blur: 12, align: "left", pad: PAD };
  var SB = { text: "0.1.12", font: FONT, w: FW, h: FH, blur: 12, align: "left", pad: PAD };
  var TY = 470;      // visual centre of the version text
  var FLOOR = 620;   // first landing
  var DFLOOR = 790;  // where the drip lands
  var ROW = 470;     // tile row centre
  var PY = 640;      // melt puddle floor
  var APPS = ["doc", "show", "pdf", "mail", "tree", "mark"];
  var RANK = [2, 1, 0, 0, 1, 2];
  var TSNAP = 4.66, TLAND = 5.0;
  var HOPS = [{ t: 5.5, c: 5 }, { t: 6.0, c: 4 }, { t: 6.5, c: 2 }, { t: 7.0, c: 0 }];

  function rgb(h) { return M.hex(h).slice(0, 3); }
  var WRGB = rgb(WHITE);

  /* squash impulse: rises within 50 ms, then a damped ring */
  function imp(t, t0, freq, damp) {
    var x = t - t0;
    if (x <= 0) return 0;
    return M.smooth(x / 0.05) * Math.exp(-(damp || 7) * x) * Math.cos(2 * Math.PI * (freq || 2.6) * x);
  }

  var G = null;
  function geo() {
    if (G) return G;
    var ctx = document.createElement("canvas").getContext("2d");
    ctx.font = FONT;
    ctx.textBaseline = "middle";
    var w = function (s) { return ctx.measureText(s).width; };
    var m = ctx.measureText("0");
    var b = "0.1.12", ctr = [];
    for (var i = 0; i < b.length; i++) ctr.push(w(b.slice(0, i)) + w(b.charAt(i)) / 2);
    var g = { w9: w("0.1.11"), w10: w(b), asc: m.actualBoundingBoxAscent, desc: m.actualBoundingBoxDescent, ctr: ctr, c9: w("0.1.1") + w("1") / 2 };
    if (document.fonts && document.fonts.check(FONT)) G = g;
    return g;
  }

  /* text field pose: centre (cx, cy), scale s, glyph top and bottom */
  function pose(lt) {
    var g = geo();
    var s = 1 + M.wobble(lt, 2.6, 2.4, 4, 0.05);
    s *= 1 + M.wobble(lt, 3.95, 3.2, 6, 0.025) + M.wobble(lt, 5.0, 3.4, 4.2, 0.075);
    s *= 1 + 0.013 * Math.sin((lt - 5.5) * Math.PI) * M.smooth(M.prog(lt, 5.5, 6.0)) * (1 - M.smooth(M.prog(lt, 7.1, 7.4)));
    var sh = M.inOutCubic(M.prog(lt, 3.2, 3.95));
    var tc = PAD + M.lerp(g.w9, g.w10, sh) / 2;
    var cx = 960 - (tc - FW / 2) * s - M.wobble(lt, 3.95, 3.4, 6, 12);
    var cy = TY - ((g.desc - g.asc) / 2) * s + 26 * M.inOutCubic(M.prog(lt, 7.4, 7.76));
    return { cx: cx, cy: cy, s: s, top: cy - g.asc * s, bot: cy + g.desc * s, g: g };
  }
  function gx(p, ox) { return p.cx + (PAD + ox - FW / 2) * p.s; }

  /* ---------- 0.4–2.8: the first drop ---------- */
  function drop(lt, P) {
    if (lt < 0.42 || lt >= 2.8) return;
    var R = 44, sx, sy, y;
    if (lt < 1.0) {
      var T = 0.58, ya = FLOOR - R * 1.3, y0 = -130;
      var g = (2 * (ya - y0)) / (T * T), u = lt - (1.0 - T);
      y = y0 + 0.5 * g * u * u;
      var v = g * u;
      sy = 1 + 0.3 * Math.min(v / (g * T), 1);
      sx = 1 / sy;
      P.ball(960, y, R, WHITE, { sx: sx, sy: sy, k: 30 });
      P.cap(960, y - R * sy * 0.45, 960, y - R * sy * 0.45 - Math.min(0.05 * v, 120), R * sx * 0.58, 2.5, WHITE, { k: 30 });
      return;
    }
    var r = R - 7 * M.smooth(M.prog(lt, 1.0, 1.1)) + 7 * M.smooth(M.prog(lt, 1.55, 1.9));
    sy = 1 + 0.3 * (1 - M.smooth((lt - 1.0) / 0.05)) - 0.5 * imp(lt, 1.0, 2.6, 8) - 0.22 * imp(lt, 1.58, 3, 7);
    var hop = 0;
    if (lt > 1.2 && lt < 1.58) {
      var hu = (lt - 1.2) / 0.38;
      hop = 52 * 4 * hu * (1 - hu);
      sy += 0.12 * Math.abs(1 - 2 * hu);
    }
    if (lt < 2.12) {
      sy -= 0.3 * M.smooth(M.prog(lt, 1.88, 2.1));
      sy *= 1 + 0.012 * Math.sin((lt - 1.6) * 9) * M.prog(lt, 1.6, 1.8);
      P.ball(960, FLOOR - hop - r * sy, r, WHITE, { sx: 1 / sy, sy: sy, k: 40 });
      return;
    }
    var lu = M.prog(lt, 2.12, 2.42), e = 1 - (1 - lu) * (1 - lu);
    var yb = M.lerp(FLOOR - r * 1.5, TY, e);
    var st = 1 + 0.5 * (1 - lu);
    if (lu < 1) {
      P.ball(960, yb, r, WHITE, { sx: 1 / st, sy: st, k: 40 });
      P.cap(960, yb + r * st * 0.4, 960, yb + r * st * 0.4 + 90 * (1 - lu), (r / st) * 0.55, 2, WHITE, { k: 30 });
      return;
    }
    /* bloom: the drop stretches into a bar over the word, then thins away as the letters fatten */
    var e2 = M.outBack(M.prog(lt, 2.42, 2.56), 1.4), thin = 1 - M.smooth(M.prog(lt, 2.56, 2.76));
    var hw = (geo().w9 / 2 - 96) * e2, rr = M.lerp(r, 104, M.smooth(M.prog(lt, 2.42, 2.54))) * thin;
    if (rr > 0.5) P.cap(960 - hw, TY, 960 + hw, TY, rr, rr, WHITE, { k: 40 });
    var rnd = M.rng(3), x = lt - 2.42;
    for (var k = 0; k < 4 && x < 0.5; k++) {
      var s = k % 2 ? 1 : -1, vx = s * (260 + rnd() * 200), vy = -(520 + rnd() * 260);
      var rr = (5 + rnd() * 4) * M.smooth(x / 0.05) * (1 - M.smooth(M.prog(x, 0.3, 0.45)));
      P.ball(960 + s * 120 + vx * x, TY - 40 + vy * x + 1300 * x * x, rr, WHITE, { k: 10 });
    }
  }

  /* 1.0–1.9: the crown splash, droplets fly, land, slide home */
  function crown(lt, P) {
    if (lt < 1.0 || lt > 1.9) return;
    var rnd = M.rng(11), g = 2600;
    for (var k = 0; k < 7; k++) {
      var side = k - 3;
      var ang = -Math.PI / 2 + side * 0.37 + (rnd() - 0.5) * 0.12;
      var sp = k === 3 ? 640 : 400 + rnd() * 240;
      var rd = k === 3 ? 9 : 6 + rnd() * 6;
      var vx = Math.cos(ang) * sp, vy = Math.sin(ang) * sp;
      var x0 = 960 + side * 16, y0 = FLOOR - 22;
      var yl = FLOOR - rd * 0.7;
      var tf = (-vy + Math.sqrt(vy * vy + 2 * g * (yl - y0))) / g;
      var x = lt - 1.0;
      if (x < tf) {
        var vyy = vy + g * x, v = Math.sqrt(vx * vx + vyy * vyy), st = 1 + Math.min(v / 1400, 0.7);
        P.ball(x0 + vx * x, y0 + vy * x + 0.5 * g * x * x, rd * M.smooth(x / 0.05), WHITE, { sx: st, sy: 1 / st, rot: Math.atan2(vyy, vx), k: 14 });
      } else {
        var q = M.prog(lt, 1.0 + tf + 0.06, 1.88), e = M.inCubic(q);
        var sq = 1 - 0.38 * imp(lt, 1.0 + tf, 4, 9);
        var rr = rd * (1 - M.smooth(M.prog(q, 0.75, 1)));
        P.ball(M.lerp(x0 + vx * tf, 960, e), FLOOR - rr * sq, rr, WHITE, { sx: (1 / sq) * (1 + 0.5 * e), sy: sq, k: 14 + 30 * e });
      }
    }
  }

  /* ---------- 2.38–8.0: the liquid version number ---------- */
  function text(lt, P) {
    if (lt < 2.54 || lt >= 7.76) return;
    var p = pose(lt);
    var amt = (0.5 + 0.5 * M.smooth(M.prog(lt, 2.54, 2.74))) * (1 - 0.5 * M.smooth(M.prog(lt, 7.6, 7.74)));
    if (lt < 4.2) P.text(0, SA, p.cx, p.cy, { scale: p.s, amount: amt, color: CANDY });
    else if (lt < 4.9) P.morph(SA, SB, p.cx, p.cy, M.inOutCubic(M.prog(lt, 4.2, 4.9)), { scale: p.s, color: CANDY });
    else P.text(0, SB, p.cx, p.cy, { scale: p.s, amount: amt, color: CANDY });
  }

  /* 4.1–5.0: the last "1" swells into a blob and comes back a "2" */
  function cover(lt, P) {
    if (lt < 4.1 || lt > 5.0) return;
    var p = pose(lt), g = p.g;
    var x9 = gx(p, g.c9), x0 = gx(p, g.ctr[5]);
    var yc = (p.top + p.bot) / 2;
    var grow = M.outBack(M.prog(lt, 4.1, 4.4), 2.4);
    var split = M.inOutCubic(M.prog(lt, 4.45, 4.82));
    var fade = 1 - M.smooth(M.prog(lt, 4.74, 4.99));
    var r = 88 * p.s * grow * fade * (1 - 0.25 * split);
    var jig = M.wobble(lt, 4.3, 3.6, 5, 0.08);
    P.ball(M.lerp(x9, x0, split), yc, r, WHITE, { sx: 0.9 + jig, sy: 1.18 - jig, k: 30 });
  }

  function dripX(lt) {
    var p = pose(lt), g = p.g;
    return M.lerp(gx(p, g.c9), gx(p, g.ctr[5]), M.inOutCubic(M.prog(lt, 4.45, 4.82)));
  }
  var DR = 24;
  function dripSag(lt) {
    return 6 + 70 * M.inCubic(M.prog(lt, 3.7, TSNAP)) + M.wobble(lt, 4.3, 2.5, 3, 4);
  }
  /* 3.55–5.0: a drip grows under the 9, snaps, falls */
  function drip(lt, P) {
    if (lt < 3.55 || lt >= TLAND) return;
    var p, ax, dy;
    if (lt < TSNAP) {
      p = pose(lt);
      ax = dripX(lt);
      var dr = 4 + (DR - 4) * M.smooth(M.prog(lt, 3.55, 4.25));
      dy = p.bot + dripSag(lt) + dr * 0.4;
      var neck = dr * 0.62 * (1 - 0.82 * M.smooth(M.prog(lt, 4.15, TSNAP)));
      P.cap(ax, p.bot - 16, ax, dy, Math.max(neck * 1.35, 3), Math.max(neck, 2), WHITE, { k: 20 });
      P.ball(ax, dy, dr, WHITE, { sx: 0.95, sy: 1.1, k: 20 });
      return;
    }
    p = pose(TSNAP);
    ax = dripX(TSNAP);
    var y0 = p.bot + dripSag(TSNAP) + DR * 0.4;
    var T = TLAND - TSNAP, yl = DFLOOR - DR * 1.3;
    var g = (2 * (yl - y0)) / (T * T), u = lt - TSNAP;
    var y = y0 + 0.5 * g * u * u, v = g * u;
    var st = 1 + 0.3 * Math.min(v / (g * T), 1);
    P.ball(ax, y, DR, WHITE, { sx: 1 / st, sy: st, k: 20 });
    P.cap(ax, y - DR * 0.5, ax, y - DR * 0.5 - Math.min(0.06 * v, 70), DR * 0.5, 2, WHITE, { k: 20 });
    var su = u - 0.06;
    if (su > 0) {
      var sy = p.bot + 34 + 0.5 * g * 0.9 * su * su;
      if (sy < DFLOOR - 5) P.ball(ax + 2, sy, 5.5, WHITE, { k: 10 });
    }
    var pc = pose(lt);
    var stub = 34 * (1 - M.smooth(M.prog(lt, TSNAP, TSNAP + 0.16))) + M.wobble(lt, TSNAP + 0.16, 4, 9, 6);
    if (stub > 1) P.cap(dripX(lt), pc.bot - 16, dripX(lt), pc.bot + stub, 7, 4, WHITE, { k: 20 });
  }

  /* 5.0: splash, ring shockwave */
  function impact(lt, P) {
    if (lt < 5.0 || lt > 6.0) return;
    var x = lt - 5.0;
    var ax = dripX(TSNAP), g = 2600;
    if (x < 0.5) {
      var rnd = M.rng(5);
      for (var k = 0; k < 4; k++) {
        var s = k < 2 ? -1 : 1;
        var ang = -Math.PI / 2 + s * (0.55 + (k % 2) * 0.45 + rnd() * 0.1);
        var sp = 300 + rnd() * 160;
        var vx = Math.cos(ang) * sp, vy = Math.sin(ang) * sp;
        var py = DFLOOR - 8 + vy * x + 0.5 * g * x * x;
        if (py < DFLOOR - 3) P.ball(ax + vx * x, py, (4 + rnd() * 2.5) * M.smooth(x / 0.04), WHITE, { k: 8 });
      }
    }
    var pr = M.prog(lt, 5.0, 5.9);
    if (pr < 0.98) {
      var R = 400 + 760 * M.outCubic(pr);
      P.ring(960, TY, R, (8 * Math.pow(1 - pr, 1.5) + 0.7) * M.smooth(x / 0.05), WHITE, { k: 12 });
    }
  }

  function perch(lt, c, r) {
    var p = pose(lt);
    return { x: gx(p, p.g.ctr[c]), y: p.top - r * 0.72 };
  }
  /* 5.0–7.8: the drip becomes a stray droplet that hops along the digits */
  function stray(lt, P) {
    if (lt < TLAND || lt > 7.8) return;
    var r = 24;
    var prevT = TLAND, prev = { x: dripX(TSNAP), y: DFLOOR - r }, next = -1;
    for (var i = 0; i < HOPS.length; i++) {
      if (lt >= HOPS[i].t) { prevT = HOPS[i].t; prev = perch(lt, HOPS[i].c, r); }
      else if (next < 0) next = i;
    }
    var launch = prevT + 0.13;
    if (next >= 0 && lt > launch) {
      var tgt = perch(lt, HOPS[next].c, r);
      var dur = HOPS[next].t - launch, u = (lt - launch) / dur;
      var h = next === 0 ? 120 : 95;
      var x = M.lerp(prev.x, tgt.x, u), y = M.lerp(prev.y, tgt.y, u) - h * 4 * u * (1 - u);
      var vx = (tgt.x - prev.x) / dur, vy = (tgt.y - prev.y) / dur - (h * 4 * (1 - 2 * u)) / dur;
      var st = 1 + Math.min(Math.sqrt(vx * vx + vy * vy) / 1800, 0.45);
      P.ball(x, y, r, WHITE, { sx: st, sy: 1 / st, rot: Math.atan2(vy, vx), k: 22 });
      return;
    }
    var sq = 1 - 0.45 * imp(lt, prevT, 3.4, 8);
    if (next < 0) sq *= 1 + 0.03 * Math.sin((lt - 7.0) * 7);
    var rr = r * (1 - M.smooth(M.prog(lt, 7.4, 7.75)));
    P.ball(prev.x, prev.y + r * (1 - sq), rr, WHITE, { sx: 1 / sq, sy: sq, k: 22 });
  }

  /* ---------- 7.4–12: puddle → six tiles → ElyTable drop → H1 ---------- */
  function slot6(i) { return 960 + (i - 2.5) * 200; }
  function puddleX(i) { return 960 + (i - 2.5) * 84; }
  function shove(i, lt) { return (i < 3 ? -1 : 1) * 100 * M.spring(lt, 9.46 + RANK[i] * 0.035, 2.6, 0.4); }
  function tq(i) { return 8.5 + i * 0.045; }
  function tA(i) { return 8.06 + RANK[i] * 0.06; }
  function tS(i) { return 10.3 + RANK[i] * 0.13; }
  function tilePose(i, lt) {
    var w = M.wobble(lt, 9.46 + RANK[i] * 0.035, 3.2, 5.5, 1);
    return { x: slot6(i) + shove(i, lt), y: ROW, sx: 1 - 0.12 * w, sy: 1 + 0.1 * w };
  }

  /* 7.4–8.06: drips sag, a bar swallows the word, then slumps into the puddle */
  function melt(lt, P) {
    if (lt < 7.4 || lt > 8.08) return;
    var p = pose(Math.min(lt, 7.76)), g = p.g;
    var dg = M.smooth(M.prog(lt, 7.42, 7.72)) * (1 - M.smooth(M.prog(lt, 7.7, 7.8)));
    var idx = [0, 2, 4, 5];
    for (var k = 0; k < idx.length && dg > 0.02; k++) {
      var x = gx(p, g.ctr[idx[k]]), len = 66 * dg + M.wobble(lt, 7.5 + k * 0.05, 3, 4, 5);
      P.cap(x, p.bot - 18, x, p.bot + len, 10 + 6 * dg, 14 * dg + 2, WHITE, { k: 24 });
      P.ball(x, p.bot + len + 4, 16 * dg, WHITE, { sy: 1.15, k: 24 });
    }
    var cov = M.outCubic(M.prog(lt, 7.56, 7.74));
    var slump = M.inOutCubic(M.prog(lt, 7.76, 8.0));
    var gone = M.smooth(M.prog(lt, 7.97, 8.06));
    var yc = (p.top + p.bot) / 2;
    var hw = M.lerp(g.w10 / 2 - 96, 300, slump);
    var r = M.lerp(108 * cov, 31, slump) * (1 - gone);
    var y = M.lerp(yc, PY - 31, slump) + M.wobble(lt, 8.0, 3, 6, 6);
    if (r > 0.5) P.cap(960 - hw, y, 960 + hw, y, r, r, WHITE, { k: 40 });
  }

  function tiles(lt, P) {
    if (lt < 7.9 || lt > 10.9) return;
    for (var i = 0; i < 6; i++) {
      var app = rgb(C[APPS[i]]);
      var a = tA(i), b = a + 0.28, q = tq(i), s0 = tS(i);
      if (lt < a) {
        var r0 = 50 * M.smooth(M.prog(lt, 7.9, 8.04));
        var pc = M.mix3(WRGB, app, M.smooth(M.prog(lt, 7.94, 8.12)));
        var ant = 0.62 - 0.14 * M.smooth(M.prog(lt, a - 0.08, a));
        P.ball(puddleX(i), PY - r0 * ant, r0, pc, { sx: 1 / ant * 0.95, sy: ant, k: 44 });
      } else if (lt < b) {
        var u = (lt - a) / 0.28, r = M.lerp(44, 66, M.smooth(u));
        var fx = puddleX(i), fy = PY - 50 * 0.48, h = 120;
        var x = M.lerp(fx, slot6(i), u), y = M.lerp(fy, ROW, u) - h * 4 * u * (1 - u);
        var vx = (slot6(i) - fx) / 0.28, vy = (ROW - fy) / 0.28 - (h * 4 * (1 - 2 * u)) / 0.28;
        var st = 1 + Math.min(Math.sqrt(vx * vx + vy * vy) / 5000, 0.22);
        P.ball(x, y, r, app, { sx: st, sy: 1 / st, rot: Math.atan2(vy, vx), k: 44 });
      } else if (lt < q) {
        var sq = 1 - 0.36 * imp(lt, b, 3, 7) * (1 - M.prog(lt, q - 0.1, q));
        P.ball(slot6(i), ROW + 66 - 66 * sq, 66, app, { sx: 1 / sq, sy: sq, k: 40 });
      } else if (lt < s0) {
        var tp = tilePose(i, lt);
        var back = M.smooth(M.prog(lt, 10.12, 10.3));
        var rad = M.lerp(66 - 30 * M.spring(lt, q, 2.8, 0.45), 66, back);
        var tint = M.mix3(app, WRGB, 0.74 * M.smooth(M.prog(lt, q, q + 0.22)) * (1 - back));
        P.box(tp.x, tp.y, 132 * tp.sx, 132 * tp.sy, rad, tint, { k: 40 });
      } else {
        var su = M.inCubic(M.prog(lt, s0, s0 + 0.3));
        var mp = heroPose(lt);
        var tx = slot6(i) + shove(i, lt);
        var sxp = M.lerp(tx, mp.x, su), syp = M.lerp(ROW, mp.y, su);
        var rr = 66 * (1 - 0.45 * su) * (1 - M.smooth(M.prog(su, 0.7, 1)));
        var str = 1 + 0.7 * Math.sin(Math.PI * su);
        P.ball(sxp, syp, rr, app, { sx: str, sy: 1 / str, rot: Math.atan2(mp.y - syp, mp.x - sxp), k: 52 });
        if (rr > 1) P.cap(sxp, syp, mp.x, mp.y, rr * 0.5 * Math.min(1, su * 4 + 0.3), mp.r * 0.4, app, { k: 40 });
      }
    }
  }

  function heroPose(lt) {
    var R0 = 84;
    if (lt < 9.5) {
      var T = 0.32, ya = ROW + R0 - R0 * 1.55, y0 = -200;
      var g = (2 * (ya - y0)) / (T * T), u = Math.max(lt - (9.5 - T), 0);
      var st = 1 + 0.55 * Math.min((g * u) / (g * T), 1);
      return { x: 960, y: y0 + 0.5 * g * u * u, r: R0, sx: 1 / st, sy: st, v: g * u };
    }
    var r = R0 + 36 * M.smooth(M.prog(lt, 10.3, 10.9));
    var yc = M.lerp(ROW, 500, M.inOutCubic(M.prog(lt, 10.3, 10.92)));
    var env = 1 - M.smooth(M.prog(lt, 10.78, 10.95));
    var sy = 1 + 0.55 * (1 - M.smooth((lt - 9.5) / 0.045)) - 0.58 * imp(lt, 9.5, 2.3, 5.5);
    for (var k = 0; k < 3; k++) sy -= M.wobble(lt, 10.6 + k * 0.13, 3.6, 7, 0.07) * env;
    sy += 0.016 * Math.sin((lt - 9.9) * 5.6) * M.smooth(M.prog(lt, 9.9, 10.1)) * env;
    return { x: 960, y: yc + r - r * sy, r: r, sx: 1 / sy, sy: sy, v: 0 };
  }

  function hero(lt, P) {
    if (lt < 9.18) return;
    var m = heroPose(lt);
    P.ball(m.x, m.y, m.r, C.table, { sx: m.sx, sy: m.sy, k: 36 });
    if (lt < 9.5) P.cap(960, m.y - m.r * m.sy * 0.45, 960, m.y - m.r * m.sy * 0.45 - Math.min(0.03 * m.v, 150), m.r * m.sx * 0.6, 3, C.table, { k: 30 });
    var x = lt - 9.5;
    if (x > 0 && x < 0.5) {
      var rnd = M.rng(23), g = 3000;
      for (var k = 0; k < 4; k++) {
        var s = k < 2 ? -1 : 1;
        var vx = s * (120 + rnd() * 160), vy = -(480 + rnd() * 200);
        var r = (6 + rnd() * 5) * M.smooth(x / 0.05) * (1 - M.smooth(M.prog(x, 0.3, 0.46)));
        P.ball(960 + s * 40 + vx * x, ROW - 40 + vy * x + 0.5 * g * x * x, r, C.table, { k: 12 });
      }
    }
  }

  LQ.add({
    id: "s1",
    start: 0,
    end: 12,
    build: function (lt, P) {
      if (lt >= 10.95) {
        /* hold on H1; squat before the 12.0 drop, exact at the end */
        var sq = 1 - 0.12 * Math.sin(Math.PI * M.prog(lt, 11.4, 11.98));
        P.ball(960, 500 + 120 * (1 - sq), 120, C.table, { sx: 1 / sq, sy: sq });
        return;
      }
      drop(lt, P);
      crown(lt, P);
      text(lt, P);
      cover(lt, P);
      drip(lt, P);
      impact(lt, P);
      stray(lt, P);
      melt(lt, P);
      tiles(lt, P);
      hero(lt, P);
    },
  });

  window.__s1 = { pose: pose, gx: gx, geo: geo, tile: tilePose, hero: heroPose, tq: tq, ROW: ROW, TY: TY };
})();
