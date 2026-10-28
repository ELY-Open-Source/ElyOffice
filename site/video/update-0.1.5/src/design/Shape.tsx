import type { ReactNode } from 'react';
import { AbsoluteFill, Img, staticFile } from 'remotion';
import { ICON, STAGE, STAGE_CENTER } from '../theme';
import type { ShapeId } from './morph';

const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

export const rgba = (hex: string, alpha: number) => {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${alpha.toFixed(4)})`;
};

export const mix = (a: string, b: string, t: number) => {
  const x = parseInt(a.slice(1), 16);
  const y = parseInt(b.slice(1), 16);
  const c = (shift: number) => Math.round(lerp((x >> shift) & 255, (y >> shift) & 255, t));
  return `#${((c(16) << 16) | (c(8) << 8) | c(0)).toString(16).padStart(6, '0')}`;
};

// The highlighted table cell that the cell-editing section grows out of.
const CellArt: React.FC = () => (
  <svg viewBox="0 0 192 192" width="100%" height="100%">
    <rect x="16" y="51" width="160" height="90" rx="8" fill="#3E4A5C" />
    <path d="M24 51H168A8 8 0 0 1 176 59V69H16V59A8 8 0 0 1 24 51Z" fill="#4E5B70" />
    <g stroke="#FFFFFF" strokeOpacity="0.16" strokeWidth="1.2">
      <path d="M69.3 51V141M122.7 51V141M16 69H176M16 93H176M16 117H176" />
    </g>
    <g fill="#FFFFFF" fillOpacity="0.55">
      <rect x="24" y="57" width="26" height="6" rx="3" />
      <rect x="77" y="57" width="30" height="6" rx="3" />
      <rect x="130" y="57" width="22" height="6" rx="3" />
      <rect x="24" y="78" width="32" height="6" rx="3" />
      <rect x="130" y="78" width="28" height="6" rx="3" />
      <rect x="24" y="102" width="22" height="6" rx="3" />
      <rect x="130" y="102" width="34" height="6" rx="3" />
      <rect x="24" y="126" width="30" height="6" rx="3" />
      <rect x="77" y="126" width="24" height="6" rx="3" />
      <rect x="130" y="126" width="26" height="6" rx="3" />
    </g>
    <rect x="77" y="78" width="26" height="6" rx="3" fill="#FFFFFF" fillOpacity="0.55" />
    <rect x="68.5" y="92.5" width="55" height="25" rx="2.5" fill="#F2A900" />
    <rect x="76" y="102" width="28" height="6" rx="3" fill="#FFFFFF" />
    <rect x="107" y="99" width="2.4" height="12" rx="1.2" fill="#FFFFFF" />
  </svg>
);

export type Details = { shape: ShapeId; opacity: number };

const Artwork: React.FC<{ details: Details; size: number; cx: number; cy: number }> = ({ details, size, cx, cy }) => {
  if (details.shape === 'dot' || details.opacity <= 0.001) return null;
  const style: React.CSSProperties = {
    position: 'absolute',
    left: cx - size / 2,
    top: cy - size / 2,
    width: size,
    height: size,
    opacity: details.opacity,
  };
  if (details.shape === 'cell') {
    return (
      <div style={style}>
        <CellArt />
      </div>
    );
  }
  return <Img src={staticFile(`icons/${details.shape}.svg`)} style={style} />;
};

export type ShapeViewProps = {
  d: string;
  color: string;
  // Ambient glow colour (default: `color`).
  glowColor?: string;
  cx: number;
  cy: number;
  k: number;
  // App-colour wash over the content.
  fill: number;
  lift: number;
  border?: number;
  sweep?: number | null;
  details?: Details[];
  // Multiplier on shadow and glow, for shapes stacked on one another.
  presence?: number;
  content?: {
    node: ReactNode;
    opacity: number;
    scale: number;
    blur: number;
    background: string;
  };
};

