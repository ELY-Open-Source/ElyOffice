// Comment spans in JS and TS, by the TypeScript parser.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join, resolve } from "node:path";

const ts = createRequire(join(resolve(process.argv[2]), "package.json"))("typescript");
const KINDS = { js: ts.ScriptKind.JS, ts: ts.ScriptKind.TS, tsx: ts.ScriptKind.TSX };

function spans(text, kind) {
  const file = ts.createSourceFile("x", text, ts.ScriptTarget.Latest, true, KINDS[kind]);
  const found = new Map();
  const prose = [];
  const add = (ranges) => {
    for (const range of ranges ?? []) found.set(range.pos, range.end);
  };
  const visit = (node) => {
    // JSX text is prose: "//" in it is no comment.
    if (node.kind === ts.SyntaxKind.JsxText) return prose.push([node.pos, node.end]);
    add(ts.getLeadingCommentRanges(text, node.pos));
    add(ts.getTrailingCommentRanges(text, node.end));
    for (const child of node.getChildren(file)) visit(child);
  };
  visit(file);
  return [...found]
    .filter(([pos, end]) => !text.startsWith("#!", pos) && !prose.some(([a, b]) => pos >= a && pos < b))
    .sort((a, b) => a[0] - b[0]);
}

const asked = JSON.parse(readFileSync(0, "utf8"));
process.stdout.write(JSON.stringify(asked.map(({ text, kind }) => spans(text, kind))));
