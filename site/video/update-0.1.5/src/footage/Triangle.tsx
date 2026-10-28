import React from 'react';
import { interpolate } from 'remotion';
import { COLOR, EASE, FONT } from '../theme';
import { pointerAt, TAKES, type TakeName } from './take';

type Props = {
  take: TakeName;
  t: number;
  /** Footage seconds the pointer crosses toward the submenu. */
  from: number;
  to: number;
  /** The row it crosses without opening (y0..y1), and its note. */
  passed: [number, number];
  note: string;
};

const clamp = { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' } as const;

/** The safe triangle: pointer to the submenu's near corners, live. */
export const Triangle: React.FC<Props> = ({ take, t, from, to, passed, note }) => {
  const menu = TAKES[take].menus;
  if (!menu) throw new Error(`${take} has no submenu rectangle`);
  const show = interpolate(t, [from - 0.35, from, to + 0.25, to + 0.7], [0, 1, 1, 0], { ...clamp, easing: EASE.inOut });
  if (show <= 0) return null;
  const p = pointerAt(take, Math.min(t, to + 0.25));
  const [x0, y0, , y1] = menu.panel;
  const [px0, , px1] = menu.parent;
  const drawn = interpolate(t, [from - 0.35, from + 0.1], [0, 1], { ...clamp, easing: EASE.out });
  const near = { x: x0 + 2, top: y0 + 6, bottom: y0 + 6 + (y1 - y0 - 12) * drawn };
  const row = interpolate(t, [from + 0.15, from + 0.45], [0, 1], { ...clamp, easing: EASE.out });
  return (
    <div style={{ position: 'absolute', inset: 0, opacity: show, pointerEvents: 'none' }}>
      <svg width={2048} height={1152} style={{ position: 'absolute', inset: 0, overflow: 'visible' }}>
        <defs>
          <linearGradient id="safe" x1="0" x2="1" y1="0" y2="0">
            <stop offset="0" stopColor={COLOR.accent} stopOpacity={0.05} />
            <stop offset="1" stopColor={COLOR.accent} stopOpacity={0.3} />
          </linearGradient>
        </defs>
        <path
          d={`M ${p.x} ${p.y} L ${near.x} ${near.top} L ${near.x} ${near.bottom} Z`}
          fill="url(#safe)"
          stroke={COLOR.accent}
          strokeWidth={2.5}
          strokeLinejoin="round"
          strokeDasharray="10 7"
        />
        <circle cx={p.x} cy={p.y} r={7} fill={COLOR.accent} />
        <rect
          x={px0 + 6}
          y={passed[0]}
          width={(px1 - px0 - 12) * row}
          height={passed[1] - passed[0]}
          rx={8}
          fill="none"
          stroke="#F2A900"
          strokeWidth={2.5}
          opacity={0.9}
        />
      </svg>
      <div
        style={{
          position: 'absolute',
          right: 2048 - px0 + 22,
          top: (passed[0] + passed[1]) / 2 - 24,
          opacity: row,
          transform: `translateX(${(1 - row) * 16}px)`,
          padding: '8px 16px',
          borderRadius: 12,
          background: 'rgba(28,28,30,0.92)',
          border: '1.5px solid #F2A90088',
          color: '#F7E2A6',
          fontFamily: FONT.sans,
          fontSize: 26,
          fontWeight: 500,
          whiteSpace: 'nowrap',
          boxShadow: '0 10px 30px rgba(0,0,0,0.45)',
        }}
      >
        {note}
      </div>
    </div>
  );
};
