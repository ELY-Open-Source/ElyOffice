// Repo rules for this folder: length, one-line short comments.
import fs from 'node:fs';
import path from 'node:path';

const SKIP = new Set(['node_modules', '.work', 'renders']);
const LIMIT = 500;
const WORDS = 10;

function files(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((d) => {
    if (SKIP.has(d.name)) return [];
    const p = path.join(dir, d.name);
    if (d.isDirectory()) return files(p);
    return /\.(js|mjs|py|sh)$/.test(d.name) ? [p] : [];
  });
}

export function rules(root) {
  const fail = [];
  for (const file of files(root)) {
    const lines = fs.readFileSync(file, 'utf8').split('\n');
    const name = path.relative(root, file);
    if (lines.length - (lines.at(-1) === '' ? 1 : 0) > LIMIT) fail.push(`${name}: over ${LIMIT} lines`);
    const mark = file.endsWith('.js') || file.endsWith('.mjs') ? /^\s*\/\/\s?(.*)$/ : /^\s*#(?!!)\s?(.*)$/;
    lines.forEach((line, i) => {
      if (/\/\*/.test(line) && !/\*\//.test(line)) fail.push(`${name}:${i + 1}: block comment over lines`);
      const m = line.match(mark);
      if (m && m[1].trim().split(/\s+/).length > WORDS) fail.push(`${name}:${i + 1}: comment over ${WORDS} words`);
    });
  }
  return fail;
}
