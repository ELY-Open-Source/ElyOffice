/* s3: accounts, global 18s to 26s.
   Back layer: each provider arrives as a drop and squashes into its chip (the HTML chip pops over it).
   Front layer: each chip melts back into a drop and is reeled along a gooey string into 全部收件箱. */
(function () {
  var M = LQ.M, C = LQ.C;
  /* shared with scenes/s3.html (keep the numbers in sync) */
  var S3 = ({
    cam: { s: 0.78, x: 514, y: 60 },
    chipW: 200,
    chipH: 64,
    cx: 180,
    /* land = local landing time, y0/y1 = chip centre before/after the cards leave, T = absorption time */
    acc: [
      { id: "gmail", c: "#EA4B5A", land: 0.5, y0: 264, y1: 264, T: 5.5, from: "top" },
      { id: "ms", c: "#2F8CF0", land: 1.0, y0: 480, y1: 344, T: 6.0, from: "left" },
      { id: "icloud", c: "#8FB4FF", land: 4.0, y0: 424, y1: 424, T: 6.5, from: "left" },
      { id: "imap", c: "#7B5CFF", land: 4.25, y0: 504, y1: 504, T: 6.85, from: "left" },
      { id: "pop3", c: "#2FD8E8", land: 4.5, y0: 584, y1: 584, T: 7.0, from: "left" },
    ],
    moveUp: [3.85, 4.15],
  });
  function hx(c) { return M.hex(c).slice(0, 3); }
  var MAIL = hx(C.mail);

  /* the 全部收件箱 icon, in screen space, following the camera return (power3.inOut == cubic in-out) */
  function mouth(lt) {
    var k = M.inOutCubic(M.prog(lt, 7, 8));
    return { x: M.lerp(696.7, 234.2, k), y: M.lerp(244.9, 237, k), s: M.lerp(S3.cam.s, 1, k) };
  }
  function chipY(a, lt) {
    return M.lerp(a.y0, a.y1, M.inOutCubic(M.prog(lt, S3.moveUp[0], S3.moveUp[1])));
  }
  function bez(o, c, t, s) {
    var u = 1 - s;
    return { x: u * u * o.x + 2 * u * s * c.x + s * s * t.x, y: u * u * o.y + 2 * u * s * c.y + s * s * t.y };
  }
  function bezD(o, c, t, s) {
    return { x: 2 * (1 - s) * (c.x - o.x) + 2 * s * (t.x - c.x), y: 2 * (1 - s) * (c.y - o.y) + 2 * s * (t.y - c.y) };
  }

  /* back layer: a drop flies in and squashes into a 200x64 pill under the HTML chip */
  function arrive(P, lt, a) {
    var u = lt - a.land, cx = S3.cx, cy = a.y0;
    if (u < -0.45 || u > 0.46) return;
    if (u < 0) {
      var p = 1 + u / 0.45, x, y, vx, vy;
      if (a.from === "top") {
        x = cx;
        y = cy - 0.5 * 4000 * u * u;
        vx = 0;
        vy = -4000 * u;
      } else {
        x = M.lerp(-90, cx, p);
        y = cy - 300 * (1 - p) * (1 - p) - 70 * Math.sin(Math.PI * p);
        vx = (cx + 90) / 0.45;
        vy = (600 * (1 - p) - 70 * Math.PI * Math.cos(Math.PI * p)) / 0.45;
      }
      var sp = Math.min(1, Math.sqrt(vx * vx + vy * vy) / 2400);
      P.ball(x, y, 28, a.c, { sx: 1 / (1 + 0.45 * sp), sy: 1 + 0.45 * sp, rot: Math.atan2(vy, vx) - Math.PI / 2, k: 24 });
      if (a.from === "top") P.cap(x, y - 30, x, y - 30 - 60 * sp, 14, 3, a.c, { k: 14 });
      return;
    }
    var e = M.spring(lt, a.land, 2.6, 0.38);
    var wb = M.wobble(lt, a.land, 4, 7, 1);
    var gone = 1 - M.smooth(M.prog(u, 0.3, 0.46));
    var w = M.lerp(56, S3.chipW, e) * (1 + 0.12 * wb) * gone;
    var h = M.lerp(56, S3.chipH, M.clamp(e, 0, 1.2)) * (1 - 0.28 * wb) * gone;
    P.box(cx, cy + (64 - h) * 0.25, w, h, h / 2, a.c, { k: 20 });
    /* two splash droplets on impact */
    var tau = u;
    if (tau < 0.3) {
      for (var i = 0; i < 2; i++) {
        var dir = i ? 1 : -1;
        P.ball(cx + dir * (110 + 420 * tau), cy + 10 - 260 * tau + 0.5 * 2600 * tau * tau, 7 * (1 - tau / 0.3), a.c, { k: 12 });
      }
    }
  }

  /* front layer: chip -> drop -> pulled along a string -> gulped by the inbox */
  function strand(P, lt, a) {
    var u = lt - a.T;
    if (u < -0.8 || u > 0.32) return;
    var col = hx(a.c);
    var o = { x: S3.cx, y: chipY(a, lt) };
    var mo = mouth(lt), t = { x: mo.x, y: mo.y };
    var c = { x: (o.x + t.x) / 2 - 40, y: Math.min(o.y, t.y) - 120 - 0.18 * (o.y - t.y) };
    var depart = -0.42;

    /* melt: the chip-shaped puddle rounds into a drop */
    var m = M.smooth(M.prog(u, -0.8, -0.6));
    if (u < depart) {
      var pw = M.lerp(S3.chipW, 64, m), ph = M.lerp(S3.chipH, 62, m) * (1 + 0.1 * M.wobble(lt, a.T - 0.6, 4, 6, 1));
      P.box(o.x, o.y, pw, ph, ph / 2, col, { k: 26 });
    }

    /* drop position along the arc */
    var fp = M.prog(u, depart, 0);
    var s = M.inOutCubic(fp);
    var dp = bez(o, c, t, s);
    var dv = bezD(o, c, t, s);
    var speed = Math.sin(Math.PI * fp);
    var rd = M.lerp(31, 19 * mo.s / 0.78, M.smooth(M.prog(u, depart, -0.1))) * (1 - M.smooth(M.prog(u, -0.04, 0.12)));
    var dcol = M.mix3(col, MAIL, M.smooth(M.prog(u, -0.15, 0)));

    /* puddle left behind + the neck that thins and snaps */
    if (u >= depart) {
      var rec = 1 - M.smooth(M.prog(u, -0.2, -0.02));
      var pr = 30 * (1 - 0.45 * M.prog(u, depart, -0.28)) * rec * (1 + 0.14 * M.wobble(lt, a.T - 0.28, 5, 6, 1));
      P.ball(o.x, o.y + 4, pr, col, { sx: 1.25, sy: 0.8, k: 22 });
      var n = 15 * Math.pow(1 - M.prog(u, depart, -0.27), 1.4);
      if (n > 3.5) P.cap(o.x, o.y, dp.x, dp.y, n, n * 0.75, col, { k: 16 });
      var su = u + 0.27;
      if (su > 0 && su < 0.3) {
        var sp0 = bez(o, c, t, M.inOutCubic(M.prog(-0.27, depart, 0)));
        P.ball(M.lerp(o.x, sp0.x, 0.5), M.lerp(o.y, sp0.y, 0.5) + 0.5 * 2400 * su * su, 7 * (1 - M.prog(su, 0.12, 0.3)), col, { k: 12 });
      }
    }

    /* the string: shoots out of the inbox, hooks the drop, and is reeled in with it */
    var reach = M.outCubic(M.prog(u, -0.62, -0.46));
    if (reach > 0 && u < 0.02) {
      var s0 = u < depart ? 1 - reach : s;
      var thin = 1 - M.smooth(M.prog(u, -0.08, 0.02));
      var seg = 6, nx = -(t.y - o.y), ny = t.x - o.x, nl = Math.sqrt(nx * nx + ny * ny) || 1;
      nx /= nl;
      ny /= nl;
      var amp = 9 * thin * Math.min(1, reach * 1.4);
      var pts = [];
      for (var i = 0; i <= seg; i++) {
        var q = i / seg, sq = s0 + (1 - s0) * q, Bp = bez(o, c, t, sq);
        var wv = amp * Math.sin(Math.PI * q) * Math.sin(lt * 21 + q * 5.5);
        pts.push({ x: Bp.x + nx * wv, y: Bp.y + ny * wv, r: (3.6 + 22 * (q - 0.5) * (q - 0.5)) * thin, c: M.mix3(col, MAIL, sq) });
      }
      for (i = 0; i < seg; i++) P.cap(pts[i].x, pts[i].y, pts[i + 1].x, pts[i + 1].y, pts[i].r, pts[i + 1].r, pts[i].c, { k: 10 });
      if (u < depart) {
        var tip = bez(o, c, t, s0);
        P.ball(tip.x, tip.y, 9 * thin, M.mix3(col, MAIL, s0), { k: 16 });
      }
    }

    /* the travelling drop (stretched along its velocity) */
    if (u >= depart && rd > 0.6) {
      var st = 1 + 0.6 * speed;
      P.ball(dp.x, dp.y, rd, dcol, { sx: st, sy: 1 / st, rot: Math.atan2(dv.y, dv.x), k: 24 });
    }

    /* the inbox mouth: a little lip while the string is out, a gulp when the drop arrives */
    var lip = M.prog(u, -0.62, -0.55) * (1 - M.smooth(M.prog(u, 0.12, 0.3)));
    var gulp = Math.sin(Math.PI * M.prog(u, -0.04, 0.24));
    var mr = (9 + 14 * gulp) * lip * (mo.s / 0.78);
    if (mr > 0.6) P.ball(t.x, t.y, mr, MAIL, { sx: 1 + 0.25 * gulp, sy: 1 - 0.15 * gulp, k: 20 });
  }

  LQ.add({
    id: "s3",
    start: 18,
    end: 26,
    build: function (lt, P) {
      var i;
      for (i = 0; i < S3.acc.length; i++) arrive(P, lt, S3.acc[i]);
      P.layer(1);
      P.material({ shadow: [6, 14], shadowA: 0.4 });
      for (i = 0; i < S3.acc.length; i++) strand(P, lt, S3.acc[i]);
      P.layer(0);
    },
  });
})();
