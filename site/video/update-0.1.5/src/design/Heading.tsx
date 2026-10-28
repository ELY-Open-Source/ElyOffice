import { Img, interpolate, staticFile, useCurrentFrame } from 'remotion';
import { mix } from './Shape';
import { type AppId, COLOR, EASE, FONT, HEADING, STAGE, appOf } from '../theme';

export type HeadingSpec = {
  title: string;
  sub?: string;
  // App chip at the right edge, above the stage.
  app?: AppId;
  // Section-local frame the title starts to enter (default: beat 1).
  at?: number;
  // Section-local frame it starts to leave (default: the next heading's
  // entrance, or HEADING.out frames before the section ends).
  until?: number;
};

const ENTER = 28;
const LEAVE = 22;

const clamp = { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' } as const;

// Per-character rise, unblur and fade; leaving drifts on up and dissolves.
// The first `keepIn` characters are already on screen (shared with the previous
// heading); the first `keepOut` stay for the next one until `handoff`.
const Kinetic: React.FC<{
  text: string;
  local: number;
  leave: number;
  stagger: number;
  rise: number;
  keepIn?: number;
  keepOut?: number;
  handoff?: number;
}> = ({ text, local, leave, stagger, rise, keepIn = 0, keepOut = 0, handoff = Infinity }) => {
  const chars = Array.from(text);
  return (
    <>
      {chars.map((char, i) => {
        const kept = i < keepOut;
        if (kept && local >= handoff) return <span key={i} style={{ display: 'inline-block', whiteSpace: 'pre', opacity: 0 }}>{char}</span>;
        const order = i - keepIn;
        const p = i < keepIn ? 1 : interpolate(local - order * stagger, [0, ENTER], [0, 1], { ...clamp, easing: EASE.out });
        const q = kept ? 0 : interpolate(local - leave - (i - keepOut) * stagger * 0.55, [0, LEAVE], [0, 1], { ...clamp, easing: EASE.in });
        const y = (1 - p) * rise - q * rise * 0.6;
        const blur = (1 - p) * 9 + q * 7;
        return (
          <span
            key={i}
            style={{
              display: 'inline-block',
              whiteSpace: 'pre',
              opacity: p * (1 - q),
              transform: `translateY(${y.toFixed(2)}px)`,
              filter: blur > 0.05 ? `blur(${blur.toFixed(2)}px)` : undefined,
            }}
          >
            {char}
          </span>
        );
      })}
    </>
  );
};

// One section title above the stage's left edge, with an optional app chip.
export type HeadingTiming = {
  spec: HeadingSpec;
  enter: number;
  leave: number;
  // Characters shared with the previous / next heading of the section.
  keepIn: number;
  keepOut: number;
  // Section-local frame the next heading takes over (null for the last).
  handoff: number | null;
  // Rule colour of the previous heading, which this one eases from.
  fromColor: string | null;
};

export const Heading: React.FC<{ timing: HeadingTiming; color: string }> = ({ timing, color }) => {
  const { spec, enter, leave, keepIn, keepOut, handoff, fromColor } = timing;
  const frame = useCurrentFrame();
  const local = frame - enter;
  const until = handoff ?? leave + LEAVE + 40;
  if (local < 0 || frame >= until) return null;
  const stay = leave - enter;
  const subAt = HEADING.sub;
  const rule = fromColor === null ? interpolate(local, [-6, 22], [0, 1], { ...clamp, easing: EASE.out }) : 1;
  const ruleOut = handoff === null ? interpolate(local - stay - 8, [0, 18], [0, 1], { ...clamp, easing: EASE.inOut }) : 0;
  const ruleColor = fromColor === null ? color : mix(fromColor, color, interpolate(local, [0, 24], [0, 1], { ...clamp, easing: EASE.inOut }));
  const chip = interpolate(local - 10, [0, 30], [0, 1], { ...clamp, easing: EASE.out });
  const chipOut = interpolate(local - stay, [0, LEAVE], [0, 1], { ...clamp, easing: EASE.in });
  const app = spec.app ? appOf(spec.app) : null;
  return (
    <div style={{ position: 'absolute', left: STAGE.x, top: 28, width: STAGE.w, height: 76 }}>
      <div
        style={{
          position: 'absolute',
          left: 0,
          top: 6,
          width: 4,
          height: 64,
          borderRadius: 2,
          background: ruleColor,
          transformOrigin: '50% 0%',
          transform: `scaleY(${(rule * (1 - ruleOut)).toFixed(4)})`,
        }}
      />
      <div
        style={{
          position: 'absolute',
          left: 22,
          top: 0,
          fontFamily: FONT.sans,
          fontWeight: 600,
          fontSize: 42,
          lineHeight: '48px',
          letterSpacing: '0.02em',
          color: COLOR.ink,
          whiteSpace: 'nowrap',
        }}
      >
        <Kinetic
          text={spec.title}
          local={local}
          leave={stay}
          stagger={1.8}
          rise={24}
          keepIn={keepIn}
          keepOut={keepOut}
          handoff={handoff === null ? Infinity : handoff - enter}
        />
      </div>
      {spec.sub ? (
        <div
          style={{
            position: 'absolute',
            left: 23,
            top: 52,
            fontFamily: FONT.sans,
            fontWeight: 400,
            fontSize: 19,
            lineHeight: '24px',
            letterSpacing: '0.04em',
            color: COLOR.muted,
            whiteSpace: 'nowrap',
          }}
        >
          <Kinetic text={spec.sub} local={local - subAt} leave={stay - subAt - 6} stagger={0.55} rise={12} />
        </div>
      ) : null}
      {app ? (
        <div
          style={{
            position: 'absolute',
            right: 0,
            top: 18,
            height: 40,
            display: 'flex',
            alignItems: 'center',
            gap: 12,
            opacity: chip * (1 - chipOut),
            transform: `translateX(${((1 - chip) * 18 - chipOut * 10).toFixed(2)}px)`,
            filter: chip < 1 || chipOut > 0 ? `blur(${((1 - chip) * 6 + chipOut * 5).toFixed(2)}px)` : undefined,
          }}
        >
          <Img src={staticFile(`icons/${app.id}.svg`)} style={{ width: 40, height: 40 }} />
          <span style={{ fontFamily: FONT.latin, fontWeight: 500, fontSize: 21, letterSpacing: '0.01em', color: COLOR.muted }}>
            {app.name}
          </span>
        </div>
      ) : null}
    </div>
  );
};

const shared = (a: string, b: string) => {
  const x = Array.from(a);
  const y = Array.from(b);
  let n = 0;
  while (n < x.length && n < y.length && x[n] === y[n]) n++;
  // Keep whole words only: stop after the last space of the shared run.
  const cut = x.slice(0, n).lastIndexOf(' ');
  return n === x.length || n === y.length ? n : cut + 1;
};

// Resolve each heading's frames within a section; `accent` colours the rule.
export const headingTimes = (specs: HeadingSpec[], duration: number, accent: (spec: HeadingSpec) => string): HeadingTiming[] =>
  specs.map((spec, i) => {
    const enter = spec.at ?? HEADING.in;
    const prev = specs[i - 1];
    const next = specs[i + 1];
    const nextAt = next ? (next.at ?? HEADING.in) : null;
    const leave = spec.until ?? (nextAt !== null ? nextAt - LEAVE - 6 : duration - HEADING.out);
    return {
      spec,
      enter,
      leave,
      keepIn: prev ? shared(prev.title, spec.title) : 0,
      keepOut: next ? shared(spec.title, next.title) : 0,
      handoff: nextAt,
      fromColor: prev ? accent(prev) : null,
    };
  });
