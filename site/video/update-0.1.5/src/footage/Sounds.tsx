import React from 'react';
import { Audio, Sequence, staticFile } from 'remotion';
import { FPS } from '../theme';
import { lay, type Cut } from './Clip';
import { TAKES } from './take';

const SOUND: Record<string, { file: string; volume: number } | undefined> = {
  down: { file: 'click', volume: 0.55 },
  right: { file: 'menu', volume: 0.6 },
  key: { file: 'key', volume: 0.5 },
  char: { file: 'key', volume: 0.45 },
  wheel: { file: 'tick', volume: 0.18 },
};

/** UI sounds on the logged presses, keys and scrolls. */
export const Sounds: React.FC<{ cuts: Cut[] }> = ({ cuts }) => {
  const hits: { frame: number; file: string; volume: number }[] = [];
  for (const cut of lay(cuts)) {
    for (const e of TAKES[cut.take].events) {
      if (e.t < cut.from || e.t >= cut.to) continue;
      const sound = SOUND[e.kind === 'down' && e.right ? 'right' : e.kind];
      if (!sound) continue;
      hits.push({ frame: Math.round(cut.start + ((e.t - cut.from) * FPS) / (cut.rate ?? 1)), ...sound });
    }
  }
  return (
    <>
      {hits.map((hit, i) => (
        <Sequence key={i} from={hit.frame} durationInFrames={60} layout="none">
          <Audio src={staticFile(`audio/sfx/${hit.file}.wav`)} volume={hit.volume} />
        </Sequence>
      ))}
    </>
  );
};

/** Section frame of footage time `t` in `cuts`. */
export const frameOf = (cuts: Cut[], t: number) => {
  const cut = lay(cuts).find((c) => t >= c.from && t < c.to);
  if (!cut) throw new Error(`no cut shows footage time ${t}`);
  return Math.round(cut.start + ((t - cut.from) * FPS) / (cut.rate ?? 1));
};
