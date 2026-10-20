// Every word on the page, in both languages.
export type Lang = "en" | "zh";

const en = {
  tagline: "Six desktop apps. They work offline.",
  ElyDoc: "Letters, reports, books. Opens and saves Word files.",
  ElyTable: "Formulas, filters, charts. Opens and saves Excel files.",
  ElyShow: "Slides, animation, a presenter view. Opens PowerPoint files.",
  ElyPDF: "Read, mark up, sign, fill forms. Finds text in scans.",
  ElyTree: "Stage lines, commit, read the graph. Git without a terminal.",
  ElyMark: "Markdown as you read it. Math and diagrams in place.",
  offline: "Works with the network off.",
  open: "Open source, under the MIT licence.",
  local: "Your files stay in your folders.",
  download: "Download",
  platforms: "Out now for macOS, Windows and Linux.",
  needs: "macOS 14 or later, Apple silicon.",
  unsigned: "Not notarized. If macOS blocks the app, open System Settings › Privacy & Security and click Open Anyway.",
  get: "Download",
  other: "中文",
  roads: "Drag a file down the road.",
  fileDoc: "ElyDoc file",
  fileTable: "ElyTable file",
  fileShow: "ElyShow file",
  fileMark: "ElyMark file",
  moveHint: "Arrow keys move it along the road.",
  toTree: "Open a Markdown file's history in ElyTree.",
  toPdf: "Export to PDF, open it in ElyPDF.",
  putBack: "Put the files back",
  play: "Play music",
  pause: "Pause music",
  filmPlay: "Play the film",
  filmLine: "90 seconds. Real use, no mockups.",
  dark: "Switch to dark",
  light: "Switch to light",
  clipPlay: "Play",
  clipPause: "Pause",
};

const zh: typeof en = {
  tagline: "六个桌面应用，断网也能用。",
  ElyDoc: "信件、报告、书稿。打开并保存 Word 文件。",
  ElyTable: "公式、筛选、图表。打开并保存 Excel 文件。",
  ElyShow: "幻灯片、动画、演讲者视图。打开 PowerPoint 文件。",
  ElyPDF: "阅读、批注、签名、填表。扫描件里也能查字。",
  ElyTree: "按行暂存、提交、看提交图。不用终端的 Git。",
  ElyMark: "写的就是看到的 Markdown。公式与图表就地显示。",
  offline: "断网照常工作。",
  open: "开源，MIT 许可。",
  local: "文件留在你自己的文件夹里。",
  download: "下载",
  platforms: "macOS、Windows 与 Linux 版已发布。",
  needs: "macOS 14 或更新，Apple 芯片。",
  unsigned: "未经公证。macOS 拦下应用时，打开“系统设置 › 隐私与安全性”，点“仍要打开”。",
  get: "前往下载",
  other: "English",
  roads: "把文件沿着路拖过去。",
  fileDoc: "ElyDoc 文件",
  fileTable: "ElyTable 文件",
  fileShow: "ElyShow 文件",
  fileMark: "ElyMark 文件",
  moveHint: "方向键沿路移动它。",
  toTree: "在 ElyTree 里看这份 Markdown 的历史。",
  toPdf: "导出 PDF，在 ElyPDF 打开。",
  putBack: "把文件放回去",
  play: "播放音乐",
  pause: "暂停音乐",
  filmPlay: "播放影片",
  filmLine: "90 秒，全部是真实操作。",
  dark: "切换到深色",
  light: "切换到浅色",
  clipPlay: "播放",
  clipPause: "暂停",
};

export type Key = keyof typeof en;
export const TEXT: Record<Lang, typeof en> = { en, zh };

/** An element whose text follows the language. */
export function said<K extends keyof HTMLElementTagNameMap>(tag: K, key: Key): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  el.dataset.t = key;
  return el;
}

/** Writes every keyed text in `lang`. */
export function speak(lang: Lang): void {
  document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
  for (const el of document.querySelectorAll<HTMLElement>("[data-t]")) {
    el.textContent = TEXT[lang][el.dataset.t as Key];
  }
}
