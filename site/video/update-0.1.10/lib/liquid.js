/* Ely liquid engine: glossy metaball goo + aurora, driven only by time.
   Coordinates are CSS pixels on the 1920x1080 frame, y down.
   Scenes register with LQ.add({ id, start, end, build(lt, P, t) }).
   Two canvases: #lq-back (aurora + liquid behind the UI) and #lq-front
   (transparent, liquid above the UI). P.layer(1) sends primitives to the front. */
(function () {
  "use strict";
  var W = 1920, H = 1080;

  var LQ = (window.LQ = {
    W: W,
    H: H,
    scenes: [],
    fonts: {},
    ready: null,
  });

  LQ.add = function (s) {
    LQ.scenes.push(s);
  };

  /* ---------- motion helpers (pure functions of time) ---------- */
  var M = (LQ.M = {});
  M.clamp = function (x, a, b) {
    return x < a ? a : x > b ? b : x;
  };
  M.lerp = function (a, b, t) {
    return a + (b - a) * t;
  };
  M.prog = function (t, a, b) {
    return M.clamp((t - a) / (b - a), 0, 1);
  };
  M.smooth = function (x) {
    x = M.clamp(x, 0, 1);
    return x * x * (3 - 2 * x);
  };
  M.outCubic = function (x) {
    x = M.clamp(x, 0, 1);
    return 1 - Math.pow(1 - x, 3);
  };
  M.inOutCubic = function (x) {
    x = M.clamp(x, 0, 1);
    return x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2;
  };
  M.outBack = function (x, s) {
    s = s == null ? 1.7 : s;
    x = M.clamp(x, 0, 1) - 1;
    return 1 + (s + 1) * x * x * x + s * x * x;
  };
  M.inCubic = function (x) {
    x = M.clamp(x, 0, 1);
    return x * x * x;
  };
  /* Damped spring from 0 to 1 that starts at time t0.
     freq in Hz, damp 0..1 (0.25 = very wobbly, 0.7 = soft). */
  M.spring = function (t, t0, freq, damp) {
    var x = t - t0;
    if (x <= 0) return 0;
    var w = 2 * Math.PI * (freq || 2.2);
    var z = damp == null ? 0.35 : damp;
    var wd = w * Math.sqrt(Math.max(1 - z * z, 0.0001));
    return 1 - Math.exp(-z * w * x) * (Math.cos(wd * x) + (z * w / wd) * Math.sin(wd * x));
  };
  /* Decaying wobble around 0 that starts at t0: a squash impulse. */
  M.wobble = function (t, t0, freq, damp, amp) {
    var x = t - t0;
    if (x <= 0) return 0;
    var w = 2 * Math.PI * (freq || 3);
    return (amp == null ? 1 : amp) * Math.exp(-(damp || 4) * x) * Math.sin(w * x);
  };
  /* Ballistic drop with bounces onto floor y = 0 (returns height above floor, >= 0). */
  M.bounce = function (t, t0, h0, g, restitution) {
    var x = t - t0;
    if (x < 0) return h0;
    g = g || 4200;
    var e = restitution == null ? 0.42 : restitution;
    var tFall = Math.sqrt((2 * h0) / g);
    if (x < tFall) return h0 - 0.5 * g * x * x;
    x -= tFall;
    var v = g * tFall * e;
    for (var i = 0; i < 6; i++) {
      var tb = (2 * v) / g;
      if (x < tb) return v * x - 0.5 * g * x * x;
      x -= tb;
      v *= e;
    }
    return 0;
  };
  M.hex = function (h, a) {
    h = h.replace("#", "");
    var n = parseInt(h, 16);
    return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255, a == null ? 1 : a];
  };
  M.mix3 = function (a, b, t) {
    return [M.lerp(a[0], b[0], t), M.lerp(a[1], b[1], t), M.lerp(a[2], b[2], t)];
  };
  /* Seeded PRNG */
  M.rng = function (seed) {
    var v = seed >>> 0;
    return function () {
      v += 0x6d2b79f5;
      var m = v;
      m = Math.imul(m ^ (m >>> 15), m | 1);
      m ^= m + Math.imul(m ^ (m >>> 7), m | 61);
      return ((m ^ (m >>> 14)) >>> 0) / 4294967296;
    };
  };

  /* App palette */
  LQ.C = {
    doc: "#3D7FE5",
    table: "#2E9E5B",
    show: "#F07B2A",
    pdf: "#E0344F",
    mail: "#1E8FE0",
    tree: "#7E57C2",
    mark: "#F2A900",
    white: "#F4F6FF",
    violet: "#7B5CFF",
    pink: "#FF5FA8",
    cyan: "#2FD8E8",
    peach: "#FFB46B",
    lime: "#B8F35A",
  };

  /* ---------- shaders: lib/shaders.js, loaded first ---------- */
  var GLSL = window.LQ_SHADERS;
  if (!GLSL) throw new Error("liquid.js: load lib/shaders.js first");
  var VS = GLSL.VS, FS_AURORA = GLSL.AURORA, FS_LIQ = GLSL.LIQ, MAXP = GLSL.MAXP;

  function compile(gl, type, src) {
    var s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
      throw new Error("liquid shader: " + gl.getShaderInfoLog(s));
    }
    return s;
  }
  function program(gl, fs) {
    var p = gl.createProgram();
    gl.attachShader(p, compile(gl, gl.VERTEX_SHADER, VS));
    gl.attachShader(p, compile(gl, gl.FRAGMENT_SHADER, fs));
    gl.bindAttribLocation(p, 0, "aP");
    gl.linkProgram(p);
    if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error("liquid link: " + gl.getProgramInfoLog(p));
    var u = {};
    var n = gl.getProgramParameter(p, gl.ACTIVE_UNIFORMS);
    for (var i = 0; i < n; i++) {
      var info = gl.getActiveUniform(p, i);
      var name = info.name.replace(/\[0\]$/, "");
      u[name] = gl.getUniformLocation(p, info.name);
    }
    return { p: p, u: u };
  }

  /* ---------- text fields ---------- */
  var TXW = 2048, TXH = 512;
  /* spec: { text, font: "900 220px Inter", w, h (field box in px), blur, align, ls (letterSpacing px) } */
  function textKey(s) {
    return [s.text, s.font, s.w, s.h, s.blur, s.ls || 0, s.align || "c", s.pad || 0, s.lh || 0].join("|");
  }
  function makeTextCanvas(s) {
    var c = document.createElement("canvas");
    c.width = TXW;
    c.height = TXH;
    var x = c.getContext("2d");
    var sx = TXW / s.w, sy = TXH / s.h;
    x.setTransform(sx, 0, 0, sy, 0, 0);
    x.filter = "blur(" + s.blur + "px)";
    x.fillStyle = "#fff";
    x.font = s.font;
    if (s.ls) x.letterSpacing = s.ls + "px";
    var al = s.align || "center";
    x.textAlign = al;
    x.textBaseline = "middle";
    var ax = al === "left" ? s.pad || 24 : al === "right" ? s.w - (s.pad || 24) : s.w / 2;
    var lines = String(s.text).split("\n");
    var lh = s.lh || 0;
    for (var i = 0; i < lines.length; i++) {
      x.fillText(lines[i], ax, s.h / 2 + (i - (lines.length - 1) / 2) * lh);
    }
    return c;
  }

  /* ---------- renderer ---------- */
  function Engine(canvas, front) {
    var gl = canvas.getContext("webgl", {
      alpha: !!front,
      premultipliedAlpha: true,
      antialias: false,
      preserveDrawingBuffer: true,
    });
    if (!gl) throw new Error("liquid: WebGL unavailable");
    this.gl = gl;
    this.front = !!front;
    var buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    this.liq = program(gl, FS_LIQ);
    if (!front) {
      this.aur = program(gl, FS_AURORA);
      this.bw = 640;
      this.bh = 360;
      this.bgTex = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, this.bgTex);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, this.bw, this.bh, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      this.fb = gl.createFramebuffer();
      gl.bindFramebuffer(gl.FRAMEBUFFER, this.fb);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, this.bgTex, 0);
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    }
    this.txTex = [this.newTex(), this.newTex()];
    this.txKey = ["", ""];
    this.A = new Float32Array(MAXP * 4);
    this.B = new Float32Array(MAXP * 4);
    this.Cc = new Float32Array(MAXP * 4);
  }
  Engine.prototype.newTex = function () {
    var gl = this.gl;
    var t = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, t);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(4));
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    return t;
  };
  var textCache = {};
  Engine.prototype.setText = function (slot, spec) {
    var gl = this.gl;
    var key = textKey(spec);
    if (this.txKey[slot] === key) return;
    var c = textCache[key] || (textCache[key] = makeTextCanvas(spec));
    gl.activeTexture(gl.TEXTURE7);
    gl.bindTexture(gl.TEXTURE_2D, this.txTex[slot]);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, c);
    this.txKey[slot] = key;
  };
  Engine.prototype.draw = function (t, F) {
    var gl = this.gl, u, i;
    var bg = F.bg;
    if (!this.front) {
      u = this.aur.u;
      gl.bindFramebuffer(gl.FRAMEBUFFER, this.fb);
      gl.viewport(0, 0, this.bw, this.bh);
      gl.useProgram(this.aur.p);
      gl.uniform1f(u.uT, t + bg.timeOffset);
      gl.uniform3fv(u.uBase, bg.base);
      gl.uniform3fv(u.uC0, bg.c0);
      gl.uniform3fv(u.uC1, bg.c1);
      gl.uniform3fv(u.uC2, bg.c2);
      gl.uniform3fv(u.uC3, bg.c3);
      gl.uniform4fv(u.uAmt, bg.amt);
      gl.uniform2fv(u.uWarp, bg.warp);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    }
    var L = this.front ? F.front : F.back;
    var n = Math.min(L.length, MAXP);
    for (i = 0; i < n; i++) {
      var p = L[i];
      this.A.set(p.a, i * 4);
      this.B.set(p.b, i * 4);
      this.Cc.set(p.c, i * 4);
    }
    gl.viewport(0, 0, W, H);
    gl.useProgram(this.liq.p);
    u = this.liq.u;
    gl.uniform2f(u.uRes, W, H);
    gl.uniform1f(u.uT, t);
    gl.uniform1i(u.uN, n);
    gl.uniform1i(u.uFront, this.front ? 1 : 0);
    gl.uniform4fv(u.uA, this.A);
    gl.uniform4fv(u.uB, this.B);
    gl.uniform4fv(u.uC, this.Cc);
    var mat = F.mat;
    gl.uniform1f(u.uBevel, mat.bevel);
    gl.uniform1f(u.uRefr, mat.refr);
    gl.uniform1f(u.uGloss, mat.gloss);
    gl.uniform1f(u.uBody, this.front ? mat.frontBody : mat.body);
    gl.uniform2fv(u.uShadow, mat.shadow);
    gl.uniform1f(u.uShadowA, mat.shadowA);
    gl.uniform1f(u.uGrain, this.front ? 0 : F.grain);
    gl.uniform1f(u.uFade, this.front ? 1 : F.fade);
    gl.uniform1f(u.uBgOn, this.front ? 0 : 1);
    gl.uniform3fv(u.uBase, bg.base);
    gl.uniform1f(u.uBright, bg.bright);
    gl.uniform1f(u.uVig, bg.vig);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.front ? this.txTex[0] : this.bgTex);
    gl.uniform1i(u.uBg, 0);
    // text slots
    var tx = this.front ? F.textFront : F.textBack;
    for (i = 0; i < 2; i++) {
      var T = tx[i];
      var P = i === 0 ? u.uTxP0 : u.uTxP1;
      if (T && T.amount > 0) {
        this.setText(i, T.spec);
        gl.activeTexture(gl.TEXTURE1 + i);
        gl.bindTexture(gl.TEXTURE_2D, this.txTex[i]);
        gl.uniform1i(i === 0 ? u.uTx0 : u.uTx1, 1 + i);
        gl.uniform4f(i === 0 ? u.uTxR0 : u.uTxR1, T.x - T.spec.w * T.scale / 2, T.y - T.spec.h * T.scale / 2, T.spec.w * T.scale, T.spec.h * T.scale);
        gl.uniform4f(P, T.color[0], T.color[1], T.color[2], T.amount);
        gl.uniform1f(u.uTxBlur, T.spec.blur * T.scale);
      } else {
        gl.activeTexture(gl.TEXTURE1 + i);
        gl.bindTexture(gl.TEXTURE_2D, this.txTex[i]);
        gl.uniform1i(i === 0 ? u.uTx0 : u.uTx1, 1 + i);
        gl.uniform4f(P, 0, 0, 0, 0);
      }
    }
    gl.uniform1f(u.uTxK, F.textK);
    gl.uniform1f(u.uTxSum, (this.front ? F.textSumFront : F.textSumBack) ? 1 : 0);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  };

  /* ---------- frame builder handed to scenes ---------- */
  function defaultBg() {
    return {
      base: M.hex("#08061A").slice(0, 3),
      c0: M.hex("#5B3CF0").slice(0, 3),
      c1: M.hex("#1E8FE0").slice(0, 3),
      c2: M.hex("#FF4FA3").slice(0, 3),
      c3: M.hex("#2FD8E8").slice(0, 3),
      amt: [0.9, 0.75, 0.55, 0.45],
      warp: [0, 0],
      bright: 1,
      vig: 0.55,
      timeOffset: 0,
    };
  }
  function Frame() {
    this.back = [];
    this.front = [];
    this.textBack = [null, null];
    this.textFront = [null, null];
    this.bg = defaultBg();
    this.mat = { bevel: 34, refr: 22, gloss: 1, body: 0.42, frontBody: 0.82, shadow: [10, 26], shadowA: 0.45 };
    this.textK = 26;
    this.textSumBack = false;
    this.textSumFront = false;
    this.grain = 0.035;
    this.fade = 1;
    this._layer = 0;
  }
  function col(c) {
    if (typeof c === "string") return M.hex(c).slice(0, 3);
    return c;
  }
  Frame.prototype._push = function (a, b, c) {
    (this._layer ? this.front : this.back).push({ a: a, b: b, c: c });
  };
  /* P.ball(x, y, r, color, {sx, sy, rot, k}) */
  Frame.prototype.ball = function (x, y, r, color, o) {
    o = o || {};
    if (r <= 0.05) return this;
    var c = col(color);
    this._push([x, y, r * (o.sx || 1), r * (o.sy || 1)], [o.rot || 0, 0, o.clear || 0, 0], [c[0], c[1], c[2], o.k == null ? 40 : o.k]);
    return this;
  };
  /* P.cap(ax, ay, bx, by, ra, rb, color, {k}) : tapered capsule, use for strings and necks */
  Frame.prototype.cap = function (ax, ay, bx, by, ra, rb, color, o) {
    o = o || {};
    if (ra <= 0.05 && rb <= 0.05) return this;
    var c = col(color);
    this._push([ax, ay, bx, by], [ra, rb, o.clear || 0, 1], [c[0], c[1], c[2], o.k == null ? 30 : o.k]);
    return this;
  };
  /* P.box(cx, cy, w, h, radius, color, {rot, k}) : full width/height */
  Frame.prototype.box = function (x, y, w, h, r, color, o) {
    o = o || {};
    if (w <= 0.1 || h <= 0.1) return this;
    var c = col(color);
    r = Math.min(r, w / 2, h / 2);
    this._push([x, y, w / 2, h / 2], [o.rot || 0, r, o.clear || 0, 2], [c[0], c[1], c[2], o.k == null ? 40 : o.k]);
    return this;
  };
  /* P.ring(cx, cy, radius, halfWidth, color, {k}) */
  Frame.prototype.ring = function (x, y, r, w, color, o) {
    o = o || {};
    var c = col(color);
    this._push([x, y, r, w], [0, 0, o.clear || 0, 3], [c[0], c[1], c[2], o.k == null ? 20 : o.k]);
    return this;
  };
  /* P.text(slot, {text, font, w, h, blur, ls, lh}, x, y, {scale, color, amount}) */
  Frame.prototype.text = function (slot, spec, x, y, o) {
    o = o || {};
    var T = {
      spec: spec,
      x: x,
      y: y,
      scale: o.scale || 1,
      color: col(o.color || "#F4F6FF"),
      amount: o.amount == null ? 1 : o.amount,
    };
    (this._layer ? this.textFront : this.textBack)[slot] = T;
    return this;
  };
  /* P.tri(x0,y0,x1,y1,x2,y2, round, color, {k}) : rounded triangle */
  Frame.prototype.tri = function (x0, y0, x1, y1, x2, y2, r, color, o) {
    o = o || {};
    // A near-collinear triangle turns the SDF inside out and floods the layer.
    var area2 = Math.abs((x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0));
    var span = Math.max(Math.hypot(x1 - x0, y1 - y0), Math.hypot(x2 - x1, y2 - y1), Math.hypot(x0 - x2, y0 - y2));
    if (!(area2 > span * 2)) return this;
    var c = col(color);
    this._push([x0, y0, x1, y1], [x2, y2, r || 0, 4], [c[0], c[1], c[2], o.k == null ? 30 : o.k]);
    return this;
  };
  /* P.morph(specA, specB, x, y, f, {scale, colorA, colorB}) : one liquid field blended from A (f=0) to B (f=1). Uses both text slots. */
  Frame.prototype.morph = function (specA, specB, x, y, f, o) {
    o = o || {};
    this.text(0, specA, x, y, { scale: o.scale, color: o.colorA || o.color, amount: 1 - f });
    this.text(1, specB, x, y, { scale: o.scale, color: o.colorB || o.color, amount: f });
    if (this._layer) this.textSumFront = true; else this.textSumBack = true;
    return this;
  };
  Frame.prototype.layer = function (l) {
    this._layer = l;
    return this;
  };
  /* Background mood: P.mood({ base, c0, c1, c2, c3 (hex), amt [4], warp [2], bright, vig }, weight) */
  Frame.prototype.mood = function (m, w) {
    w = w == null ? 1 : w;
    var b = this.bg;
    ["base", "c0", "c1", "c2", "c3"].forEach(function (k) {
      if (m[k]) b[k] = M.mix3(b[k], col(m[k]), w);
    });
    if (m.amt) for (var i = 0; i < 4; i++) b.amt[i] = M.lerp(b.amt[i], m.amt[i], w);
    if (m.warp) b.warp = [M.lerp(b.warp[0], m.warp[0], w), M.lerp(b.warp[1], m.warp[1], w)];
    if (m.bright != null) b.bright = M.lerp(b.bright, m.bright, w);
    if (m.vig != null) b.vig = M.lerp(b.vig, m.vig, w);
    return this;
  };
  Frame.prototype.material = function (m) {
    for (var k in m) this.mat[k] = m[k];
    return this;
  };

  var back, front;
  /* Global background mood track, keyframed in global seconds (lib/moods.js). */
  LQ.moods = [];
  function applyMoodTrack(F, t) {
    var K = LQ.moods;
    if (!K.length) return;
    var i = 0;
    while (i < K.length - 1 && K[i + 1].t <= t) i++;
    F.mood(K[i].mood, 1);
    if (i < K.length - 1) {
      var blend = K[i + 1].blend || 1.6;
      var w = M.smooth((t - (K[i + 1].t - blend)) / blend);
      if (w > 0) F.mood(K[i + 1].mood, w);
    }
  }
  LQ.frame = function (t) {
    var F = new Frame();
    applyMoodTrack(F, t);
    for (var i = 0; i < LQ.scenes.length; i++) {
      var s = LQ.scenes[i];
      if (t >= s.start && t < s.end) {
        F._layer = 0;
        try {
          s.build(t - s.start, F, t);
        } catch (e) {
          console.error("liquid scene " + s.id + " failed at " + t.toFixed(3) + ": " + e.message);
        }
      }
    }
    return F;
  };
  LQ.render = function (t) {
    if (!back) return;
    var F = LQ.frame(t);
    back.draw(t, F);
    if (front) front.draw(t, F);
  };
  LQ.init = function () {
    back = new Engine(document.getElementById("lq-back"), false);
    var fc = document.getElementById("lq-front");
    if (fc) front = new Engine(fc, true);
  };
})();
