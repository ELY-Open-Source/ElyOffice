import { Fragment } from 'react';
import { interpolate, spring, useCurrentFrame, useVideoConfig } from 'remotion';
import { COLOR, EASE, FONT, SPRING, STAGE } from '../theme';

export type KeyCapProps = {
  // Keys of one chord, e.g. ['Ctrl', '1'], ['Tab'], ['右键'].
  keys: string[];
  // Section-local frames: pops in at `from`, gone by `to`.
  from: number;
  to: number;
  // Section-local frames at which the chord is pressed.
  press?: number[];
  // Centre in composition pixels; default: bottom-right corner of the stage.
  x?: number;
  y?: number;
};

const MOUSE: Record<string, 'left' | 'right' | 'wheel'> = { 右键: 'right', 左键: 'left', 点击: 'left', 滚轮: 'wheel' };

const clamp = { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' } as const;

const Mouse: React.FC<{ part: 'left' | 'right' | 'wheel'; lit: number }> = ({ part, lit }) => {
  const on = `rgba(61,127,229,${(0.55 + 0.45 * lit).toFixed(3)})`;
  return (
    <svg width="18" height="26" viewBox="0 0 18 26">
      <rect x="1" y="1" width="16" height="24" rx="8" fill="none" stroke={COLOR.ink} strokeOpacity="0.85" strokeWidth="1.6" />
      {part === 'left' ? <path d="M9 1.8A7.2 7.2 0 0 0 1.8 9V11H9Z" fill={on} /> : null}
      {part === 'right' ? <path d="M9 1.8A7.2 7.2 0 0 1 16.2 9V11H9Z" fill={on} /> : null}
      <path d="M9 1.8V11M1.8 11H16.2" stroke={COLOR.ink} strokeOpacity="0.5" strokeWidth="1.2" />
      <rect x="7.4" y="4.6" width="3.2" height="5" rx="1.6" fill={part === 'wheel' ? on : COLOR.ink} fillOpacity={part === 'wheel' ? 1 : 0.6} />
    </svg>
  );
};

// A key chord or mouse button that pops, presses, leaves.
export const KeyCap: React.FC<KeyCapProps> = ({ keys, from, to, press = [], x, y }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  if (frame < from || frame >= to) return null;
  const pop = spring({ frame: frame - from, fps, config: SPRING, durationInFrames: 24 });
  const out = interpolate(frame, [to - 14, to], [0, 1], { ...clamp, easing: EASE.in });
  // Down in 4 frames, back up over 12.
  const down = press.reduce((acc, at) => {
    const t = frame - at;
    const v = t < 0 ? 0 : t < 4 ? EASE.out(t / 4) : 1 - EASE.inOut(Math.min(1, (t - 4) / 12));
    return Math.max(acc, v);
  }, 0);
  const anchored = x === undefined || y === undefined;
  const blur = (1 - pop) * 5 + out * 4;
  return (
    <div
      style={{
        position: 'absolute',
        left: x ?? STAGE.x + STAGE.w - 36,
        top: y ?? STAGE.y + STAGE.h - 36,
        transform: `translate(${anchored ? '-100%, -100%' : '-50%, -50%'}) translateY(${((1 - pop) * 12 + out * 6).toFixed(2)}px) scale(${(0.88 + 0.12 * pop - 0.04 * out).toFixed(4)})`,
        transformOrigin: anchored ? '100% 100%' : '50% 50%',
        opacity: Math.min(1, pop * 1.4) * (1 - out),
        filter: blur > 0.05 ? `blur(${blur.toFixed(2)}px)` : undefined,
        display: 'flex',
        alignItems: 'center',
        gap: 10,
      }}
    >
      {keys.map((key, i) => {
        const mouse = MOUSE[key];
        return (
          <Fragment key={`${key}-${i}`}>
            {i > 0 ? (
              <span style={{ fontFamily: FONT.latin, fontSize: 22, fontWeight: 400, color: COLOR.faint }}>+</span>
            ) : null}
            <div
              style={{
                height: 58,
                minWidth: 58,
                padding: '0 18px',
                boxSizing: 'border-box',
                borderRadius: 13,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 10,
                transform: `translateY(${(down * 3).toFixed(2)}px)`,
                background: 'linear-gradient(180deg, #3b3b3e 0%, #2d2d30 100%)',
                border: '1px solid rgba(255,255,255,0.12)',
                boxShadow: [
                  `0 ${(4 - down * 3).toFixed(2)}px 0 #161618`,
                  `0 ${(12 - down * 6).toFixed(2)}px 28px rgba(0,0,0,0.5)`,
                  'inset 0 1px 0 rgba(255,255,255,0.10)',
                  `0 0 0 ${(down * 2).toFixed(2)}px rgba(61,127,229,${(down * 0.7).toFixed(3)})`,
                ].join(', '),
                fontFamily: FONT.latin,
                fontWeight: 600,
                fontSize: 23,
                letterSpacing: '0.01em',
                color: COLOR.ink,
                whiteSpace: 'nowrap',
              }}
            >
              {mouse ? <Mouse part={mouse} lit={down} /> : null}
              <span style={{ fontFamily: mouse ? FONT.sans : FONT.latin, fontWeight: mouse ? 500 : 600 }}>{key}</span>
            </div>
          </Fragment>
        );
      })}
    </div>
  );
};
