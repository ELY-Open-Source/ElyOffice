/* Global aurora mood keyframes (global seconds). A key blends in over `blend` seconds ending at its t. */
(function () {
  var dark = { bright: 0.0 };
  var dawn = { base: "#07051A", c0: "#5B3CF0", c1: "#1E8FE0", c2: "#FF4FA3", c3: "#2FD8E8", amt: [0.85, 0.7, 0.45, 0.4], bright: 1, vig: 0.6 };
  var table = { base: "#04120E", c0: "#1F8F5B", c1: "#2FD8A8", c2: "#3A7BFF", c3: "#B8F35A", amt: [0.85, 0.7, 0.45, 0.4], bright: 1.0, vig: 0.58 };
  var pdf = { base: "#14060E", c0: "#E0344F", c1: "#7B3CF0", c2: "#FFB46B", c3: "#FF5FA8", amt: [0.8, 0.6, 0.45, 0.35], bright: 1.0, vig: 0.58 };
  var mark = { base: "#120C04", c0: "#F2A900", c1: "#FF5FA8", c2: "#7B5CFF", c3: "#FFB46B", amt: [0.8, 0.6, 0.45, 0.35], bright: 1.02, vig: 0.58 };
  var calm = { base: "#090722", c0: "#5B3CF0", c1: "#2FD8E8", c2: "#3A7BFF", c3: "#C04FD8", amt: [0.8, 0.65, 0.45, 0.35], bright: 0.98, vig: 0.6 };
  var party = { base: "#0A0620", c0: "#6E44FF", c1: "#FF5FA8", c2: "#2FD8E8", c3: "#FFB46B", amt: [0.95, 0.8, 0.65, 0.5], bright: 1.1, vig: 0.5 };
  var finale = { base: "#07051A", c0: "#5B3CF0", c1: "#FF5FA8", c2: "#2E9E5B", c3: "#FFB46B", amt: [0.95, 0.75, 0.6, 0.45], bright: 1.08, vig: 0.55 };
  LQ.moods = [
    { t: 0, mood: Object.assign({}, dawn, dark) },
    { t: 1.1, blend: 1.1, mood: dawn },
    { t: 12.0, blend: 1.2, mood: table },
    { t: 26.8, blend: 1.2, mood: pdf },
    { t: 32.8, blend: 1.2, mood: mark },
    { t: 38.8, blend: 1.2, mood: calm },
    { t: 44.6, blend: 1.0, mood: party },
    { t: 48.5, blend: 1.2, mood: finale },
    { t: 58.6, blend: 0.1, mood: finale },
    { t: 60.0, blend: 1.4, mood: Object.assign({}, finale, dark) },
  ];
})();
