// Writes public/changelog.html from one notes file per release.
import { readdirSync, readFileSync, writeFileSync } from "node:fs";

type Release = { version: string; date: string; en: string[]; zh: string[] };

const MONTHS = "January February March April May June July August September October November December".split(" ");

const escape = (text: string) => text.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");

/** A line's `code` and [links](url) as HTML; the rest escaped. */
export function inline(text: string): string {
  return text
    .split(/(`[^`]+`)/)
    .map((part, n) => {
      if (n % 2) return `<code>${escape(part.slice(1, -1))}</code>`;
      return escape(part).replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, '<a href="$2">$1</a>');
    })
    .join("");
}

/** A section's `### heads` and `- items` as HTML lines. */
function section(lines: string[]): string[] {
  const html: string[] = [];
  let open = false;
  for (const line of lines.filter(Boolean)) {
    const item = line.startsWith("- ");
    if (!item && !line.startsWith("### ")) throw new Error(`neither a head nor an item: ${line}`);
    if (item !== open) html.push(item ? "<ul>" : "</ul>");
    open = item;
    html.push(item ? `  <li>${inline(line.slice(2))}</li>` : `<h3>${inline(line.slice(4))}</h3>`);
  }
  if (open) html.push("</ul>");
  return html;
}

/** One notes file: `# version date`, then English and 中文. */
export function parse(text: string): Release {
  const [head, ...rest] = text.split("\n");
  const found = /^# (\d+\.\d+\.\d+) (\d{4}-\d{2}-\d{2})$/.exec(head);
  if (!found) throw new Error(`a notes file starts with "# version date": ${head}`);
  const body = rest.join("\n");
  const [before, en, zh, ...extra] = body.split(/^## (?:English|中文)$/m);
  if (before.trim() || zh === undefined || extra.length || !body.trimStart().startsWith("## English")) {
    throw new Error(`${found[1]} needs ## English, then ## 中文, and nothing else`);
  }
  return { version: found[1], date: found[2], en: section(en.split("\n")), zh: section(zh.split("\n")) };
}

const newest = (a: Release, b: Release) => {
  const [x, y] = [a.version, b.version].map((v) => v.split(".").map(Number));
  return y[0] - x[0] || y[1] - x[1] || y[2] - x[2];
};

/** Every release in `dir`, newest first. */
export function releases(dir: URL): Release[] {
  const files = readdirSync(dir).filter((name) => name.endsWith(".md"));
  return files.map((name) => parse(readFileSync(new URL(name, dir), "utf8").replaceAll("\r\n", "\n"))).sort(newest);
}

const said = {
  en: (r: Release) => {
    const [y, m, d] = r.date.split("-").map(Number);
    return `<h2 id="v${r.version}">${r.version} <span class="when">${d} ${MONTHS[m - 1]} ${y}</span></h2>`;
  },
  zh: (r: Release) => {
    const [y, m, d] = r.date.split("-").map(Number);
    return `<h2>${r.version} <span class="when">${y} 年 ${m} 月 ${d} 日</span></h2>`;
  },
};

/** The whole page. */
export function page(all: Release[]): string {
  const article = (lang: "en" | "zh", top: string) =>
    [top, ...all.flatMap((r) => [said[lang](r), ...r[lang]])].map((line) => `        ${line}`).join("\n");
  const en = `<h1>Changelog</h1>
<p class="date">Every release, newest first. Downloads are on <a href="https://github.com/ELY-Open-Source/ElyOffice/releases">GitHub</a>. Issue numbers up to 0.1.8 refer to the earlier Gitea tracker.</p>`;
  const zh = `<h1>更新日志</h1>
<p class="date">所有版本，新的在前。下载在 <a href="https://github.com/ELY-Open-Source/ElyOffice/releases">GitHub</a>。0.1.8 及以前的 issue 编号指原先的 Gitea。</p>`;
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Changelog · Ely Office</title>
    <meta name="description" content="What changed in each Ely Office release." />
    <link rel="stylesheet" href="/legal.css" />
  </head>
  <body>
    <main>
      <nav><a href="/">Ely Office</a><button type="button"></button></nav>
      <article data-title="Changelog · Ely Office">
${article("en", en)}
      </article>
      <article lang="zh-CN" data-title="更新日志 · Ely Office">
${article("zh", zh)}
      </article>
    </main>
    <script src="/lang.js"></script>
  </body>
</html>
`;
}

if (import.meta.main) {
  const all = releases(new URL("./", import.meta.url));
  writeFileSync(new URL("../public/changelog.html", import.meta.url), page(all));
  console.log(`changelog: ${all.length} releases, newest ${all[0].version}`);
}
