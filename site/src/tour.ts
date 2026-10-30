// The camera glides from the desk into each app.
import { block, project, stage, type Block } from "./iso.ts";
import { grow } from "./beat.ts";
import { model } from "./model.ts";
import { touch } from "./touch.ts";
import { said, TEXT, type Key } from "./text.ts";
import { RELEASES } from "./release.ts";

const SCREEN = "#3a3a3c";
const DESK = "#e6e6e2";
const STEP = 60 / 112 / 4;
const ROWS = 4;
/** Share of each span the camera holds still, per side. */
const HOLD = 0.35;
const CLIP = [320, 180];
const NARROW = 760;
const TOP = model[0].blocks[0].h;

const apps = model.slice(1, -1);
const last = apps.length + 1;

/** Each app's screen: corner x, y, post height, width. */
const SPOTS: Record<string, [number, number, number, number]> = {
  ElyDoc: [0.1, 0, 0.5, 4],
  ElyTable: [4.4, 0, 1.6, 4],
  ElyShow: [10.8, 0.5, 2.2, 4],
  ElyPDF: [12, 2.3, 0.3, 4],
  ElyMail: [8.6, 6.6, 1, 4],
  ElyTree: [5.9, 5.6, 0, 4],
  ElyMark: [0.7, 4.3, 0.1, 4],
};

/** An app's post, then its screen rows, bottom up. */
export function slab(name: string): Required<Block>[] {
  const spot = SPOTS[name];
  if (!spot) throw new Error(`no screen for ${name}`);
  const [x, y, post, w] = spot;
  const d = 0.2;
  const row = (w * 9) / 16 / ROWS;
  const out: Required<Block>[] = [];
  if (post > 0) out.push({ x: x + w / 2 - 0.1, y, z: TOP, w: 0.2, d, h: post, tone: SCREEN });
  // Each row sits on the last; sums keep faces flush.
  let z = TOP + post;
  for (let i = 0; i < ROWS; i++) {
    out.push({ x, y, z, w, d, h: row, tone: SCREEN });
    z += row;
  }
  return out;
}

type Box = [number, number, number, number];

/** Screen bounds of blocks, in SVG units. */
function bounds(blocks: Required<Block>[]): Box {
  const points = blocks.flatMap((b) =>
    [0, 1].flatMap((i) => [0, 1].flatMap((j) => [0, 1].map((k) => project(b.x + i * b.w, b.y + j * b.d, b.z + k * b.h)))),
  );
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
}

/** World point c drawn at screen point a, scale s. */
interface Cam {
  s: number;
  cx: number;
  cy: number;
  ax: number;
  ay: number;
}

/** Fits `box` into the screen rectangle `area`. */
function fit(box: Box, area: Box, most: number): Cam {
  const s = Math.min((area[2] - area[0]) / (box[2] - box[0]), (area[3] - area[1]) / (box[3] - box[1]), most);
  return { s, cx: (box[0] + box[2]) / 2, cy: (box[1] + box[3]) / 2, ax: (area[0] + area[2]) / 2, ay: (area[1] + area[3]) / 2 };
}

const ease = (t: number) => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2);
const mix = (a: number, b: number, t: number) => a + (b - a) * t;

function between(a: Cam, b: Cam, t: number): Cam {
  return {
    s: Math.exp(mix(Math.log(a.s), Math.log(b.s), t)),
    cx: mix(a.cx, b.cx, t),
    cy: mix(a.cy, b.cy, t),
    ax: mix(a.ax, b.ax, t),
    ay: mix(a.ay, b.ay, t),
  };
}

/** Maps a clip's box onto the screen's left face. */
function face(rows: Required<Block>[], origin: [number, number]): string {
  const [first, top] = [rows[0], rows[rows.length - 1]];
  const y = first.y + first.d;
  const z = top.z + top.h;
  const corner = project(first.x, y, z);
  const across = project(first.x + first.w, y, z);
  const down = project(first.x, y, first.z);
  const m = [
    (across[0] - corner[0]) / CLIP[0],
    (across[1] - corner[1]) / CLIP[0],
    (down[0] - corner[0]) / CLIP[1],
    (down[1] - corner[1]) / CLIP[1],
    corner[0] - origin[0],
    corner[1] - origin[1],
  ];
  return `matrix(${m.join(", ")})`;
}

/** A clip that loads only while its app is current. */
function clip(name: string, transform: string, lands: number): HTMLVideoElement {
  const video = document.createElement("video");
  video.className = "clip";
  video.muted = true;
  video.defaultMuted = true;
  video.loop = true;
  video.playsInline = true;
  video.preload = "none";
  video.dataset.src = `/clips/${name.slice(3).toLowerCase()}`;
  video.style.transform = transform;
  video.style.setProperty("--at", `${lands.toFixed(3)}s`);
  video.setAttribute("aria-hidden", "true");
  return video;
}

