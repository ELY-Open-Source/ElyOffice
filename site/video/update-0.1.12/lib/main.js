/* Master timeline: drives the liquid canvases and the caption track. */
(function () {
  "use strict";
  var DUR = 60;
  window.__hf = window.__hf || {};
  window.__hf.buildReady = window.__hf.buildReady || {};
  var state = { t: 0 };

  function buildCaptions(tl) {
    var pill = document.getElementById("cap-pill");
    var meas = document.createElement("div");
    meas.className = "cap-measure";
    document.body.appendChild(meas);
    var caps = window.CAPTIONS;
    var widths = caps.map(function (c) {
      meas.innerHTML = '<div class="cap-zh">' + c.zh + '</div><div class="cap-en">' + c.en + "</div>";
      return Math.min(1560, Math.ceil(meas.getBoundingClientRect().width) + 96);
    });
    meas.remove();
    var lines = caps.map(function (c) {
      var el = document.createElement("div");
      el.className = "cap-line";
      el.innerHTML = '<div class="cap-zh">' + c.zh + '</div><div class="cap-en">' + c.en + "</div>";
      pill.appendChild(el);
      return el;
    });
    gsap.set(pill, { width: 106, scale: 0, opacity: 0 });
    gsap.set(lines, { opacity: 0 });
    for (var i = 0; i < caps.length; i++) {
      var c = caps[i];
      var prev = caps[i - 1];
      var next = caps[i + 1];
      var gapBefore = prev ? c.s - prev.e : 99;
      var gapAfter = next ? next.s - c.e : 99;
      var zh = lines[i].children[0];
      var en = lines[i].children[1];
      if (gapBefore > 0.6) {
        tl.fromTo(pill, { width: 106, scale: 0.2, opacity: 0 }, { scale: 1, opacity: 1, duration: 0.42, ease: "back.out(2.2)", immediateRender: false }, c.s - 0.3);
      }
      tl.to(pill, { width: widths[i], duration: 0.66, ease: "elastic.out(1, 0.55)" }, c.s - 0.2);
      tl.fromTo(pill, { scaleY: 0.84, scaleX: 1.05 }, { scaleY: 1, scaleX: 1, duration: 0.62, ease: "elastic.out(1.2, 0.4)", immediateRender: false }, c.s - 0.2);
      tl.fromTo(lines[i], { opacity: 0 }, { opacity: 1, duration: 0.01, immediateRender: false }, c.s - 0.06);
      tl.fromTo(zh, { y: 18, opacity: 0, filter: "blur(8px)" }, { y: 0, opacity: 1, filter: "blur(0px)", duration: 0.5, ease: "power3.out", immediateRender: false }, c.s - 0.05);
      tl.fromTo(en, { y: 14, opacity: 0, filter: "blur(6px)" }, { y: 0, opacity: 1, filter: "blur(0px)", duration: 0.5, ease: "power3.out", immediateRender: false }, c.s + 0.05);
      tl.to([zh, en], { y: -12, opacity: 0, filter: "blur(6px)", duration: 0.28, ease: "power2.in", stagger: 0.03 }, c.e - 0.28);
      tl.set(lines[i], { opacity: 0 }, c.e + 0.04);
      if (gapAfter > 0.6) {
        tl.to(pill, { width: 106, duration: 0.3, ease: "power3.in" }, c.e - 0.12);
        tl.to(pill, { scale: 0.2, opacity: 0, duration: 0.24, ease: "power2.in" }, c.e + 0.14);
      }
    }
  }

  var fonts = ['900 200px "Inter"', '600 35px "Noto Sans SC"', '900 84px "Noto Sans SC"', '500 23px "Inter"', '500 20px "JetBrains Mono"'];
  window.__hf.buildReady.main = Promise.all(
    fonts.map(function (f) {
      return document.fonts.load(f, "Ely 邮件更新");
    })
  )
    .then(function () {
      return document.fonts.ready;
    })
    .then(function () {
      LQ.init();
      var tl = gsap.timeline({ paused: true });
      tl.to(state, { t: DUR, duration: DUR, ease: "none", onUpdate: function () { LQ.render(state.t); } }, 0);
      buildCaptions(tl);
      LQ.render(0);
      window.__timelines["main"] = tl;
    });
})();
