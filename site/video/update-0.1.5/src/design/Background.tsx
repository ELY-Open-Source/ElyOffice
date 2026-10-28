import { AbsoluteFill, useCurrentFrame } from 'remotion';
import { COLOR, HEIGHT, WIDTH } from '../theme';

// Near-black field, faint centre lift, 30 Hz seeded film grain.
export const Background: React.FC = () => {
  const frame = useCurrentFrame();
  const seed = Math.floor(frame / 2) % 997;
  return (
    <AbsoluteFill style={{ background: COLOR.bgOuter }}>
      <AbsoluteFill
        style={{ background: `radial-gradient(ellipse 72% 66% at 50% 47%, ${COLOR.bgInner} 0%, ${COLOR.bgOuter} 100%)` }}
      />
      <AbsoluteFill style={{ background: 'radial-gradient(ellipse 90% 90% at 50% 50%, rgba(0,0,0,0) 60%, rgba(0,0,0,0.28) 100%)' }} />
      <svg width={WIDTH} height={HEIGHT} style={{ position: 'absolute', inset: 0, opacity: 0.055, mixBlendMode: 'screen' }}>
        <filter id="grain" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="0.85" numOctaves="2" seed={seed} stitchTiles="stitch" />
          <feColorMatrix type="saturate" values="0" />
        </filter>
        <rect width="100%" height="100%" filter="url(#grain)" />
      </svg>
    </AbsoluteFill>
  );
};
