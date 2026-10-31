import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { inline, page, parse, releases } from "../changelog/page.ts";

const root = new URL("../../", import.meta.url);
const read = (path: string) => readFileSync(new URL(path, root), "utf8");

/** `v1.2.3` as numbers, for sorting. */
const parts = (tag: string) => tag.slice(1).split(".").map(Number);
const newer = (a: string, b: string) => {
  const [x, y] = [parts(a), parts(b)];
  return y[0] - x[0] || y[1] - x[1] || y[2] - x[2];
};

test("the changelog has every release tag, newest first, in both languages", () => {
  const tags = execFileSync("git", ["tag", "-l", "v*"], { cwd: root, encoding: "utf8" }).split("\n").filter(Boolean);
  assert.ok(tags.length > 10, `only ${tags.length} tags`);
  const all = releases(new URL("site/changelog/", root));
  assert.deepEqual(all.map((r) => `v${r.version}`), [...tags].sort(newer));
  for (const r of all) assert.ok(r.en.length && r.zh.length, `${r.version} lacks a language`);
  const [en, zh] = page(all).split('<article id="zh"');
  assert.equal(en.split("<h2 ").length - 1, tags.length);
  assert.equal(zh.split("<h2>").length - 1, tags.length);
});

test("notes lines become safe HTML", () => {
  assert.equal(inline("a `<b>` & [c](https://d/e)"), 'a <code>&lt;b&gt;</code> &amp; <a href="https://d/e">c</a>');
  assert.throws(() => parse("# 0.1.0 2026-10-08\n\n## 中文\n\n- a\n\n## English\n\n- b\n"), /English, then/);
  assert.throws(() => parse("# 0.1.0 2026-10-08\n\n## English\n\nloose text\n\n## 中文\n\n- a\n"), /neither a head nor an item/);
});

test("the docs page shows each app's own usage, line for line", () => {
  const page = read("site/public/docs.html").replaceAll("&amp;", "&");
  for (const app of ["ely-doc", "ely-table", "ely-show"]) {
    const usage = read(`apps/${app}/src/remote.rs`).match(/const USAGE: &str = "([^"]+)";/)?.[1];
    assert.ok(usage, `${app} has no USAGE`);
    const commands = usage.split("\n").map((line) => line.replace(/^(usage:)?\s+/, "")).filter((line) => line.startsWith(app));
    assert.equal(commands.length, 4, `${app}: ${commands}`);
    // Once in English, once in Chinese.
    for (const command of commands) assert.equal(page.split(command).length - 1, 2, command);
  }
});
