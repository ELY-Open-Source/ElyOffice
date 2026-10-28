import { AbsoluteFill, interpolate, useCurrentFrame } from 'remotion';
import { APPS, BEAT, COLOR, EASE, FONT, ICON } from '../theme';
import { SHAPES, boundaryPulse, shapePath } from '../design/morph';
import { ShapeView, mix, rgba } from '../design/Shape';

const clamp = { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' } as const;
const ramp = (f: number, a: number, b: number, ease = EASE.out) => interpolate(f, [a, b], [0, 1], { ...clamp, easing: ease });
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

// The row after the bloom, and its tighter row above.
const ROW = { gap: 212, y: 470, k: 0.62 };
const GATHER = { gap: 128, y: 384, k: 0.38, at: 5 * BEAT };
const BLOOM = { at: BEAT, every: BEAT / 2, dur: 26 };
const SPLIT = { at: 16, dur: 40 };
const FADE = { from: 390, to: 446 };

const Ring: React.FC<{ start: number; reach: number; width: number }> = ({ start, reach, width }) => {
  const f = useCurrentFrame();
  const p = ramp(f, start, start + 64);
  if (p <= 0 || p >= 1) return null;
  return (
    <circle
      cx={ICON.x}
      cy={ICON.y}
      r={18 + reach * p}
      fill="none"
      stroke={rgba(SHAPES.dot.color, 0.55 * (1 - p) * Math.min(1, p * 10))}
      strokeWidth={width}
    />
  );
};

const AppIcon: React.FC<{ index: number }> = ({ index }) => {
  const f = useCurrentFrame();
  const app = APPS[index];
  const offset = index - (APPS.length - 1) / 2;
  const travel = Math.min(1, Math.max(0, (f - SPLIT.at) / SPLIT.dur));
  const bloomAt = BLOOM.at + index * BLOOM.every;
  const t = EASE.inOut(Math.min(1, Math.max(0, (f - bloomAt) / BLOOM.dur)));
  const land = bloomAt + BLOOM.dur;
  const gather = ramp(f, GATHER.at + index * 2, GATHER.at + index * 2 + 44, EASE.inOut);

  // Split: dots rise from rest, then fan out, a fountain.
  const splitX = lerp(ICON.x, ICON.x + offset * ROW.gap, EASE.inOut(Math.max(0, (travel - 0.15) / 0.85)));
  const splitY = lerp(ICON.y, ROW.y, EASE.inOut(Math.min(1, travel / 0.7)));
  const cx = lerp(splitX, ICON.x + offset * GATHER.gap, gather);
  const cy = lerp(splitY, GATHER.y, gather);

  const swell = 0.035 * Math.exp(-(((f - land) / 6) ** 2));
  const base = lerp(lerp(1 + boundaryPulse(f), ROW.k, EASE.inOut(travel)), GATHER.k, gather);
  const k = base * (1 - 0.08 * Math.sin(Math.PI * t)) * (1 + swell);
  const d = t >= 1 ? shapePath(app.id, 'stage', 0, k, cx, cy) : shapePath('dot', app.id, t, k, cx, cy);
  const color = mix(SHAPES.dot.color, app.color, Math.max(EASE.inOut(travel) * 0.6, t));
  return (
    <ShapeView
      d={d}
      color={color}
      cx={cx}
      cy={cy}
      k={k}
      fill={1}
      lift={0}
      presence={lerp(1 / APPS.length, 1, EASE.inOut(travel))}
      details={[{ shape: app.id, opacity: ramp(f, land - 4, land + 8, EASE.inOut) }]}
    />
  );
};

const Line: React.FC<{ text: string; start: number; stagger: number; style: React.CSSProperties }> = ({ text, start, stagger, style }) => {
  const f = useCurrentFrame();
  return (
    <div style={{ display: 'flex', justifyContent: 'center', whiteSpace: 'nowrap', ...style }}>
      {Array.from(text).map((char, i) => {
        const p = ramp(f, start + i * stagger, start + i * stagger + 30);
        const b = (1 - p) * 8;
        return (
          <span
            key={i}
            style={{
              display: 'inline-block',
              whiteSpace: 'pre',
              opacity: p,
              transform: `translateY(${((1 - p) * 18).toFixed(2)}px)`,
              filter: b > 0.05 ? `blur(${b.toFixed(2)}px)` : undefined,
            }}
          >
            {char}
          </span>
        );
      })}
    </div>
  );
};

export const Outro: React.FC = () => {
  const f = useCurrentFrame();
  const fade = 1 - ramp(f, FADE.from, FADE.to, EASE.inOut);
  const push = 1 + 0.022 * ramp(f, GATHER.at, FADE.to, EASE.inOut);
  const glow = ramp(f, 0, 18) * (1 - ramp(f, 18, 80, EASE.inOut));
  return (
    <AbsoluteFill style={{ opacity: fade, transform: `scale(${push.toFixed(5)})`, transformOrigin: '960px 520px' }}>
      <div
        style={{
          position: 'absolute',
          left: ICON.x - 360,
          top: ICON.y - 360,
          width: 720,
          height: 720,
          background: `radial-gradient(closest-side, ${rgba(COLOR.accent, 0.22 * glow)}, ${rgba(COLOR.accent, 0)})`,
        }}
      />
      <svg width={1920} height={1080} style={{ position: 'absolute', inset: 0 }}>
        <Ring start={0} reach={300} width={1.6} />
        <Ring start={9} reach={460} width={1} />
      </svg>
      {f < SPLIT.at ? (
        <ShapeView
          d={shapePath('dot', 'stage', 0, 1 + boundaryPulse(f), ICON.x, ICON.y)}
          color={SHAPES.dot.color}
          cx={ICON.x}
          cy={ICON.y}
          k={1 + boundaryPulse(f)}
          fill={1}
          lift={0}
        />
      ) : (
        APPS.map((app, i) => <AppIcon key={app.id} index={i} />)
      )}
      <div style={{ position: 'absolute', left: 0, width: 1920, top: 478 }}>
        <Line
          text="Ely Office 0.1.5"
          start={6 * BEAT}
          stagger={1.8}
          style={{ fontFamily: FONT.latin, fontWeight: 600, fontSize: 84, lineHeight: '96px', letterSpacing: '-0.02em', color: '#ECEAE6' }}
        />
        <Line
          text="Windows · macOS · Linux"
          start={7 * BEAT}
          stagger={1}
          style={{ marginTop: 22, fontFamily: FONT.latin, fontWeight: 400, fontSize: 26, lineHeight: '34px', letterSpacing: '0.06em', color: COLOR.muted }}
        />
        <Line
          text="git.zacharyzhang.com/ZacharyZhang-NY/ElyOffice"
          start={8 * BEAT}
          stagger={0.6}
          style={{ marginTop: 18, fontFamily: FONT.mono, fontWeight: 400, fontSize: 22, lineHeight: '30px', letterSpacing: '0.01em', color: '#86A9E6' }}
        />
      </div>
    </AbsoluteFill>
  );
};
