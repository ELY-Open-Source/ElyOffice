// Footage test entry: `remotion still src/footage/preview.tsx <Section>`.
import { AbsoluteFill, Composition, registerRoot } from 'remotion';
import '../design/fonts';
import { Background } from '../design/Background';
import { Cells } from '../scenes/Cells';
import { Menus } from '../scenes/Menus';
import { More } from '../scenes/More';
import { RightClick } from '../scenes/RightClick';
import { SECTION } from '../theme';

const framed = (Section: React.FC) => () => (
  <AbsoluteFill>
    <Background />
    <Section />
  </AbsoluteFill>
);

const ALL = [
  ['Menus', Menus, SECTION.menus],
  ['Cells', Cells, SECTION.cells],
  ['RightClick', RightClick, SECTION.rightClick],
  ['More', More, SECTION.more],
] as const;

registerRoot(() => (
  <>
    {ALL.map(([id, Section, at]) => (
      <Composition key={id} id={id} component={framed(Section)} width={1920} height={1080} fps={60} durationInFrames={at.duration} />
    ))}
  </>
));
