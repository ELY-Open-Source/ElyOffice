import { Easing } from 'remotion';

export const WIDTH = 1920;
export const HEIGHT = 1080;
export const FPS = 60;
export const TOTAL = 3600;

// 96 BPM, 4/4.
export const BEAT = 37.5;
export const BAR = 150;
export const beat = (n: number) => Math.round(n * BEAT);
export const bar = (n: number) => n * BAR;

// Section starts (bars 1, 3, 7, 11, 17, 22), lengths.
export const SECTION = {
  intro: { from: 0, duration: 300 },
  menus: { from: 300, duration: 600 },
  cells: { from: 900, duration: 600 },
  rightClick: { from: 1500, duration: 900 },
  more: { from: 2400, duration: 750 },
  outro: { from: 3150, duration: 450 },
} as const;

export const COLOR = {
  bgInner: '#252525',
  bgOuter: '#1c1c1e',
  ink: '#F4F4F2',
  muted: '#A8A6A3',
  faint: '#6E6C6A',
  accent: '#3D7FE5',
  panel: '#1f1f21',
  hairline: 'rgba(255,255,255,0.08)',
} as const;

export type AppId = 'ely-doc' | 'ely-table' | 'ely-show' | 'ely-pdf' | 'ely-tree' | 'ely-mark';

export const APPS: { id: AppId; name: string; color: string }[] = [
  { id: 'ely-doc', name: 'ElyDoc', color: '#3D7FE5' },
  { id: 'ely-table', name: 'ElyTable', color: '#2E9E5B' },
  { id: 'ely-show', name: 'ElyShow', color: '#F07B2A' },
  { id: 'ely-pdf', name: 'ElyPDF', color: '#E0344F' },
  { id: 'ely-tree', name: 'ElyTree', color: '#7E57C2' },
  { id: 'ely-mark', name: 'ElyMark', color: '#3E4A5C' },
];

export const appOf = (id: AppId) => APPS.find((a) => a.id === id)!;

export const STAGE = { x: 120, y: 118, w: 1680, h: 945, r: 18 } as const;
export const STAGE_CENTER = { x: STAGE.x + STAGE.w / 2, y: STAGE.y + STAGE.h / 2 };
// Recordings are 2048×1152; this maps them onto the stage width.
export const FOOTAGE = { w: 2048, h: 1152, scale: STAGE.w / 2048 } as const;

// Boundary icon rest spots and one Papirus 192-grid unit.
export const ICON = { x: 960, y: 590, unit: 1.375 } as const;

// Section-local morph frames: icons rest `hold` beside each cut.
export const MORPH = { hold: 18, morph: 42 } as const;
export const ENTER_END = MORPH.hold + MORPH.morph;
export const EXIT_LEN = MORPH.hold + MORPH.morph;

// Headings enter on beat 1, leave `out` before the end.
export const HEADING = { in: beat(1), sub: beat(0.5), out: 92 } as const;

export const EASE = {
  out: Easing.bezier(0.22, 1, 0.36, 1),
  inOut: Easing.bezier(0.65, 0, 0.35, 1),
  in: Easing.bezier(0.55, 0, 0.75, 0.06),
  // Starts from rest, long deceleration: growth that feels heavy.
  bloom: Easing.bezier(0.5, 0, 0.1, 1),
  // Gentle start, soft landing: for collapses that come to rest.
  land: Easing.bezier(0.6, 0, 0.2, 1),
} as const;

export const SPRING = { damping: 200 } as const;

export const FONT = {
  sans: '"Noto Sans SC", "Inter", sans-serif',
  latin: '"Inter", "Noto Sans SC", sans-serif',
  mono: '"JetBrains Mono", monospace',
} as const;
