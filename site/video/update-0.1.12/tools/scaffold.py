"""Writes index.html and (unless they exist) the scene stubs. Run from the project root."""
import os

SCENES = [("s1", 0, 12), ("s2", 12, 14), ("s3", 26, 6), ("s4", 32, 6), ("s5", 38, 6), ("s6", 44, 4), ("s7", 48, 12)]
DUR = 60

STUB = """<!doctype html>
<html>
<head><meta charset="UTF-8" /></head>
<body>
<template>
<style>
#{sid}-root {{ position: absolute; inset: 0; }}
</style>
<div id="{sid}-root" data-composition-id="{sid}" data-width="1920" data-height="1080" data-duration="{dur}"></div>
<script>
(function () {{
  var tl = gsap.timeline({{ paused: true }});
  window.__timelines["{sid}"] = tl;
}})();
</script>
</template>
</body>
</html>
"""

INDEX = """<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=1920, height=1080" />
<title>Ely Office 0.1.12</title>
<script src="vendor/gsap.min.js"></script>
<link rel="stylesheet" href="lib/ui.css" />
<style>
@font-face {{ font-family: "Inter"; src: url("assets/fonts/Inter.ttf"); font-weight: 100 900; }}
@font-face {{ font-family: "Noto Sans SC"; src: url("assets/fonts/NotoSansSC.ttf"); font-weight: 100 900; }}
@font-face {{ font-family: "JetBrains Mono"; src: url("assets/fonts/JetBrainsMono.ttf"); font-weight: 100 900; }}
* {{ margin: 0; padding: 0; box-sizing: border-box; }}
html, body {{ width: 1920px; height: 1080px; overflow: hidden; background: #08061a; }}
#root {{ position: relative; width: 100%; height: 100%; overflow: hidden; font-family: "Inter", "Noto Sans SC", sans-serif; }}
#lq-back, #lq-front {{ position: absolute; left: 0; top: 0; width: 1920px; height: 1080px; display: block; }}
#lq-back {{ z-index: 0; }}
.scene-host {{ position: absolute; inset: 0; z-index: 10; }}
#lq-front {{ z-index: 30; pointer-events: none; }}
#cap {{ z-index: 40; }}
#cap-pill {{ position: relative; height: 106px; width: 300px; border-radius: 999px; overflow: hidden;
  background: linear-gradient(160deg, rgba(40, 30, 96, 0.55), rgba(14, 10, 44, 0.55));
  border: 1px solid rgba(255, 255, 255, 0.26);
  -webkit-backdrop-filter: blur(26px) saturate(170%); backdrop-filter: blur(26px) saturate(170%);
  box-shadow: 0 18px 50px rgba(3, 2, 18, 0.5), inset 0 1px 0 rgba(255, 255, 255, 0.4), inset 0 -10px 24px rgba(120, 90, 255, 0.12); }}
#cap-pill::before {{ content: ""; position: absolute; inset: 0; border-radius: inherit; background: radial-gradient(90% 120% at 20% -30%, rgba(255,255,255,.28), transparent 60%); }}
.cap-line {{ position: absolute; left: 0; right: 0; top: 0; bottom: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 4px; white-space: nowrap; }}
.cap-zh {{ font: 600 35px "Noto Sans SC", sans-serif; color: #fff; letter-spacing: 0.01em; text-shadow: 0 2px 12px rgba(0,0,0,.35); }}
.cap-en {{ font: 500 23px "Inter", sans-serif; color: rgba(236, 238, 255, 0.82); letter-spacing: 0.005em; }}
.cap-measure {{ position: absolute; left: -9999px; top: 0; display: inline-flex; flex-direction: column; align-items: center; white-space: nowrap; }}
</style>
</head>
<body>
<div id="root" data-composition-id="main" data-start="0" data-duration="{dur}" data-width="1920" data-height="1080">
  <canvas id="lq-back" width="1920" height="1080"></canvas>
{hosts}
  <canvas id="lq-front" width="1920" height="1080"></canvas>
  <div id="cap"><div id="cap-pill"></div></div>
  <div id="driver" class="clip" data-start="0" data-duration="{dur}" data-track-index="0" style="position:absolute;width:1px;height:1px;opacity:0"></div>
{audio}
</div>
<script src="lib/liquid.js"></script>
<script src="lib/moods.js"></script>
<script src="lib/kit.js"></script>
<script src="lib/captions.js"></script>
<script src="lib/flow.js"></script>
{scripts}
<script>
{main}
</script>
</body>
</html>
"""


def main():
    os.makedirs("scenes", exist_ok=True)
    for sid, start, dur in SCENES:
        for name, body in (
            (f"scenes/{sid}.html", STUB.format(sid=sid, dur=dur)),
            (f"scenes/{sid}.liquid.js", f'/* {sid}: liquid choreography, global {start}s to {start + dur}s. */\nLQ.add({{ id: "{sid}", start: {start}, end: {start + dur}, build: function (lt, P, t) {{}} }});\n'),
            (f"scenes/{sid}.cues.json", "[]\n"),
        ):
            if not os.path.exists(name):
                with open(name, "w", encoding="utf-8") as f:
                    f.write(body)
    hosts = "\n".join(
        f'  <div id="{sid}" data-composition-id="{sid}" data-composition-src="scenes/{sid}.html" data-start="{start}" data-duration="{dur}" data-track-index="{i + 1}" data-width="1920" data-height="1080" class="scene-host"></div>'
        for i, (sid, start, dur) in enumerate(SCENES)
    )
    scripts = "\n".join(f'<script src="scenes/{sid}.liquid.js"></script>' for sid, _, _ in SCENES)
    audio = ""
    if os.path.exists("assets/audio/mix.wav"):
        audio = '  <audio id="mix" src="assets/audio/mix.wav" data-start="0" data-duration="60" data-track-index="20" data-volume="1"></audio>'
    with open("index.html", "w", encoding="utf-8") as f:
        main_js = open("lib/main.js", encoding="utf-8").read()
        f.write(INDEX.format(hosts=hosts, scripts=scripts, audio=audio, main=main_js, dur=DUR))


main()
