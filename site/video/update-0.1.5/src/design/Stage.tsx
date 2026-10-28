import type { ReactNode } from 'react';
import { AbsoluteFill, useCurrentFrame, useVideoConfig } from 'remotion';
import { COLOR, ENTER_END, EXIT_LEN, STAGE, STAGE_CENTER } from '../theme';
import { roundedRectPath } from './geometry';
import { Heading, type HeadingSpec, headingTimes } from './Heading';
import { type MorphState, SHAPES, STAGE_ZOOM, type ShapeId, enterState, exitState, shapePath } from './morph';
import { ShapeView, mix } from './Shape';

export type StageProps = {
  // Icon the stage blooms out of on the section's downbeat.
  enter?: { from: ShapeId };
  // Icon the stage collapses into on the next section's downbeat.
  exit?: { to: ShapeId };
  heading?: HeadingSpec | HeadingSpec[];
  // Panel colour behind the recording.
  background?: string;
  children?: ReactNode;
};

const OPEN: MorphState = {
  s: 1,
  k: STAGE_ZOOM,
  cx: STAGE_CENTER.x,
  cy: STAGE_CENTER.y,
  details: 0,
  fill: 0,
  content: 1,
  contentScale: 1,
  blur: 0,
  lift: 1,
  border: 1,
  sweep: null,
};

const OPEN_PATH = roundedRectPath(STAGE.x, STAGE.y, STAGE.w, STAGE.h, STAGE.r);

// The framed screen; children use stage coordinates, clipped to it.
export const Stage: React.FC<StageProps> = ({ enter, exit, heading, background = COLOR.panel, children }) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const entering = enter && frame < ENTER_END;
  const exiting = exit && frame >= durationInFrames - EXIT_LEN;
  const icon: ShapeId | null = entering ? enter.from : exiting ? exit.to : null;
  const state = entering ? enterState(frame) : exiting ? exitState(frame, durationInFrames) : OPEN;
  const d = icon === null ? OPEN_PATH : shapePath(icon, 'stage', state.s, state.k, state.cx, state.cy);
  const openColor = SHAPES[enter?.from ?? exit?.to ?? 'dot'].color;
  const color = icon === null ? openColor : SHAPES[icon].color;
  // The open stage glows in its app's colour, handing over.
  const glowColor = exiting ? mix(openColor, color, 1 - state.lift) : color;
  const sectionAccent = SHAPES[enter?.from ?? exit?.to ?? 'dot'].accent;
  const accent = (spec: HeadingSpec) => (spec.app ? SHAPES[spec.app].accent : sectionAccent);
  const headings = heading === undefined ? [] : headingTimes(Array.isArray(heading) ? heading : [heading], durationInFrames, accent);
  return (
    <AbsoluteFill>
      <ShapeView
        d={d}
        color={color}
        glowColor={glowColor}
        cx={state.cx}
        cy={state.cy}
        k={state.k}
        fill={state.fill}
        lift={state.lift}
        border={state.border}
        sweep={state.sweep}
        details={icon === null ? [] : [{ shape: icon, opacity: state.details }]}
        content={{ node: children, opacity: state.content, scale: state.contentScale, blur: state.blur, background }}
      />
      {headings.map((timing) => (
        <Heading key={`${timing.spec.title}-${timing.enter}`} timing={timing} color={accent(timing.spec)} />
      ))}
    </AbsoluteFill>
  );
};
