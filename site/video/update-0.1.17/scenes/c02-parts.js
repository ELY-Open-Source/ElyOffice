// c02 geometry: the box, the Linux files, the setup rows.
import { kit } from '../lib/kit/index.js';

export const SUITE = ['doc', 'table', 'show', 'pdf', 'mail', 'tree', 'mark'];
export const NAMES = ['ElyDoc', 'ElyTable', 'ElyShow', 'ElyPDF', 'ElyMail', 'ElyTree', 'ElyMark'];

export const BOX = { x: 500, rim: 330, w: 236, h: 140, dx: 54, dy: -40 };
export const ROW = { x: 500, y: 118, pitch: 124, size: 92 };
export const WIN = { x: 1130, y: 150, w: 620, h: 740 };
export const LIST = { y: 284, pitch: 82, box: 34, size: 54 };

const P = (o, x, y) => [o[0] + x, o[1] + y];

// An open carton in one stroke, as the macOS package.
export function box(squash = 0) {
  const { x, rim, w, h, dx, dy } = BOX;
  const lift = rim + squash;
  const fl = [x - w / 2, lift];
  const fr = [x + w / 2, lift];
  const bl = P(fl, dx, dy);
  const br = P(fr, dx, dy);
  const bottom = rim + h;
  const lf = P(fl, -58, -24);
  const lb = P(bl, -58, -24);
  const rf = P(fr, 60, 28);
  const rb = P(br, 60, 28);
  const fb = [fl[0], bottom];
  const rbot = [fr[0], bottom];
  const sb = [br[0], bottom + dy];
  return [kit.poly([br, P(br, 16, -64), P(bl, 16, -64), bl, lb, lf, fl, bl, br, rb, rf, fr, br, sb, rbot, fb, fl, fr, rbot], { w: 1.15 })];
}

// A page with a folded corner and its badge.
function page(cx, cy, kind) {
  const w = 120;
  const h = 150;
  const x = cx - w / 2;
  const y = cy - h / 2;
  const f = 34;
  const o = { w: 1.1 };
  const out = [
    kit.poly([[x + w - f, y], [x, y], [x, y + h], [x + w, y + h], [x + w, y + f], [x + w - f, y]], o),
    kit.poly([[x + w - f, y], [x + w - f, y + f], [x + w, y + f]], { w: 0.8 }),
  ];
  const bx = cx;
  const by = cy + 16;
  const r = 30;
  if (kind === 'deb') {
    const hex = [0, 1, 2, 3, 4, 5].map((i) => [bx + r * Math.cos(-Math.PI / 2 + (i * Math.PI) / 3), by + r * Math.sin(-Math.PI / 2 + (i * Math.PI) / 3)]);
    out.push(kit.poly([hex[0], hex[1], hex[2], hex[3], hex[4], hex[5], hex[0]], o));
    out.push(kit.poly([hex[5], [bx, by], hex[1]], { w: 0.8 }), kit.poly([[bx, by], hex[3]], { w: 0.8 }));
  } else {
    out.push(kit.rect(bx - r, by - r, r * 2, r * 2, { r: 12, w: 1.1 }));
    out.push(kit.poly([[bx - 12, by - 2], [bx, by + 12], [bx + 12, by - 2]], { w: 0.9 }), kit.poly([[bx, by - 16], [bx, by + 12]], { w: 0.9 }));
  }
  return out;
}

export function linux() {
  const y = 772;
  const name = { y: 912, size: 30, align: 'center', w: 1.05 };
  return [
    ...page(300, y, 'deb'),
    ...kit.text('ely-office', { ...name, x: 300 }),
    ...page(720, y, 'app'),
    ...kit.text('ElyOffice.AppImage', { ...name, x: 720 }),
  ];
}

export function macName() {
  return kit.text('ElyOffice.pkg', { x: BOX.x + 20, y: 560, size: 32, align: 'center', w: 1.05 });
}

export function setup() {
  return kit.window(WIN.x, WIN.y, WIN.w, WIN.h, { chrome: 'windows', title: 'Ely Office Setup', titleSize: 30, w: 1.1 });
}

export const rowY = (i) => LIST.y + LIST.pitch * i;
export const iconX = () => WIN.x + 64 + LIST.box + 58;

// One row: checkbox, tick, icon, name; sx flips the icon.
export function row(i, sx = 1) {
  const y = rowY(i);
  const bx = WIN.x + 64;
  const [square, tick] = kit.check(bx, y - LIST.box / 2, LIST.box, 1, { w: 1.1 });
  const ix = iconX();
  const icon = kit.icon(SUITE[i], ix, y, LIST.size, { app: SUITE[i], w: 1.1 }).map((st) => ({ ...st, pts: st.pts.map(([px, py]) => [ix + (px - ix) * sx, py]) }));
  const name = kit.text(NAMES[i], { x: ix + 54, y: y + 12, size: 34, w: 1.1 });
  return { square: { ...square, link: 0.6 }, tick: { ...tick, w: 1.4 }, icon, name };
}

export function version(u) {
  const at = { x: WIN.x + WIN.w - 36, y: WIN.y + WIN.h - 34, size: 30, align: 'right', w: 1.05 };
  const from = kit.text('0.1.16', at);
  const to = kit.text('0.1.17', at);
  let j = 0;
  const seven = to.filter((s) => s.g === 5);
  return from.map((s) => (s.g === 5 ? kit.morph(s, seven[j++], u) : s));
}
