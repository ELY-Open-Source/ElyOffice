import { AbsoluteFill, interpolate, useCurrentFrame } from 'remotion';
import { COLOR, EASE, FONT, ICON, SECTION } from '../theme';
import { type ShapeId, SHAPES, boundaryPulse, shapePath } from '../design/morph';
import { ShapeView, type Details, mix, rgba } from '../design/Shape';

// Each icon lands on a beat; the dot breathes for the first two beats.
const STEPS: { shape: ShapeId; land: number; dur: number }[] = [
  { shape: 'ely-doc', land: 75, dur: 30 },
  { shape: 'ely-show', land: 112.5, dur: 24 },
  { shape: 'ely-table', land: 150, dur: 24 },
  { shape: 'ely-tree', land: 187.5, dur: 24 },
  { shape: 'ely-pdf', land: 225, dur: 24 },
  { shape: 'ely-mark', land: 262.5, dur: 24 },
];

const END = SECTION.intro.duration;
const clamp = { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' } as const;
const ramp = (f: number, a: number, b: number, ease = EASE.out) => interpolate(f, [a, b], [0, 1], { ...clamp, easing: ease });

const iconAt = (f: number) => {
  // Before the wordmark the icon sits a little higher, at the frame's centre.
  const cy = ICON.y - 40 * (1 - ramp(f, 150, 240, EASE.inOut));
  const pulse = boundaryPulse(f - END);
  const index = STEPS.findLastIndex((s) => f >= s.land - s.dur);
  if (index < 0) {
    const appear = ramp(f, 4, 38);
    const breath = 0.16 * Math.exp(-(((f - 37.5) / 8) ** 2));
    const k = Math.max(0.001, appear * (1 + breath));
    return { d: shapePath('dot', 'stage', 0, k, ICON.x, cy), color: SHAPES.dot.color, k, cy, details: [] as Details[] };
  }
  const step = STEPS[index];
  const from: ShapeId = index === 0 ? 'dot' : STEPS[index - 1].shape;
  const start = step.land - step.dur;
  const t = EASE.inOut(Math.min(1, (f - start) / step.dur));
  const swell = STEPS.slice(0, -1).reduce((sum, s) => sum + 0.03 * Math.exp(-(((f - s.land) / 6) ** 2)), 0);
  const k = (1 - 0.07 * Math.sin(Math.PI * t)) * (1 + swell + pulse);
  const d = t >= 1 ? shapePath(step.shape, 'stage', 0, k, ICON.x, cy) : shapePath(from, step.shape, t, k, ICON.x, cy);
  const details: Details[] = [
    { shape: from, opacity: 1 - ramp(f, start, start + 6, EASE.inOut) },
    { shape: step.shape, opacity: ramp(f, step.land - 3, step.land + 7, EASE.inOut) },
  ];
  return { d, color: mix(SHAPES[from].color, SHAPES[step.shape].color, t), k, cy, details };
};

const Letters: React.FC<{ text: string; start: number; stagger: number; rise: number; blur: number }> = ({ text, start, stagger, rise, blur }) => {
  const f = useCurrentFrame();
  return (
    <>
      {Array.from(text).map((char, i) => {
        const p = ramp(f, start + i * stagger, start + i * stagger + 32);
        const b = (1 - p) * blur;
        return (
          <span
            key={i}
            style={{
              display: 'inline-block',
              whiteSpace: 'pre',
              opacity: p,
              transform: `translateY(${((1 - p) * rise).toFixed(2)}px)`,
              filter: b > 0.05 ? `blur(${b.toFixed(2)}px)` : undefined,
            }}
          >
            {char}
          </span>
        );
      })}
    </>
  );
};

const WORD: React.CSSProperties = {
  fontFamily: FONT.latin,
  fontSize: 92,
  lineHeight: '100px',
  whiteSpace: 'nowrap',
  display: 'flex',
  alignItems: 'baseline',
  gap: 30,
};

const Wordmark: React.FC<{ reveal: boolean; color?: string }> = ({ reveal, color }) => {
  const word = (text: string, start: number) =>
    reveal ? <Letters text={text} start={start} stagger={2.2} rise={30} blur={10} /> : text.split('').map((c, i) => <span key={i} style={{ display: 'inline-block', whiteSpace: 'pre' }}>{c}</span>);
  return (
    <div style={WORD}>
      <span style={{ fontWeight: 600, letterSpacing: '-0.025em', color: color ?? '#ECEAE6' }}>{word('Ely Office', 150)}</span>
      <span style={{ fontWeight: 300, letterSpacing: '0.01em', color: color ?? COLOR.muted }}>{word('0.1.5', 172)}</span>
    </div>
  );
};

export const Intro: React.FC = () => {
  const f = useCurrentFrame();
  const icon = iconAt(f);
  const dawn = 1 - ramp(f, 0, 50, EASE.inOut);
  const ripple = ramp(f, 37.5, 37.5 + 50);
  const leaveWord = ramp(f, 272, 294, EASE.in);
  const leaveLine = ramp(f, 264, 286, EASE.in);
  const rule = ramp(f, 182, 210) * (1 - ramp(f, 258, 278, EASE.inOut));
  const sweep = ramp(f, 216, 262, EASE.inOut);
  return (
    <AbsoluteFill>
      {f < 92 ? (
        <svg width={1920} height={1080} style={{ position: 'absolute', inset: 0 }}>
          <circle
            cx={ICON.x}
            cy={icon.cy}
            r={20 + 150 * ripple}
            fill="none"
            stroke={rgba(SHAPES.dot.color, 0.5 * (1 - ripple) * Math.min(1, ripple * 8))}
            strokeWidth={1.5}
          />
        </svg>
      ) : null}
      <ShapeView d={icon.d} color={icon.color} cx={ICON.x} cy={icon.cy} k={icon.k} fill={1} lift={0} details={icon.details} />
      <div
        style={{
          position: 'absolute',
          left: 0,
          width: 1920,
          top: 232,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          opacity: 1 - leaveWord,
          transform: `translateY(${(-20 * leaveWord).toFixed(2)}px)`,
          filter: leaveWord > 0.01 ? `blur(${(8 * leaveWord).toFixed(2)}px)` : undefined,
        }}
      >
        <div style={{ position: 'relative' }}>
          <Wordmark reveal />
          {sweep > 0 && sweep < 1 ? (
            <div
              style={{
                position: 'absolute',
                inset: 0,
                opacity: 0.9,
                maskImage: 'linear-gradient(100deg, transparent 0%, transparent 44%, black 50%, transparent 56%, transparent 100%)',
                maskSize: '320% 100%',
                maskRepeat: 'no-repeat',
                maskPosition: `${(100 - sweep * 100).toFixed(2)}% 0%`,
              }}
            >
              <Wordmark reveal={false} color="#FFFFFF" />
            </div>
          ) : null}
        </div>
      </div>
      <div style={{ position: 'absolute', left: 960 - 28, top: 354, width: 56, height: 2, borderRadius: 1, background: COLOR.accent, transform: `scaleX(${rule.toFixed(4)})` }} />
      <div
        style={{
          position: 'absolute',
          left: 0,
          width: 1920,
          top: 372,
          textAlign: 'center',
          fontFamily: FONT.sans,
          fontWeight: 300,
          fontSize: 31,
          lineHeight: '40px',
          letterSpacing: '0.34em',
          paddingLeft: '0.34em',
          boxSizing: 'border-box',
          color: '#BDBAB5',
          opacity: 1 - leaveLine,
          transform: `translateY(${(-12 * leaveLine).toFixed(2)}px)`,
          filter: leaveLine > 0.01 ? `blur(${(6 * leaveLine).toFixed(2)}px)` : undefined,
        }}
      >
        <Letters text="更顺手的一次更新" start={192} stagger={3} rise={14} blur={8} />
      </div>
      {dawn > 0.001 ? <AbsoluteFill style={{ background: '#09090a', opacity: dawn }} /> : null}
    </AbsoluteFill>
  );
};
