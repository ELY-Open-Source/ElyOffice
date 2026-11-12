// Comment spans: JS and TS by TypeScript, HTML by parse5.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const site = createRequire(join(resolve(process.argv[2]), "package.json"));
const ts = site("typescript");
const { parse } = await import(pathToFileURL(site.resolve("parse5")).href);
const KINDS = { js: ts.ScriptKind.JS, ts: ts.ScriptKind.TS, tsx: ts.ScriptKind.TSX };
const XHTML = "http://www.w3.org/1999/xhtml";

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

/** Comments, scripts' comments, and style bodies tagged "style". */
function html(text) {
  const out = [];
  const visit = (node) => {
    const at = node.sourceCodeLocation;
    if (node.nodeName === "#comment" && at) out.push([at.startOffset, at.endOffset]);
    // Only HTML's script and style hold code, as browsers read.
    if (node.namespaceURI === XHTML && (node.tagName === "script" || node.tagName === "style") && at?.startTag) {
      // Unclosed, the body runs to the end, as browsers read.
      const [start, end] = [at.startTag.endOffset, at.endTag?.startOffset ?? text.length];
      if (node.tagName === "style") out.push([start, end, "style"]);
      else for (const [s, e] of spans(text.slice(start, end), "js")) out.push([start + s, start + e]);
    }
    for (const child of node.content?.childNodes ?? node.childNodes ?? []) visit(child);
  };
  visit(parse(text, { sourceCodeLocationInfo: true }));
  return out;
}

const asked = JSON.parse(readFileSync(0, "utf8"));
process.stdout.write(JSON.stringify(asked.map(({ text, kind }) => (kind === "html" ? html(text) : spans(text, kind)))));
