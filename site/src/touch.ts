// Buildings lift on hover or focus and sink when pressed.
import { UNIT } from "./iso.ts";

const SVG = "http://www.w3.org/2000/svg";

/** Links each app's blocks to its region, in reading order. */
export function touch(host: HTMLElement, svg: SVGSVGElement, parts: Map<string, SVGGElement[]>): void {
  host.style.setProperty("--step", `${UNIT}px`);
  const nav = document.createElement("nav");
  nav.className = "doors";
  host.append(nav);

  const owner = new Map([...parts].flatMap(([name, blocks]) => blocks.map((b) => [b, name] as const)));
  const { layer, hits } = still(svg, owner);
  opens(layer);

  for (const [name, blocks] of parts) {
    const target = `#${name.toLowerCase()}`;
    const state = { hover: false, focus: false, press: false };
    const hit = hits.filter((h) => h.dataset.part === name && !h.classList.contains("lift"));
    const lifts = hits.filter((h) => h.dataset.part === name && h.classList.contains("lift"));
    const caption = document.createElement("p");
    caption.className = "caption";
    caption.textContent = name;
    caption.setAttribute("aria-hidden", "true");
    host.append(caption);

    const follow = () => place(caption, host, svg, hit);
    const sized = new ResizeObserver(follow);
    const show = () => {
      const up = state.hover || state.focus;
      for (const b of blocks) {
        b.classList.toggle("lifted", up);
        b.classList.toggle("pressed", up && state.press);
      }
      caption.classList.toggle("shown", up);
      for (const l of lifts) l.classList.toggle("on", up);
      // The camera announces each move on the host.
      if (up) {
        host.addEventListener("moved", follow);
        sized.observe(svg);
        follow();
      } else {
        host.removeEventListener("moved", follow);
        sized.disconnect();
      }
    };
    const set = (change: Partial<typeof state>) => {
      Object.assign(state, change);
      show();
    };

    const own = (node: EventTarget | null) => node instanceof SVGElement && node.dataset.part === name;
    for (const h of [...hit, ...lifts]) {
      h.addEventListener("pointerenter", () => set({ hover: true }));
      h.addEventListener("pointerleave", (e) => own(e.relatedTarget) || set({ hover: false, press: false }));
      h.addEventListener("pointerdown", (e) => e.button === 0 && set({ press: true }));
      h.addEventListener("pointerup", () => set({ press: false }));
      h.addEventListener("pointercancel", () => set({ press: false }));
    }

    const door = document.createElement("a");
    door.href = target;
    door.textContent = name;
    door.addEventListener("focus", () => set({ focus: true }));
    door.addEventListener("blur", () => set({ focus: false, press: false }));
    door.addEventListener("keydown", (e) => e.key === "Enter" && !e.repeat && set({ press: true }));
    door.addEventListener("keyup", (e) => e.key === "Enter" && set({ press: false }));
    nav.append(door);
  }
}

/** Opens the app a primary press starts and ends on. */
function opens(layer: SVGGElement): void {
  const part = (node: EventTarget | null) => (node instanceof SVGElement ? (node.dataset.part ?? "") : "");
  let pressed = "";
  // Any press elsewhere forgets the last one.
  addEventListener("pointerdown", () => (pressed = ""), true);
  layer.addEventListener("pointerdown", (e) => (pressed = e.button === 0 ? part(e.target) : ""));
  layer.addEventListener("pointercancel", () => (pressed = ""));
  layer.addEventListener("pointerup", (e) => {
    if (e.button === 0 && pressed && part(e.target) === pressed) location.hash = `#${pressed.toLowerCase()}`;
    pressed = "";
  });
}

/** Resting and lifted outlines over the model, layered as drawn. */
function still(svg: SVGSVGElement, owner: Map<SVGGElement, string>): { layer: SVGGElement; hits: SVGPolygonElement[] } {
  const layer = document.createElementNS(SVG, "g");
  layer.setAttribute("class", "hits");
  const hits: SVGPolygonElement[] = [];
  for (const b of svg.querySelectorAll<SVGGElement>(".block")) {
    const outline = b.querySelector(".outline");
    if (!outline) throw new Error("block has no outline");
    const name = owner.get(b);
    b.dataset.part = name ?? "";
    const face = outline.cloneNode() as SVGPolygonElement;
    face.setAttribute("class", name ? "hit-face" : "hit-face ground");
    face.dataset.part = name ?? "";
    layer.append(face);
    if (!name) continue;
    // Where a lifted block draws; front blocks still cover it.
    const lift = outline.cloneNode() as SVGPolygonElement;
    lift.setAttribute("class", "hit-face lift");
    lift.dataset.part = name;
    layer.append(lift);
    hits.push(face, lift);
  }
  svg.append(layer);
  return { layer, hits };
}

/** Puts the caption one cell above the lifted top. */
function place(caption: HTMLElement, host: HTMLElement, svg: SVGSVGElement, faces: SVGPolygonElement[]) {
  const origin = host.getBoundingClientRect();
  const ctm = svg.getScreenCTM();
  if (!ctm) throw new Error("model is not rendered");
  // Hit faces never lift, so they give the rest pose.
  const rects = faces.map((f) => f.getBoundingClientRect());
  const left = Math.min(...rects.map((r) => r.left));
  const right = Math.max(...rects.map((r) => r.right));
  const top = Math.min(...rects.map((r) => r.top));
  caption.style.left = `${(left + right) / 2 - origin.left}px`;
  caption.style.top = `${top - origin.top - 2 * UNIT * ctm.d}px`;
}
