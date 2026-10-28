import React from 'react';
import { AbsoluteFill, interpolate, OffthreadVideo, Sequence, staticFile, useCurrentFrame } from 'remotion';
import { EASE, FOOTAGE, FPS, STAGE } from '../theme';
import { Cursor } from './Cursor';
import type { TakeName } from './take';

/** A stretch of one take: footage seconds `from` to `to`. */
export type Cut = { take: TakeName; from: number; to: number; rate?: number; fade?: number };

/** Camera key at frame `f`: zoom, centred footage point. */
export type Shot = { f: number; z: number; x: number; y: number };

export type Placed = Cut & { start: number; length: number };

/** Each cut's section frames; its fade overlaps the one before. */
export const lay = (cuts: Cut[], first = 0): Placed[] => {
  let at = first;
  return cuts.map((cut, i) => {
    const length = Math.round(((cut.to - cut.from) * FPS) / (cut.rate ?? 1));
    const start = i === 0 ? at : at - (cut.fade ?? 0);
    at = start + length;
    return { ...cut, start, length };
  });
};

/** Footage time a placed cut shows at section frame `f`. */
export const sourceAt = (cut: Placed, f: number) => cut.from + ((f - cut.start) * (cut.rate ?? 1)) / FPS;

const camera = (shots: Shot[], f: number) => {
  if (f <= shots[0].f) return shots[0];
  const last = shots[shots.length - 1];
  if (f >= last.f) return last;
  const i = shots.findIndex((s, k) => k > 0 && s.f > f);
  const a = shots[i - 1];
  const b = shots[i];
  const k = EASE.inOut((f - a.f) / (b.f - a.f));
  return { f, z: a.z + (b.z - a.z) * k, x: a.x + (b.x - a.x) * k, y: a.y + (b.y - a.y) * k };
};

/** Footage pixels to stage pixels under the camera at `f`. */
export const viewOf = (shots: Shot[], f: number) => {
  const c = camera(shots, f);
  const k = FOOTAGE.scale * c.z;
  const vw = FOOTAGE.w / c.z;
  const vh = FOOTAGE.h / c.z;
  const left = Math.min(Math.max(c.x - vw / 2, 0), FOOTAGE.w - vw);
  const top = Math.min(Math.max(c.y - vh / 2, 0), FOOTAGE.h - vh);
  return { k, left, top, z: c.z };
};

type Props = {
  cuts: Cut[];
  shots: Shot[];
  /** Footage-space overlays for a cut at footage time `t`. */
  overlay?: (cut: Placed, t: number, f: number) => React.ReactNode;
};

/** Recorded takes on stage, with drawn pointer and directed camera. */
export const Clip: React.FC<Props> = ({ cuts, shots, overlay }) => {
  const f = useCurrentFrame();
  const view = viewOf(shots, f);
  const placed = lay(cuts);
  return (
    <AbsoluteFill style={{ width: STAGE.w, height: STAGE.h, overflow: 'hidden', background: '#202020' }}>
      {placed.map((cut, i) => {
        const fade = i === 0 ? 0 : cut.fade ?? 0;
        const opacity = fade ? interpolate(f, [cut.start, cut.start + fade], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp', easing: EASE.inOut }) : 1;
        const next = placed[i + 1];
        const end = next ? next.start + (next.fade ?? 0) : cut.start + cut.length;
        if (f < cut.start || f >= end) return null;
        const t = sourceAt(cut, f);
        return (
          <AbsoluteFill key={i} style={{ opacity }}>
            <div
              style={{
                position: 'absolute',
                width: FOOTAGE.w,
                height: FOOTAGE.h,
                transformOrigin: '0 0',
                transform: `scale(${view.k}) translate(${-view.left}px, ${-view.top}px)`,
              }}
            >
              <Sequence from={cut.start} durationInFrames={end - cut.start} layout="none">
                <OffthreadVideo
                  src={staticFile(`footage/${cut.take}.mp4`)}
                  trimBefore={Math.round(cut.from * FPS)}
                  playbackRate={cut.rate ?? 1}
                  muted
                  style={{ width: FOOTAGE.w, height: FOOTAGE.h, display: 'block' }}
                />
              </Sequence>
              {overlay?.(cut, t, f)}
              <Cursor take={cut.take} t={t} zoom={view.z} />
            </div>
          </AbsoluteFill>
        );
      })}
    </AbsoluteFill>
  );
};
