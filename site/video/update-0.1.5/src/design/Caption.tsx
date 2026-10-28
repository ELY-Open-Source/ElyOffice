import { interpolate, useCurrentFrame } from 'remotion';
import { COLOR, EASE, FONT, STAGE } from '../theme';

export type CaptionProps = {
  text: string;
  // Section-local frames: starts entering at `from`, gone by `to`.
  from: number;
  to: number;
  // Pill centre in composition pixels; default bottom centre.
  x?: number;
  y?: number;
};

const clamp = { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' } as const;

// A glass pill with one line of explanation.
export const Caption: React.FC<CaptionProps> = ({ text, from, to, x = STAGE.x + STAGE.w / 2, y = STAGE.y + STAGE.h - 64 }) => {
  const frame = useCurrentFrame();
  if (frame < from || frame >= to) return null;
  const p = interpolate(frame, [from, from + 22], [0, 1], { ...clamp, easing: EASE.out });
  const q = interpolate(frame, [to - 16, to], [0, 1], { ...clamp, easing: EASE.in });
  const blur = (1 - p) * 6 + q * 5;
  return (
    <div
      style={{
        position: 'absolute',
        left: x,
        top: y,
        transform: `translate(-50%, -50%) translateY(${((1 - p) * 14 - q * 8).toFixed(2)}px) scale(${(0.98 + 0.02 * p).toFixed(4)})`,
        opacity: p * (1 - q),
        filter: blur > 0.05 ? `blur(${blur.toFixed(2)}px)` : undefined,
        padding: '13px 26px 14px',
        borderRadius: 16,
        background: 'rgba(22,22,24,0.66)',
        backdropFilter: 'blur(18px) saturate(1.2)',
        border: '1px solid rgba(255,255,255,0.09)',
        boxShadow: '0 12px 40px rgba(0,0,0,0.42)',
        fontFamily: FONT.sans,
        fontWeight: 500,
        fontSize: 26,
        lineHeight: '34px',
        letterSpacing: '0.03em',
        color: COLOR.ink,
        whiteSpace: 'nowrap',
      }}
    >
      {text}
    </div>
  );
};
