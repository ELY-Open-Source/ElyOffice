/* s2: ElyMail reveal, global 11s to 18s.
   H1 ball -> crouch -> (12.0) jelly envelope + title jet -> flap opens, title gulped ->
   letter stretches out and snaps free -> grows into the window rect -> drains away under the glass. */
(function () {
  var M = LQ.M, C = LQ.C;
  function hx(c) { return M.hex(c).slice(0, 3); }
  var WHITE = hx(C.white), SKY = hx("#3E9FEA");
  var TITLE = { text: "ElyMail", font: '900 196px "Inter"', w: 1000, h: 320, blur: 12 };
  var FLOOR = 695, ENV_W = 400, ENV_H = 270, WIN = { cx: 960, cy: 494, w: 1520, h: 780, r: 30 };
  var LETTER0 = { cy: 258, w: 320, h: 250 };

  /* Phase A: the H1 ball crouches, hops, then squashes hard (bottom pinned at y 620). */
  function ballA(lt) {
    var hx_ = M.prog(lt, 0.18, 0.6);
    var hop = lt > 0.18 && lt < 0.6 ? 52 * 4 * hx_ * (1 - hx_) : 0;
    var dip = lt < 0.18 ? Math.sin((Math.PI * lt) / 0.18) : 0;
    var land = M.wobble(lt, 0.6, 3.2, 6, 1);
    var deep = M.smooth(M.prog(lt, 0.7, 1.0));
    var air = lt > 0.18 && lt < 0.6 ? 0.1 * (1 - 2 * hx_) : 0;
    var sy = 1 - 0.09 * dip - 0.12 * land - 0.28 * deep + air;
    var sx = 1 / Math.sqrt(sy * sy * 0.6 + 0.4 * sy);
    var shiver = 1.6 * Math.sin(lt * 92) * deep * (1 - M.prog(lt, 0.94, 1));
    return { x: 960 + shiver, y: 620 - 120 * sy - hop, sx: sx, sy: sy };
  }

  /* Phase B: the envelope body (centre, size, radius). */
  function envB(lt) {
    var a = ballA(1);
    var w0 = 240 * a.sx, h0 = 240 * a.sy;
    var e = M.spring(lt, 1, 2.3, 0.36);
    var w = M.lerp(w0, ENV_W, e), h = M.lerp(h0, ENV_H, e);
    var x = M.prog(lt, 1, 1.5), airb = lt > 1 && lt < 1.5;
    var arc = airb ? 4 * x * (1 - x) : 0;
    var st = airb ? 0.12 * (1 - 2 * x) * M.smooth(M.prog(lt, 1, 1.06)) : 0;
    h *= 1 + st;
    w *= 1 - st;
    var wb = M.wobble(lt, 1.5, 3.3, 5.5, 1);
    w *= 1 + 0.1 * wb;
    h *= 1 - 0.1 * wb;
    var gl = M.wobble(lt, 2.74, 3.6, 5, 1);
    h *= 1 + 0.07 * gl;
    w *= 1 - 0.05 * gl;
    var pull = Math.sin(Math.PI * M.prog(lt, 3.18, 3.45));
    h *= 1 + 0.05 * pull;
    w *= 1 - 0.03 * pull;
    var br = lt > 1.5 ? Math.sin((lt - 1.5) * 2 * Math.PI * 1.1) * 0.012 * M.prog(lt, 1.5, 2) : 0;
    w *= 1 + br;
    h *= 1 - br;
    var floor = M.lerp(620, FLOOR, M.smooth(M.prog(lt, 1, 1.3)));
    var cy = floor - h / 2 - 120 * arc;
    var q = 1 - M.smooth(M.prog(lt, 3.45, 3.95));
    cy = M.lerp(WIN.cy, cy, q);
    w *= q;
    h *= q;
    var r = Math.min(M.lerp(h0 / 2, 36, M.clamp(e, 0, 1)), h / 2, w / 2);
    return { cx: 960, cy: cy, w: w, h: h, r: r };
  }

  /* The letter rect: rises out, stretches, snaps free, then grows into the window. */
  function letter(lt, top) {
    if (lt < 2.85) return null;
    if (lt < 3.45) {
      var hL = 250 * M.outBack(M.prog(lt, 2.85, 3.3), 1.3);
      var sp = Math.sin(Math.PI * M.prog(lt, 2.85, 3.25));
      var yb = M.lerp(top + 6, 383, M.inCubic(M.prog(lt, 3.18, 3.45)));
      var lw = LETTER0.w * (1 - 0.1 * sp), lh = hL * (1 + 0.08 * sp);
      return { cx: 960, cy: yb - lh / 2, w: lw, h: lh, r: Math.min(22, lh / 2), yb: yb, c: WHITE };
    }
    var gw = M.spring(lt, 3.48, 1.55, 0.68), gh = M.spring(lt, 3.55, 1.55, 0.68);
    var jw = M.wobble(lt, 3.45, 3.4, 6, 1) * (1 - gw);
    var w = M.lerp(LETTER0.w, WIN.w, gw) * (1 + 0.08 * jw);
    var h = M.lerp(LETTER0.h, WIN.h, gh) * (1 - 0.08 * jw);
    var cy = M.lerp(LETTER0.cy, WIN.cy, gh);
    var r = M.lerp(22, WIN.r, M.clamp(gw, 0, 1));
    var pin = M.smooth(M.prog(lt, 3.9, 4.05));
    w = M.lerp(w, WIN.w, pin);
    h = M.lerp(h, WIN.h, pin);
    cy = M.lerp(cy, WIN.cy, pin);
    r = M.lerp(r, WIN.r, pin);
    return { cx: 960, cy: cy, w: w, h: h, r: r, c: M.mix3(WHITE, SKY, M.smooth(M.prog(lt, 3.5, 4.05))) };
  }

  LQ.add({
    id: "s2",
    start: 11,
    end: 18,
    build: function (lt, P) {
      /* ---- 11.0-12.0: anticipation ---- */
      if (lt < 1.1) {
        var a = ballA(Math.min(lt, 1));
        var rr = lt < 1 ? 120 : 120 * (1 - M.prog(lt, 1, 1.1));
        P.ball(a.x, a.y, rr, C.mail, { sx: a.sx, sy: a.sy });
      }
      if (lt < 1) return;

      /* ---- drain (window liquid recedes under the glass) ---- */
      if (lt >= 4.3) {
        var d = M.inOutCubic(M.prog(lt, 4.3, 5.1));
        if (d >= 1) return;
        var bot = M.lerp(884, 846, M.smooth(M.prog(d, 0, 0.5)));
        var dh = (bot - 104) * (1 - d), top = bot - dh, dw = WIN.w - 240 * d;
        P.box(960, bot - dh / 2, dw, dh, Math.min(WIN.r, dh / 2), SKY);
        var men = Math.min(1, d * 6) * (1 - d);
        for (var i = 0; i < 5; i++) {
          var mx = 960 + (i - 2) * dw * 0.2 + 14 * Math.sin(lt * 3 + i);
          P.ball(mx, top + 6 + 7 * Math.sin(lt * 9 + i * 1.7), Math.min(30 * men, dh * 0.5), SKY, { sx: 1.6, sy: 0.8 });
        }
        return;
      }

      var E = envB(lt), top = E.cy - E.h / 2, L = E.cx - E.w / 2, R = E.cx + E.w / 2;

      /* ---- 12.0: the hit: shockwave + splash ---- */
      var rp = M.prog(lt, 1, 1.75);
      if (rp > 0 && rp < 1) P.ring(960, 560, 250 + 560 * M.outCubic(rp), 7 * (1 - rp), "#9fd4ff", { k: 12, clear: 0.7 });
      var rnd = M.rng(11);
      for (var k = 0; k < 7; k++) {
        var ang = -Math.PI / 2 + (k - 3) * 0.44 + (rnd() - 0.5) * 0.2;
        var sp = 760 + rnd() * 380, rad = 9 + rnd() * 7;
        var tau = lt - 1;
        if (tau > 0 && tau < 0.75) {
          var px = 960 + Math.cos(ang) * 190 + Math.cos(ang) * sp * tau;
          var py = 560 + Math.sin(ang) * 70 + Math.sin(ang) * sp * tau + 0.5 * 2600 * tau * tau;
          P.ball(px, py, rad * (1 - M.prog(tau, 0.4, 0.75)), k % 2 ? C.mail : "#7cc6ff", { k: 14, sy: 1.15 });
        }
      }

      /* ---- envelope body ---- */
      P.ball(E.cx, E.cy, 0.1, C.mail);
      P.box(E.cx, E.cy, E.w, E.h, E.r, C.mail);

      /* ---- open flap (back, behind the letter) ---- */
      /* a collinear tri flips sdTri inside-out over the whole frame: keep the apex >= 12 px off the hinge */
      var fp2 = M.prog(lt, 2.42, 2.7), fo2 = M.outBack(fp2, 1.8);
      if (fp2 > 0.04 && E.w > 30) P.tri(L + 14, top + 10, R - 14, top + 10, E.cx, top + 10 - Math.max(12, 118 * fo2 * (E.w / ENV_W)), 10, "#4AA8EE", { k: 24 });

      /* ---- title jet and liquid "ElyMail" ---- */
      if (lt > 1.1 && lt < 1.62) {
        var jp = M.outCubic(M.prog(lt, 1.1, 1.36));
        var tipY = M.lerp(top + 20, 318, jp);
        var thin = 1 - M.prog(lt, 1.4, 1.6);
        if (thin > 0.14) P.cap(960, top + 18, 960, tipY, 24 * thin, 12 * thin + 4, "#8cc9f6", { k: 26 });
        if (lt < 1.45) P.ball(960, tipY, 32 * jp, C.white, { k: 26 });
      }
      var dr = lt - 1.6;
      if (dr > 0 && dr < 0.3) P.ball(960, 336 + 0.5 * 2600 * dr * dr, 11, "#bfe2ff", { k: 18, sy: 1.2 });
      if (lt > 1.2 && lt < 2.78) {
        var rise = M.outCubic(M.prog(lt, 1.2, 1.6));
        var amt = (0.42 + 0.58 * rise) * (1 - M.smooth(M.prog(lt, 2.56, 2.76)));
        var f = M.inCubic(M.prog(lt, 2.36, 2.72));
        var sc = (0.55 + 0.45 * M.spring(lt, 1.2, 2.2, 0.42)) * (1 + 0.012 * Math.sin((lt - 1.2) * 7.5)) * M.lerp(1, 0.32, f);
        P.text(0, TITLE, 960, M.lerp(272, 470, f), { scale: sc, color: C.white, amount: amt });
      }

      /* ---- letter: solid white on the front while it slides out, melts into the back slab at the snap ---- */
      var Lt = letter(lt, top);
      if (Lt && lt >= 3.45) P.box(Lt.cx, Lt.cy, Lt.w, Lt.h, Lt.r, Lt.c, { k: 12 });
      if (Lt && lt < 3.45) {
        P.layer(1);
        var pp = M.prog(lt, 3.18, 3.45);
        var n = 58 * Math.pow(1 - pp, 1.5);
        if (n > 4) P.cap(960, top + 4, 960, Lt.yb - 6, n * 0.9, n, WHITE, { k: 18 });
        P.box(Lt.cx, Lt.cy, Lt.w, Lt.h, Lt.r, WHITE, { k: 18 });
        P.layer(0);
      } else if (Lt && lt < 3.64) {
        var mq = 1 - M.inOutCubic(M.prog(lt, 3.45, 3.64));
        P.layer(1);
        P.box(Lt.cx, Lt.cy, Lt.w * (0.25 + 0.75 * mq) * mq, Lt.h * mq, 22 * mq, WHITE, { k: 18 });
        P.layer(0);
      }
      var sat = lt - 3.42;
      if (sat > 0 && sat < 0.2) {
        P.layer(1);
        P.ball(960, 412 + 0.5 * 3000 * sat * sat, 8 * (1 - M.prog(sat, 0.08, 0.2)), WHITE, { k: 12 });
        P.layer(0);
      }

      /* ---- closed flap (front, glossy white) ---- */
      if (lt < 2.47) {
        var fa = M.outBack(M.prog(lt, 1.02, 1.28), 2.2);
        var fo = M.smooth(M.prog(lt, 2.2, 2.45));
        var x0 = L + 30, x1 = R - 30, yT = top + 22, ay = M.lerp(top + E.h * 0.5, top + 36, fo);
        var gx = (x0 + x1 + E.cx) / 3, gy = (2 * yT + ay) / 3;
        function S(px, py, o) { return [gx + (px - gx) * fa, gy + (py - gy) * fa][o]; }
        if (fa > 0.02) {
          P.layer(1);
          P.tri(S(x0, yT, 0), S(x0, yT, 1), S(x1, yT, 0), S(x1, yT, 1), S(E.cx, ay, 0), S(E.cx, ay, 1), 12 * (1 - fo) + 2, C.white, { k: 20 });
          P.layer(0);
        }
      }
    },
  });
})();
