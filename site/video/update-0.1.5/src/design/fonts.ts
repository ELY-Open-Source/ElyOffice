import { cancelRender, continueRender, delayRender, staticFile } from 'remotion';

const FACES = [
  { family: 'Noto Sans SC', file: 'fonts/NotoSansSC.ttf', weight: '100 900' },
  { family: 'Inter', file: 'fonts/Inter.ttf', weight: '100 900' },
  { family: 'JetBrains Mono', file: 'fonts/JetBrainsMono.ttf', weight: '100 800' },
];

const handle = delayRender('Loading fonts', { timeoutInMilliseconds: 120000 });

Promise.all(
  FACES.map(async ({ family, file, weight }) => {
    const face = new FontFace(family, `url(${staticFile(file)}) format("truetype")`, { weight });
    await face.load();
    document.fonts.add(face);
  }),
).then(
  () => continueRender(handle),
  (error: unknown) => cancelRender(error),
);
