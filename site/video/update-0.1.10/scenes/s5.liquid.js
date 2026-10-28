/* s5 writing and sending, 34–44 s; window.S5X shares the model. */
(function () {
  "use strict";
  var M = LQ.M, C = LQ.C;
  var X = (window.S5X = {});

  /* ---------- compose window (screen space) ---------- */
  X.CW = { x: 560, y: 152, w: 1000, h: 640 };
  X.CC = [1060, 472]; // its centre
  X.SEND = { x: 1424, y: 734, w: 112, h: 44 }; // send button (screen)
  X.FS = 30; // editor font size
  X.LH = 52; // line pitch
  X.LB = 44; // line box height
  X.LX = 40; // editor left (compose-local)
  X.lineY = function (i) {
    return 232 + X.LH * i; // top of line box, compose-local
  };
  X.glyphCY = function (i) {
    return X.CW.y + X.lineY(i) + X.LB / 2;
  };

  /* ---------- the Markdown source and its typing schedule ---------- */
  var SRC = ["评审时间：", "**周五 15:00**", "请带上：", "- 新版原型", "- 动效稿"];
  X.SRC = SRC;
  X.L5 = "谢谢大家！";
  function isAscii(ch) {
    return ch.charCodeAt(0) < 128;
  }
  var G = [];
  var rng = M.rng(1534);
  var t = 1.42;
  SRC.forEach(function (line, li) {
    var x = X.LX;
    for (var k = 0; k < line.length; k++) {
      var ch = line[k];
      var adv = isAscii(ch) ? X.FS * 0.6 : X.FS;
      var role = "text";
      if (li === 1 && (k <= 1 || k >= 10)) role = "star";
      if (li >= 3 && k === 0) role = "dash";
      if (li >= 3 && k === 1) role = "gap";
      G.push({ ch: ch, line: li, k: k, x: x, adv: adv, role: role, bold: li === 1 && role === "text", ascii: isAscii(ch), t: t });
      x += adv;
      t += 0.034 + (rng() - 0.5) * 0.022 + (ch === "：" ? 0.05 : 0);
    }
    t += 0.1;
  });
  X.G = G;
  X.TYPE_END = G[G.length - 1].t;
  X.L5T = [];
  for (var q = 0; q < X.L5.length; q++) X.L5T.push(4.12 + q * 0.072 + (rng() - 0.5) * 0.02);
  X.SAVE = X.L5T[X.L5T.length - 1] + 1.5; // auto-save after a 1.5 s pause

  /* Melt schedule: ** pairs drip; the dash becomes a bullet. */
  X.MELT = {};
  var mt = { "1:0": 3.12, "1:1": 3.16, "1:10": 3.24, "1:11": 3.28, "3:0": 3.38, "4:0": 3.44 };
  G.forEach(function (g, i) {
    var key = g.line + ":" + g.k;
    if (mt[key] != null) X.MELT[i] = mt[key];
    if (g.role === "gap") X.MELT[i] = mt[g.line + ":0"];
  });
  X.SLIDE = 3.22; // bold text slides left
  X.BOLD = 3.3;
  X.BULLET = function (line) {
    return mt[line + ":0"] + 0.3;
  };
  X.slide = function (lt) {
    return M.spring(lt, X.SLIDE, 2.3, 0.62);
  };

  /* ---------- the rest of the story ---------- */
  X.OFF = 7.0; // connection drops
  X.SENDT = 7.4; // send pressed (music muffles)
  X.BACK = 8.42; // connection returns
  X.FIRE = 8.6; // slingshot release (music returns)
  X.TOAST = 8.72;

  function drop(P, x, y, r, col, o) {
    P.ball(x, y, r, col, o);
  }
  function quad(a, c, b, u) {
    var v = 1 - u;
    return [v * v * a[0] + 2 * v * u * c[0] + u * u * b[0], v * v * a[1] + 2 * v * u * c[1] + u * u * b[1]];
  }

  /* compose window born from the 写邮件 button */
  function birth(P, lt, g) {
    if (lt < 0.28 || lt > 1.34) return;
    var pc = [g.pill.x + g.pill.w / 2, g.pill.y + g.pill.h / 2];
    var A0 = [pc[0] - 24, pc[1] + 8], CTL = [pc[0] - 40, X.CC[1] - 40];
    if (lt < 0.68) {
      var sw = M.prog(lt, 0.28, 0.42), ret = M.inCubic(M.prog(lt, 0.5, 0.68));
      var pw = g.pill.w * (1 + 0.1 * Math.sin(Math.PI * sw)) * (1 - 0.7 * ret), ph = g.pill.h * (1 + 0.22 * Math.sin(Math.PI * sw)) * (1 - ret);
      if (ph > 1.5) P.box(pc[0], pc[1], pw, ph, ph / 2, C.mail, { k: 24 });
    }
    if (lt >= 0.34 && lt < 0.64) {
      var u = M.inOutCubic(M.prog(lt, 0.34, 0.64));
      var b = quad(A0, CTL, X.CC, u), b2 = quad(A0, CTL, X.CC, Math.min(u + 0.03, 1));
      var r = M.lerp(10, 44, u);
      var nk = 9 * (1 - M.prog(lt, 0.36, 0.52));
      if (nk > 0.8) P.cap(A0[0], A0[1], b[0], b[1], nk, nk * 0.8 + 3, C.mail, { k: 18 });
      var st = 1 + 0.5 * Math.sin(Math.PI * u);
      P.ball(b[0], b[1], r, C.mail, { sx: st, sy: 1 / st, rot: Math.atan2(b2[1] - b[1], b2[0] - b[0]), k: 26 });
      return;
    }
    if (lt >= 0.64) {
      // The drop inflates into a bubble, then recedes into 发送.
      var s = M.spring(lt, 0.62, 2.6, 0.38);
      var wv = M.wobble(lt, 0.66, 3.2, 5, 1);
      var w = M.lerp(88, 320, s) * (1 + 0.06 * wv), h = M.lerp(88, 200, s) * (1 - 0.07 * wv);
      var rr = M.lerp(44, 56, M.clamp(s, 0, 1));
      var rc = M.inOutCubic(M.prog(lt, 0.9, 1.24));
      var tx = X.SEND.x + X.SEND.w / 2, ty = X.SEND.y + X.SEND.h / 2;
      var cx = M.lerp(X.CC[0], tx, rc), cy = M.lerp(X.CC[1], ty, rc) - 60 * Math.sin(Math.PI * rc);
      w = M.lerp(w, X.SEND.w, rc);
      h = M.lerp(h, X.SEND.h, rc);
      rr = M.lerp(rr, 22, rc);
      var st = 1 + 0.25 * Math.sin(Math.PI * rc);
      if (lt > 1.3) return;
      P.box(cx, cy, w * st, h / st, rr, C.mail, { clear: 0.3 * (1 - rc), rot: -0.12 * Math.sin(Math.PI * rc), k: 30 });
    }
  }

  /* each Markdown mark liquefies */
  function melt(P, lt) {
    if (lt < 3.08 || lt > 4.05) return;
    for (var i in X.MELT) {
      var g = G[i], T = X.MELT[i];
      if (g.role === "gap") continue;
      var d = lt - T;
      if (d < 0 || d > 0.85) continue;
      var cx = X.CW.x + g.x + g.adv / 2, cy = X.glyphCY(g.line);
      var grow = M.outBack(M.prog(d, 0, 0.12), 2);
      if (g.role === "star") {
        cy -= 6;
        var r = 10 * (0.6 + 0.4 * grow);
        if (d < 0.14) {
          P.ball(cx, cy, r, C.mail, { k: 10 });
        } else {
          var f = d - 0.14;
          var fall = 40 * f + 0.5 * 2600 * Math.max(0, f - 0.08) * Math.max(0, f - 0.08);
          var by = cy + 6 + fall, sag = M.smooth(M.prog(f, 0, 0.12));
          var anchorR = r * (1 - M.smooth(M.prog(f, 0.08, 0.3)));
          if (anchorR > 0.5) P.ball(cx, cy, anchorR, C.mail, { sx: 1.1, sy: 0.85, k: 10 });
          var neck = 3.2 * (1 - M.prog(f, 0.06, 0.16));
          if (neck > 0.4) P.cap(cx, cy, cx, by, neck, neck + 1.5, C.mail, { k: 8 });
          var dr = r * (0.75 + 0.15 * sag) * (1 - M.smooth(M.prog(f, 0.45, 0.7)));
          if (dr > 0.4) P.ball(cx + 2 * Math.sin(f * 9), by, dr, C.mail, { sy: 1 + 0.5 * sag * Math.min(1, fall / 40), sx: 1 - 0.2 * sag, k: 10 });
        }
      } else if (g.role === "dash") {
        cy += 1;
        var con = M.smooth(M.prog(d, 0.06, 0.26));
        var half = M.lerp(7.5, 0, con);
        var rad = M.lerp(2.8, 6.8, con) * (0.6 + 0.4 * grow);
        var bx = X.CW.x + X.LX + 9; // bullet centre
        var px = M.lerp(cx, bx, con);
        var fade = 1 - M.smooth(M.prog(d, 0.42, 0.55));
        if (fade > 0.02) {
          var w = M.wobble(lt, T + 0.26, 4, 6, 1);
          P.cap(px - half, cy, px + half, cy, rad * fade * (1 + 0.15 * w), rad * fade * (1 - 0.15 * w), C.mail, { k: 10 });
        }
        // the excess drips away
        var e = d - 0.18;
        if (e > 0 && e < 0.6) {
          var ey = cy + 4 + 0.5 * 2400 * e * e;
          var er = 3.6 * (1 - M.smooth(M.prog(e, 0.35, 0.6)));
          if (e < 0.08) P.cap(px, cy, px, ey, 1.6, er, C.mail, { k: 6 });
          if (er > 0.3) P.ball(px, ey, er, C.mail, { sy: 1.35, k: 6 });
        }
      }
    }
  }

  /* the saved draft flies into 草稿 */
  function toDrafts(P, lt, g) {
    var T0 = X.SAVE + 0.06;
    if (lt < T0 || lt > T0 + 0.7) return;
    var a = [g.chip.x + 18, g.chip.y + g.chip.h / 2];
    var b = [g.dbadge.x + g.dbadge.w / 2, g.dbadge.y + g.dbadge.h / 2];
    var d = lt - T0;
    var r = 11 * M.outBack(M.prog(d, 0, 0.12), 2);
    if (d < 0.1) {
      P.ball(a[0], a[1], r, C.mail, { k: 14 });
      return;
    }
    var u = M.inOutCubic(M.prog(d, 0.1, 0.56));
    var ctl = [M.lerp(a[0], b[0], 0.5), Math.min(a[1], b[1]) - 150];
    var p = quad(a, ctl, b, u), p2 = quad(a, ctl, b, Math.min(u + 0.03, 1));
    var neck = 4 * (1 - M.prog(d, 0.1, 0.3));
    var an = 7 * (1 - M.smooth(M.prog(d, 0.22, 0.36)));
    if (an > 0.4) P.ball(a[0], a[1], an, C.mail, { k: 12 });
    if (neck > 0.5) P.cap(a[0], a[1], p[0], p[1], neck, neck + 2, C.mail, { k: 12 });
    var ab = M.prog(d, 0.56, 0.68);
    var st = 1 + 0.45 * Math.sin(Math.PI * u) * (1 - ab);
    var rr = r * (1 - 0.3 * u) * (1 - M.inCubic(ab));
    if (rr > 0.4) P.ball(p[0], p[1], rr, C.mail, { sx: st * (1 + 0.4 * ab), sy: (1 / st) * (1 - 0.4 * ab), rot: ab > 0 ? 0 : Math.atan2(p2[1] - p[1], p2[0] - p[0]), k: 14 });
  }

  /* outbox: rest point beside its label */
  X.rest = function (g) {
    return [g.obox.x + 132, g.obox.y + g.obox.h / 2];
  };
  X.PULL = [-40, 15];
  X.waitY = function (lt) {
    var ph = (lt - 8.04) / 0.11;
    return ph > 0 ? 18 * Math.abs(Math.sin(Math.PI * ph)) : 0;
  };
  X.SKY = [2150, -280];

  /* Send offline: crumple, hop into 发件箱, fidget, slingshot out. */
  function sendFlow(P, lt, g) {
    if (lt < X.SENDT + 0.02 || lt > 9.1) return;
    var R = X.rest(g);
    var c0 = [X.CC[0], X.CC[1] + 20];
    if (lt < 7.74) {
      // the squished window condenses into a drop
      var e = M.prog(lt, 7.42, 7.74);
      var g0 = M.outCubic(M.prog(lt, 7.42, 7.56));
      var cr = M.inOutCubic(M.prog(lt, 7.54, 7.74));
      var w = M.lerp(40, 300, g0), h = M.lerp(30, 190, g0);
      w = M.lerp(w, 76, cr);
      h = M.lerp(h, 76, cr);
      var wr = Math.sin(e * Math.PI * 4) * (1 - cr) * 0.08;
      w *= 1 + wr;
      h *= 1 - wr;
      var cx = X.CC[0], cy = M.lerp(X.CC[1], c0[1], cr);
      var clr = 0.3 * (1 - cr);
      P.box(cx, cy, w, h, M.lerp(28, 38, cr), C.mail, { clear: clr, rot: -0.1 + 0.4 * cr, k: 34 });
      var pinch = Math.sin(Math.PI * M.prog(lt, 7.5, 7.74)) * 26;
      if (pinch > 1) {
        for (var k = 0; k < 4; k++) {
          var sxn = k % 2 ? 1 : -1, syn = k < 2 ? -1 : 1;
          var ca = 0.4 * cr;
          P.ball(cx + sxn * w * (0.44 - ca), cy + syn * h * (0.44 - ca), pinch * (0.75 + 0.25 * (k / 3)), C.mail, { clear: clr, k: 36 });
        }
      }
      return;
    }
    if (lt < 8.02) {
      var u = M.prog(lt, 7.74, 8.02);
      var x = M.lerp(c0[0], R[0], u), y = M.lerp(c0[1], R[1], u) - 230 * 4 * u * (1 - u);
      var x2 = M.lerp(c0[0], R[0], u + 0.02), y2 = M.lerp(c0[1], R[1], u + 0.02) - 230 * 4 * (u + 0.02) * (0.98 - u);
      var jl = X_jel(lt, 7.74);
      var st = 1 + 0.4 * Math.sin(Math.PI * u);
      P.ball(x, y, M.lerp(38, 18, M.smooth(u)), C.mail, { sx: st * (1 + 0.1 * jl), sy: 1 / st, rot: Math.atan2(y2 - y, x2 - x), k: 16 });
      return;
    }
    if (lt < X.BACK + 0.02) {
      // impatient: tiny hops on the spot
      var ph = (lt - 8.04) / 0.11;
      var hop = ph > 0 ? Math.abs(Math.sin(Math.PI * ph)) : 0;
      var land = X_jel(lt, 8.02) * 0.5;
      var contact = ph > 0 ? Math.pow(1 - hop, 6) : 0;
      var sq = Math.max(land, 0.3 * contact);
      P.ball(R[0] + 2 * Math.sin(lt * 31), R[1] - 18 * hop + 4 * sq, 18, C.mail, { sx: 1 + 0.45 * sq - 0.08 * hop, sy: 1 - 0.38 * sq + 0.1 * hop, k: 16 });
      return;
    }
    if (lt < X.FIRE) {
      // slingshot: pulled back, a neck tethers it to the row
      var pu = M.outCubic(M.prog(lt, X.BACK, X.FIRE - 0.04));
      var bx = R[0] + X.PULL[0] * pu, by = R[1] + X.PULL[1] * pu;
      var ang = Math.atan2(R[1] - by, R[0] - bx);
      var trem = 1.5 * Math.sin(lt * 90) * pu;
      P.ball(R[0], R[1], 7, C.mail, { k: 14 });
      P.cap(R[0], R[1], bx, by, 4.5 - 1.5 * pu, 8, C.mail, { k: 12 });
      P.ball(bx + trem, by, 18, C.mail, { sx: 1 + 0.35 * pu, sy: 1 - 0.22 * pu, rot: ang, k: 14 });
      return;
    }
    // release
    var f = lt - X.FIRE;
    var p0 = [R[0] + X.PULL[0], R[1] + X.PULL[1]];
    var dx = X.SKY[0] - p0[0], dy = X.SKY[1] - p0[1], L = Math.sqrt(dx * dx + dy * dy);
    var ux = dx / L, uy = dy / L;
    var dist = 2700 * f + 7600 * f * f;
    var neck = 6 * (1 - M.prog(f, 0, 0.05));
    var anc = 8 * (1 - M.smooth(M.prog(f, 0.02, 0.12)));
    var aw = M.wobble(lt, X.FIRE, 7, 9, 1);
    if (anc > 0.4) P.ball(R[0], R[1], anc, C.mail, { sx: 1 + 0.3 * aw, sy: 1 - 0.3 * aw, k: 12 });
    var x0 = p0[0] + ux * dist, y0 = p0[1] + uy * dist;
    if (neck > 0.5) P.cap(R[0], R[1], x0, y0, neck * 0.6, neck, C.mail, { k: 10 });
    if (y0 > -80 && x0 < 2040) {
      var spd = Math.min(1, (2700 + 15200 * f) / 4000);
      var rot = Math.atan2(uy, ux);
      P.ball(x0, y0, 18, C.mail, { sx: 1 + 0.9 * spd, sy: 1 / Math.sqrt(1 + 0.9 * spd), rot: rot, k: 14 });
      for (var j = 1; j <= 2; j++) {
        var bd = dist - 46 * j - 20 * j * spd;
        if (bd > 10) P.ball(p0[0] + ux * bd, p0[1] + uy * bd, 7 - 2.4 * j, C.mail, { sx: 1.5, sy: 0.75, rot: rot, k: 14 });
      }
    }
  }
  function X_jel(lt, T) {
    var x = lt - T;
    if (x < 0) return 0;
    return Math.exp(-6 * x) * Math.cos(2 * Math.PI * 4 * x);
  }

  LQ.add({
    id: "s5",
    start: 34,
    end: 44,
    build: function (lt, P) {
      var g = X.geo;
      if (!g) return;
      P.layer(1);
      birth(P, lt, g);
      melt(P, lt);
      toDrafts(P, lt, g);
      sendFlow(P, lt, g);
      P.layer(0);
    },
  });
})();