// One morphing shape: glow, shadow, clipped content, colour wash, artwork, edge.
export const ShapeView: React.FC<ShapeViewProps> = ({
  d,
  color,
  glowColor = color,
  cx,
  cy,
  k,
  fill,
  lift,
  border = 0,
  sweep = null,
  details = [],
  presence = 1,
  content,
}) => {
  const glowW = 400 * k;
  const glowH = glowW * lerp(1, 0.62, lift);
  const glowAlpha = lerp(0.3, 0.09, lift) * presence;
  const shadow = `drop-shadow(0 ${lerp(6, 34, lift).toFixed(2)}px ${lerp(10, 90, lift).toFixed(2)}px rgba(0,0,0,${(lerp(0.42, 0.6, lift) * presence).toFixed(3)}))`;
  const size = 192 * ICON.unit * k;
  const sweepX = sweep === null ? 0 : STAGE.x - 500 + sweep * (STAGE.w + 1000);
  return (
    <AbsoluteFill style={{ pointerEvents: 'none' }}>
      <div
        style={{
          position: 'absolute',
          left: cx - glowW / 2,
          top: cy - glowH / 2 + 24 * lift,
          width: glowW,
          height: glowH,
          background: `radial-gradient(closest-side, ${rgba(glowColor, glowAlpha)}, ${rgba(glowColor, 0)})`,
        }}
      />
      <AbsoluteFill style={{ filter: shadow }}>
        <AbsoluteFill style={{ clipPath: `path('${d}')` }}>
          {content && content.opacity > 0.001 ? (
            <AbsoluteFill
              style={{
                opacity: content.opacity,
                transform: `scale(${content.scale})`,
                transformOrigin: `${STAGE_CENTER.x}px ${STAGE_CENTER.y}px`,
                filter: content.blur > 0.05 ? `blur(${content.blur.toFixed(2)}px)` : undefined,
              }}
            >
              <div
                style={{
                  position: 'absolute',
                  left: STAGE.x,
                  top: STAGE.y,
                  width: STAGE.w,
                  height: STAGE.h,
                  background: content.background,
                  overflow: 'hidden',
                }}
              >
                {content.node}
              </div>
            </AbsoluteFill>
          ) : null}
          {fill > 0.001 ? (
            <AbsoluteFill
              style={{
                // A soft top light that scales with the shape gives the flat colour depth.
                background: `radial-gradient(circle at ${cx.toFixed(1)}px ${(cy - 80 * k).toFixed(1)}px, rgba(255,255,255,0.09), rgba(255,255,255,0) ${(230 * k).toFixed(1)}px), ${color}`,
                opacity: fill,
              }}
            />
          ) : null}
        </AbsoluteFill>
      </AbsoluteFill>
      {details.map((detail, i) => (
        <Artwork key={`${detail.shape}-${i}`} details={detail} size={size} cx={cx} cy={cy} />
      ))}
      {border > 0.001 || sweep !== null ? (
        <svg width={1920} height={1080} style={{ position: 'absolute', inset: 0 }}>
          <defs>
            <linearGradient id="edge-sweep" gradientUnits="userSpaceOnUse" x1={sweepX - 260} y1={0} x2={sweepX + 260} y2={0}>
              <stop offset="0" stopColor="#FFFFFF" stopOpacity="0" />
              <stop offset="0.5" stopColor="#FFFFFF" stopOpacity="1" />
              <stop offset="1" stopColor="#FFFFFF" stopOpacity="0" />
            </linearGradient>
          </defs>
          <path d={d} fill="none" stroke={`rgba(255,255,255,${(0.08 * border).toFixed(4)})`} strokeWidth={1} />
          {sweep !== null ? (
            <path d={d} fill="none" stroke="url(#edge-sweep)" strokeWidth={1.25} opacity={0.42 * Math.sin(Math.PI * sweep)} />
          ) : null}
        </svg>
      ) : null}
    </AbsoluteFill>
  );
};
