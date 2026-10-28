/* Aurora mood keyframes, global seconds; each blends in before t. */
(function () {
  var dark = { bright: 0.0 };
  var dawn = { base: "#07051A", c0: "#5B3CF0", c1: "#1E8FE0", c2: "#FF4FA3", c3: "#2FD8E8", amt: [0.85, 0.7, 0.45, 0.4], bright: 1, vig: 0.6 };
  var mail = { base: "#050B22", c0: "#2A5BFF", c1: "#1E8FE0", c2: "#7B5CFF", c3: "#2FD8E8", amt: [0.9, 0.85, 0.5, 0.55], bright: 1.05, vig: 0.55 };
  var calm = { base: "#090722", c0: "#5B3CF0", c1: "#3A7BFF", c2: "#C04FD8", c3: "#2FD8E8", amt: [0.8, 0.6, 0.45, 0.35], bright: 0.95, vig: 0.6 };
  var warm = { base: "#0C0620", c0: "#7B3CF0", c1: "#FF5FA8", c2: "#FFB46B", c3: "#2FD8E8", amt: [0.85, 0.6, 0.5, 0.35], bright: 1.05, vig: 0.55 };
  var stat = { base: "#060318", c0: "#8A4BFF", c1: "#2FD8E8", c2: "#FF4FA3", c3: "#B8F35A", amt: [0.95, 0.8, 0.6, 0.3], bright: 1.12, vig: 0.5 };
  var party = { base: "#0A0620", c0: "#6E44FF", c1: "#FF5FA8", c2: "#2FD8E8", c3: "#FFB46B", amt: [0.95, 0.8, 0.65, 0.5], bright: 1.1, vig: 0.5 };
  var finale = { base: "#07051A", c0: "#5B3CF0", c1: "#FF5FA8", c2: "#1E8FE0", c3: "#FFB46B", amt: [0.95, 0.75, 0.6, 0.45], bright: 1.08, vig: 0.55 };
  LQ.moods = [
    { t: 0, mood: Object.assign({}, dawn, dark) },
    { t: 1.1, blend: 1.1, mood: dawn },
    { t: 12.0, blend: 1.2, mood: mail },
    { t: 27.0, blend: 2.0, mood: calm },
    { t: 35.0, blend: 2.0, mood: warm },
    { t: 41.8, blend: 0.6, mood: Object.assign({}, warm, { bright: 0.7, amt: [0.5, 0.4, 0.3, 0.2] }) },
    { t: 42.8, blend: 0.4, mood: warm },
    { t: 45.0, blend: 1.4, mood: calm },
    { t: 47.0, blend: 0.4, mood: stat },
    { t: 50.5, blend: 1.4, mood: mail },
    { t: 60.5, blend: 1.2, mood: party },
    { t: 78.5, blend: 1.2, mood: finale },
    { t: 88.6, blend: 0.1, mood: finale },
    { t: 90.0, blend: 1.4, mood: Object.assign({}, finale, dark) },
  ];
})();