function start(video: HTMLVideoElement): void {
  if (!video.querySelector("source")) {
    for (const kind of ["webm", "mp4"]) {
      const source = document.createElement("source");
      source.src = `${video.dataset.src}.${kind}`;
      source.type = `video/${kind}`;
      video.append(source);
    }
  }
  video.play().catch((e: DOMException) => {
    // A pause during loading interrupts play; that is expected.
    if (e.name !== "AbortError") throw e;
  });
}

function stop(video: HTMLVideoElement): void {
  if (!video.querySelector("source")) return;
  video.pause();
  video.replaceChildren();
  video.load();
}

/** A rail cube for one app. */
function cube(): SVGSVGElement {
  const svg = stage([block({ x: 0, y: 0, w: 1, d: 1, h: 0.5, tone: DESK })]);
  svg.setAttribute("aria-hidden", "true");
  return svg;
}

const lang = () => (document.documentElement.lang.startsWith("zh") ? "zh" : "en");

/** Sets a button's keyed text now and on language change. */
function label(el: HTMLElement, key: Key): void {
  el.dataset.t = key;
  el.textContent = TEXT[lang()][key];
}

/** The pinned tour, starting with `hero` over the whole desk. */
export function tour(hero: HTMLElement): { section: HTMLElement; host: HTMLElement; svg: SVGSVGElement } {
  const calm = matchMedia("(prefers-reduced-motion: reduce)");
  const still = () => calm.matches;
  const built = model.map((part) => part.blocks.map(block));
  grow(built);
  const screens = apps.map((p) => slab(p.name));
  const shown = screens.map((list, a) =>
    list.map((b, i) => {
      const g = block(b);
      g.classList.add("slab");
      g.dataset.app = String(a);
      g.dataset.row = String(i - list.length + ROWS);
      g.style.setProperty("--at", `${(i * STEP).toFixed(3)}s`);
      return g;
    }),
  );
  const svg = stage([...built.flat(), ...shown.flat()]);
  svg.classList.add("grow", "seen");
  svg.setAttribute("aria-hidden", "true");
  const view = svg.viewBox.baseVal;
  // Exact size keeps clips on their faces when zoomed.
  svg.style.width = `${view.width}px`;
  svg.style.height = `${view.height}px`;
  const origin: [number, number] = [view.x, view.y];

  const world = document.createElement("div");
  world.className = "world";
  world.append(svg);
  const host = document.createElement("div");
  host.className = "model";
  host.append(world);

  // Screens stay out of the hit layer.
  const marks = shown.flat().map((g) => {
    const mark = document.createComment("");
    g.replaceWith(mark);
    return [mark, g] as const;
  });
  touch(host, svg, new Map(apps.map((p, i) => [p.name, built[i + 1]])));
  for (const [mark, g] of marks) mark.replaceWith(g);

  const clips = apps.map((p, i) => {
    const rows = screens[i].filter((b) => b.w > 0.2);
    return clip(p.name, face(rows, origin), shown[i].length * STEP);
  });
  world.append(...clips);

  const plays = clips.map((video) => {
    const button = document.createElement("button");
    button.className = "toggle-clip";
    const sync = () => label(button, video.paused ? "clipPlay" : "clipPause");
    video.addEventListener("play", sync);
    video.addEventListener("pause", sync);
    button.addEventListener("click", () => (video.paused ? start(video) : video.pause()));
    video.addEventListener("click", () => (video.paused ? start(video) : video.pause()));
    sync();
    return button;
  });

  const copy = document.createElement("div");
  copy.className = "copy";
  const heads: HTMLElement[] = [];
  const articles = apps.map((p, i) => {
    const article = document.createElement("article");
    const h2 = document.createElement("h2");
    h2.textContent = p.name;
    h2.tabIndex = -1;
    heads.push(h2);
    const get = said("a", "download");
    get.className = "get";
    get.href = RELEASES;
    const acts = document.createElement("div");
    acts.className = "acts";
    acts.append(get, plays[i]);
    article.append(h2, said("p", p.name as Key), acts);
    return article;
  });
  copy.append(...articles);

  const rail = document.createElement("div");
  rail.className = "rail";
  const steps = apps.map((p) => {
    const a = document.createElement("a");
    a.href = `#${p.name.toLowerCase()}`;
    a.setAttribute("aria-label", p.name);
    a.append(cube());
    return a;
  });
  rail.append(...steps);

  const pin = document.createElement("div");
  pin.className = "pin";
  hero.classList.add("hero");
  pin.append(hero, host, copy, rail);
  const section = document.createElement("section");
  section.className = "tour";
  const stops = apps.map((p, i) => {
    const stop = document.createElement("div");
    stop.id = p.name.toLowerCase();
    stop.className = "stop";
    stop.style.top = `calc(var(--span) * ${i + 1})`;
    return stop;
  });
  section.append(pin, ...stops);

  const desk = bounds(model.flatMap((p) => p.blocks));
  const homes = apps.map((p, i) => bounds([...p.blocks, ...screens[i]]));
  let top = 0;
  let span = 1;
  let cams: Cam[] = [];
  let current = -1;
  let queued = false;

  const measure = () => {
    const [w, h] = [pin.clientWidth, pin.clientHeight];
    top = section.getBoundingClientRect().top + scrollY;
    span = h;
    const below = hero.offsetTop + hero.offsetHeight + 32;
    const whole = fit(desk, [24, below, w - 24, h - 32], 2);
    // Copy below the model only on a narrow upright screen.
    const upright = w <= NARROW && h >= w;
    const area: Box = upright ? [16, h * 0.04, w - 52, h * 0.56] : [w * 0.42, h * 0.08, w - 88, h * 0.92];
    cams = [whole, ...homes.map((box) => fit(box, area, 8)), whole];
  };

  const choose = (next: number) => {
    if (next === current) return;
    if (current >= 0) {
      for (const g of shown[current]) g.classList.remove("up");
      clips[current].classList.remove("up");
      stop(clips[current]);
      articles[current].classList.remove("current");
      steps[current].removeAttribute("aria-current");
    }
    current = next;
    pin.classList.toggle("touring", current >= 0);
    if (current < 0) return;
    for (const g of shown[current]) g.classList.add("up");
    for (const i of [current - 1, current, current + 1]) {
      if (clips[i] && !clips[i].poster) clips[i].poster = `${clips[i].dataset.src}.jpg`;
    }
    clips[current].classList.add("up");
    articles[current].classList.add("current");
    if (current === arriving) land();
    steps[current].setAttribute("aria-current", "step");
    if (!still() && !document.hidden) start(clips[current]);
  };

  const render = () => {
    queued = false;
    // The first measure comes with the first layout.
    if (!cams.length) return;
    const p = Math.min(Math.max((scrollY - top) / span, 0), last);
    const k = Math.min(Math.floor(p), last - 1);
    const f = p - k;
    let cam: Cam;
    if (still()) cam = cams[Math.round(p)];
    // Leaving the hero, the desk moves at once.
    else if (k === 0 && f < 1 - HOLD) cam = between(cams[0], cams[1], ease(f / (1 - HOLD)));
    else if (f <= HOLD) cam = cams[k];
    else if (f >= 1 - HOLD) cam = cams[k + 1];
    else cam = between(cams[k], cams[k + 1], ease((f - HOLD) / (1 - 2 * HOLD)));
    const tx = cam.ax - cam.s * (cam.cx - origin[0]);
    const ty = cam.ay - cam.s * (cam.cy - origin[1]);
    world.style.transform = `matrix(${cam.s}, 0, 0, ${cam.s}, ${tx}, ${ty})`;
    host.dispatchEvent(new Event("moved"));
    const fade = still() ? (p < 0.5 ? 1 : 0) : Math.max(0, 1 - p / HOLD);
    hero.style.opacity = String(fade);
    hero.style.visibility = fade > 0 ? "" : "hidden";
    hero.style.translate = still() ? "" : `0 ${(-p * span * 0.3).toFixed(1)}px`;
    const at = Math.round(p);
    choose(at >= 1 && at < last ? at - 1 : -1);
  };

  // A link's arrival hands Tab to its region.
  let arriving = -1;
  const land = () => {
    heads[arriving].focus({ preventScroll: true });
    arriving = -1;
  };
  const arrive = () => {
    arriving = apps.findIndex((p) => `#${p.name.toLowerCase()}` === location.hash);
    if (arriving >= 0 && arriving === current) land();
  };
  addEventListener("hashchange", arrive);
  arrive();

  const later = () => {
    if (queued) return;
    queued = true;
    requestAnimationFrame(render);
  };
  addEventListener("scroll", later, { passive: true });
  const sizes = new ResizeObserver(() => {
    measure();
    later();
  });
  sizes.observe(pin);
  sizes.observe(hero);
  document.addEventListener("visibilitychange", () => {
    if (current < 0) return;
    if (document.hidden) clips[current].pause();
    else if (!still()) start(clips[current]);
  });
  return { section, host, svg };
}
