/* s7 outro, global 48–60 s: from H6 the seven balls wave, fling together into
   one marbled drop (ring shockwave on 50.0), which seeds seven icon drops and
   blooms into "Ely Office" / "0.1.12". A last drip falls, hops onto the mail
   icon and winks; fade to black. window.__s7 shares poses with scenes/s7.html. */
(function () {
  "use strict";
  var M = LQ.M, C = LQ.C;
  var WHITE = "#F4F6FF";
  var ORDER = ["doc", "table", "show", "mail", "pdf", "tree", "mark"];
  var TFONT = '900 200px "Inter"', VFONT = '900 124px "Inter"', VLS = 10;
  var FW = 1600, FH = 420;
  var ST = { text: "Ely Office", font: TFONT, w: FW, h: FH, blur: 12 };
  var SV = { text: "0.1.12", font: VFONT, w: FW, h: FH, blur: 12, ls: VLS };
  var TY = 290, VY = 458, IY = 668, ISP = 156, ISZ = 136;
  var CANDY = [1.12, 1.14, 1.26], VCOL = [0.62, 0.9, 1.38];
  var DR = 40;

  function imp(t, t0, freq, damp) {
    var x = t - t0;
    if (x <= 0) return 0;
    return M.smooth(x / 0.05) * Math.exp(-(damp || 7) * x) * Math.cos(2 * Math.PI * (freq || 2.6) * x);
  }
  function hop(lt, t0, d, h) {
    if (lt <= t0 || lt >= t0 + d) return 0;
    var u = (lt - t0) / d;
    return h * 4 * u * (1 - u);
  }
  function bx(i) { return 960 + (i - 3) * 200; }
  function ix(i) { return 960 + (i - 3) * ISP; }

  var G = null;
  function geo() {
    if (G) return G;
    var ctx = document.createElement("canvas").getContext("2d");
    ctx.textBaseline = "middle";
    function chars(font, str, ls) {
      ctx.font = font;
      ctx.letterSpacing = ls + "px";
      var tw = ctx.measureText(str).width, c = [];
      for (var i = 0; i < str.length; i++) c.push(ctx.measureText(str.slice(0, i)).width + (ctx.measureText(str.charAt(i)).width - ls) / 2 - tw / 2);
      ctx.letterSpacing = "0px";
      return c;
    }
    var g = { tc: chars(TFONT, ST.text, 0), vc: chars(VFONT, SV.text, VLS) };
    ctx.font = TFONT;
    var mE = ctx.measureText("E");
    ctx.font = VFONT;
    var m0 = ctx.measureText("0");
    g.ta = mE.actualBoundingBoxAscent;
    g.td = mE.actualBoundingBoxDescent;
    g.va = m0.actualBoundingBoxAscent;
    g.vd = m0.actualBoundingBoxDescent;
    if (document.fonts && document.fonts.check(TFONT) && document.fonts.check(VFONT)) G = g;
    return g;
  }
  function tPose(lt) {
    var g = geo();
    var s = 1 + M.wobble(lt, 2.46, 2.4, 4, 0.05);
    s *= 1 + 0.012 * Math.sin((lt - 3.4) * Math.PI) * M.smooth(M.prog(lt, 3.4, 4.0));
    var tcy = TY - ((g.td - g.ta) / 2) * s, vcy = VY - ((g.vd - g.va) / 2) * s;
    return { s: s, tcy: tcy, vcy: vcy, tbot: tcy + g.td * s, vbot: vcy + g.vd * s, g: g };
  }
  function bob(i, lt) { return 4 * Math.sin((lt - 3.2) * 2.4 + i * 0.85) * M.smooth(M.prog(lt, 3.2, 3.8)); }
  function iconTop(i, lt) { return IY - ISZ / 2 + ISZ * (25 / 256) + bob(i, lt); }

  /* marble: the seven drops orbiting inside one blob */
  function marble(i, lt) {
    var ang = (i / 7) * Math.PI * 2 + 1.8 * (lt - 1.6);
    var rad = 78 - 20 * M.smooth(M.prog(lt, 1.8, 2.1));
    var w = M.wobble(lt, 1.8, 2.2, 3.4, 0.22);
    return { x: 960 + rad * Math.cos(ang) * (1 + w), y: 470 + rad * Math.sin(ang) * (1 - w), w: w };
  }
  function leaveT(i) { return 2.05 + Math.abs(i - 3) * 0.035 + (i > 3 ? 0.015 : 0); }
  function iconT(i) { return leaveT(i) + 0.36; }

  /* 0–2.7: wave, fling, merge, seed the icon drops */
  function balls(lt, P) {
    if (lt >= 2.9) return;
    for (var i = 0; i < 7; i++) {
      var col = C[ORDER[i]];
      if (lt < 1.36) {
        var w1 = 0.1 + i * 0.06, w2 = 0.66 + Math.abs(i - 3) * 0.07, d1 = 0.32, d2 = 0.34;
        var h2 = 112 - Math.abs(i - 3) * 12;
        var sy = 1 - 0.16 * Math.sin(Math.PI * M.prog(lt, w1 - 0.09, w1)) - 0.18 * Math.sin(Math.PI * M.prog(lt, w2 - 0.09, w2));
        if (lt > w1 && lt < w1 + d1) sy += 0.14 * Math.abs(1 - (2 * (lt - w1)) / d1);
        if (lt > w2 && lt < w2 + d2) sy += 0.16 * Math.abs(1 - (2 * (lt - w2)) / d2);
        sy -= 0.3 * imp(lt, w1 + d1, 3.2, 8) + 0.34 * imp(lt, w2 + d2, 3.2, 8);
        sy -= 0.24 * M.smooth(M.prog(lt, 1.22, 1.36));
        var y = 500 - hop(lt, w1, d1, 70) - hop(lt, w2, d2, h2) + 62 * (1 - sy);
        P.ball(bx(i), y, 62, col, { sx: 1 / sy, sy: sy });
        continue;
      }
      var lv = leaveT(i);
      if (lt < lv) {
        var e = M.inCubic(M.prog(lt, 1.36, 1.8));
        var mp = marble(i, lt);
        var x = M.lerp(bx(i), mp.x, e), yy = M.lerp(500, mp.y, e);
        var r = 62 + 10 * M.smooth(M.prog(lt, 1.45, 1.9));
        var rel = 0.24 * (1 - M.smooth(M.prog(lt, 1.36, 1.46)));
        var st = 1 + 0.55 * Math.sin(Math.PI * e) * (Math.abs(i - 3) / 3);
        var k = 40 + 22 * M.smooth(M.prog(lt, 1.3, 1.75));
        P.ball(x, yy + 62 * rel, r, col, { sx: st * (1 + mp.w * e), sy: ((1 - rel) / st) * (1 - mp.w * e), k: k });
        continue;
      }
      var land = iconT(i);
      var from = marble(i, lv), to = { x: ix(i), y: IY };
      if (lt < land) {
        var u = (lt - lv) / 0.36, h = 70;
        var fx = M.lerp(from.x, to.x, u), fy = M.lerp(from.y, to.y, u) - h * 4 * u * (1 - u);
        var vx = (to.x - from.x) / 0.36, vy = (to.y - from.y) / 0.36 - (h * 4 * (1 - 2 * u)) / 0.36;
        var s2 = 1 + Math.min(Math.sqrt(vx * vx + vy * vy) / 4000, 0.3);
        P.ball(fx, fy, M.lerp(72, 30, M.smooth(u)), col, { sx: s2, sy: 1 / s2, rot: Math.atan2(vy, vx), k: 40 + 30 * (1 - u) });
      } else {
        var sq = 1 - 0.4 * imp(lt, land, 3, 7);
        var rr = 30 * (1 - M.smooth(M.prog(lt, land + 0.05, land + 0.3)));
        P.ball(to.x, to.y, rr, col, { sx: 1 / sq, sy: sq, k: 20 });
      }
    }
    /* strings between neighbours as they fling */
    if (lt > 1.3 && lt < 1.86) {
      var ee = M.inCubic(M.prog(lt, 1.36, 1.8));
      var rad2 = (4 + 26 * ee) * M.smooth(M.prog(lt, 1.3, 1.42));
      for (var j = 0; j < 6; j++) {
        var a = marble(j, lt), b = marble(j + 1, lt);
        P.cap(M.lerp(bx(j), a.x, ee), M.lerp(500, a.y, ee), M.lerp(bx(j + 1), b.x, ee), M.lerp(500, b.y, ee), rad2, rad2, C[ORDER[j]], { k: 30 });
      }
    }
  }

  /* 1.66–2.9: the white core splits; each half stretches into a bar that
     covers its word, then thins away as the letters fatten */
  function core(lt, P) {
    if (lt < 1.66 || lt > 2.9) return;
    var rc = 128 * M.smooth(M.prog(lt, 1.66, 1.98));
    var sp = M.spring(lt, 2.2, 2.2, 0.5);
    var ty = M.lerp(470, TY, sp), vy = M.lerp(470, VY, sp);
    var jw = M.wobble(lt, 2.2, 3.2, 5, 0.18);
    var mw = M.wobble(lt, 1.8, 2.2, 3.4, 0.22) * (1 - M.prog(lt, 2.2, 2.3));
    var neck = rc * 0.5 * (1 - M.smooth(M.prog(lt, 2.26, 2.44)));
    if (neck > 2) P.cap(960, ty, 960, vy, neck, neck, WHITE, { k: 40 });
    bar(P, lt, 960, ty, rc * 0.85, 455, 84, 2.3, 2.44, 2.62, WHITE, 1.25 + jw + mw, 1 - jw - mw);
    bar(P, lt, 960, vy, rc * 0.62, 205, 52, 2.4, 2.56, 2.76, VCOL, 1.1 - jw, 1 + jw);
  }
  function bar(P, lt, x, y, r0, half, rb, t0, t1, t2, col, sx, sy) {
    if (lt < t0) { P.ball(x, y, r0, col, { sx: sx, sy: sy, k: 60 }); return; }
    var e = M.outCubic(M.prog(lt, t0, t1)), thin = 1 - M.smooth(M.prog(lt, t1, t2));
    var hw = half * e, r = M.lerp(r0, rb, e) * thin;
    if (r > 0.5) P.cap(x - hw, y, x + hw, y, r, r, col, { k: 40 });
  }

  function shock(lt, P) {
    var pr = M.prog(lt, 1.95, 2.95);
    if (pr <= 0 || pr >= 0.98) return;
    P.ring(960, 470, 190 + 980 * M.outCubic(pr), (9 * Math.pow(1 - pr, 1.5) + 0.7) * M.smooth((lt - 1.95) / 0.05), WHITE, { k: 12 });
  }

  /* letters appear as a thin solid core and fatten: no ghost halo */
  function pop(lt, t0, d) { return lt < t0 ? 0 : 0.5 + 0.5 * M.smooth((lt - t0) / d); }
  function titles(lt, P) {
    if (lt < 2.42) return;
    var p = tPose(lt);
    P.text(0, ST, 960, p.tcy, { scale: p.s, amount: pop(lt, 2.42, 0.2), color: CANDY });
    if (lt > 2.54) P.text(1, SV, 960, p.vcy, { scale: p.s, amount: pop(lt, 2.54, 0.2), color: VCOL });
  }

  /* 5.6–8.2: drips that sag from the title and spring back */
  function yoyo(lt, P, c, t0) {
    var u = M.prog(lt, t0, t0 + 1.2);
    if (u <= 0 || u >= 1) return;
    var p = tPose(lt), x = 960 + p.g.tc[c] * p.s;
    var grow = M.smooth(M.prog(u, 0, 0.25)) * (1 - M.smooth(M.prog(u, 0.7, 1)));
    var sag = 6 + 48 * Math.sin(Math.PI * M.prog(u, 0.08, 0.72)) + M.wobble(lt, t0 + 0.86, 3, 6, 6);
    var dr = 15 * grow;
    if (dr < 0.6) return;
    P.cap(x, p.tbot - 14, x, p.tbot + sag, dr * 0.7, dr * 0.45, WHITE, { k: 20 });
    P.ball(x, p.tbot + sag + dr * 0.3, dr, WHITE, { sx: 0.95, sy: 1.08, k: 20 });
  }

  /* 9.0–12: the last drip: hangs from the final "0", falls onto ElyPDF, hops onto ElyMail */
  var TSN = 9.76, TL1 = 10.0, TH = 10.14, TL2 = 10.5;
  function dropPose(lt) {
    var p, x, y;
    if (lt < TSN) {
      p = tPose(lt);
      x = 960 + p.g.vc[5] * p.s;
      var dr = 4 + (DR - 4) * M.smooth(M.prog(lt, 9.0, 9.5));
      var sag = 6 + 40 * M.inCubic(M.prog(lt, 9.1, TSN)) + M.wobble(lt, 9.4, 2.6, 3, 3);
      return { x: x, y: p.vbot + sag + dr * 0.4, r: dr, sx: 0.95, sy: 1.08, hang: true, top: p.vbot - 12, neck: dr * 0.62 * (1 - 0.82 * M.smooth(M.prog(lt, 9.4, TSN))) };
    }
    var p0 = tPose(TSN), x0 = 960 + p0.g.vc[5] * p0.s;
    var y0 = p0.vbot + 46 + DR * 0.4;
    if (lt < TL1) {
      var T = TL1 - TSN, yl = iconTop(4, TL1) - DR * 1.2;
      var g = (2 * (yl - y0)) / (T * T), u = lt - TSN;
      var st = 1 + 0.2 * Math.min(u / T, 1);
      return { x: M.lerp(x0, ix(4), M.smooth(u / T)), y: y0 + 0.5 * g * u * u, r: DR, sx: 1 / st, sy: st };
    }
    if (lt < TH) {
      var sq = 1 + 0.2 * (1 - M.smooth((lt - TL1) / 0.04)) - 0.45 * imp(lt, TL1, 3.4, 8);
      return { x: ix(4), y: iconTop(4, lt) - DR * sq, r: DR, sx: 1 / sq, sy: sq };
    }
    if (lt < TL2) {
      var hu = (lt - TH) / (TL2 - TH), hh = 90;
      var ya = iconTop(4, lt) - DR, yb = iconTop(3, lt) - DR;
      var s3 = 1 + 0.18 * Math.abs(1 - 2 * hu);
      return { x: M.lerp(ix(4), ix(3), hu), y: M.lerp(ya, yb, hu) - hh * 4 * hu * (1 - hu), r: DR, sx: 1 / s3, sy: s3, rot: 0 };
    }
    var sq2 = 1 + 0.18 * (1 - M.smooth((lt - TL2) / 0.04)) - 0.5 * imp(lt, TL2, 3, 6.5);
    sq2 += 0.018 * Math.sin((lt - 10.9) * 5) * M.smooth(M.prog(lt, 10.9, 11.2));
    return { x: ix(3), y: iconTop(3, lt) - DR * sq2 + 3, r: DR, sx: 1 / sq2, sy: sq2 };
  }
  function lastDrop(lt, P) {
    if (lt < 9.0) return;
    var d = dropPose(lt);
    if (d.hang) {
      P.cap(d.x, d.top, d.x, d.y, Math.max(d.neck * 1.35, 3), Math.max(d.neck, 2), VCOL, { k: 20 });
      P.ball(d.x, d.y, d.r, VCOL, { sx: d.sx, sy: d.sy, k: 20 });
      return;
    }
    P.ball(d.x, d.y, d.r, VCOL, { sx: d.sx, sy: d.sy, k: 20 });
    var p = tPose(lt), x0 = 960 + p.g.vc[5] * p.s;
    var stub = 26 * (1 - M.smooth(M.prog(lt, TSN, TSN + 0.16))) + M.wobble(lt, TSN + 0.16, 4, 9, 5);
    if (stub > 1) P.cap(x0, p.vbot - 12, x0, p.vbot + stub, 7, 4, VCOL, { k: 20 });
  }

  LQ.add({
    id: "s7",
    start: 48,
    end: 60,
    build: function (lt, P) {
      balls(lt, P);
      core(lt, P);
      shock(lt, P);
      titles(lt, P);
      yoyo(lt, P, 0, 5.6);
      yoyo(lt, P, 9, 7.1);
      lastDrop(lt, P);
      P.fade = 1 - M.smooth(M.prog(lt, 11.4, 12.0));
    },
  });

  window.__s7 = { iconT: iconT, bob: bob, drop: dropPose, ix: ix, IY: IY, ISZ: ISZ };
})();
