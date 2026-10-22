// Every word on the page, in both languages.
export type Lang = "en" | "zh";

const en = {
  tagline: "Seven desktop apps. They work offline.",
  ElyDoc: "Letters, reports, books. Opens and saves Word files.",
  ElyTable: "Formulas, filters, charts. Opens and saves Excel files.",
  ElyShow: "Slides, animation, a presenter view. Opens PowerPoint files.",
  ElyPDF: "Read, mark up, sign, fill forms. Finds text in scans.",
  ElyMail: "Gmail, Outlook, iCloud and IMAP in one inbox. Reads and searches offline.",
  ElyTree: "Stage lines, commit, read the graph. Git without a terminal.",
  ElyMark: "Markdown as you read it. Math and diagrams in place.",
  offline: "Works with the network off.",
  open: "Open source, under the MIT licence.",
  local: "Your files stay in your folders.",
  download: "Download",
  platforms: "Out now for macOS, Windows and Linux.",
  needs: "macOS 14 or later, Apple silicon or Intel.",
  forMacArm: "Download for Mac (Apple silicon)",
  forMacIntel: "Download for Mac (Intel)",
  forWindows: "Download for Windows",
  forLinux: "Download for Linux",
  whole: "One installer with all seven apps.",
  apart: "Need only some apps? Get them one by one on ",
  apartEnd: ".",
  macArm: "Mac, Apple silicon",
  macIntel: "Mac, Intel",
  windows: "Windows",
  linux: "Linux",
  installer: "Full installer",
  packages: "Packages on GitHub",
  every: "All platforms",
  like: "Like it? Follow ",
  likeOn: " on X and ",
  star: "star the project",
  likeEnd: " on GitHub.",
  count: "{n} downloads so far.",
  source: "Source",
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
  privacy: "Privacy",
  terms: "Terms",
};

const zh: typeof en = {
  tagline: "七个桌面应用，断网也能用。",
  ElyDoc: "信件、报告、书稿。打开并保存 Word 文件。",
  ElyTable: "公式、筛选、图表。打开并保存 Excel 文件。",
  ElyShow: "幻灯片、动画、演讲者视图。打开 PowerPoint 文件。",
  ElyPDF: "阅读、批注、签名、填表。扫描件里也能查字。",
  ElyMail: "Gmail、Outlook、iCloud 与 IMAP 收在一个收件箱。断网也能读信、搜索。",
  ElyTree: "按行暂存、提交、看提交图。不用终端的 Git。",
  ElyMark: "写的就是看到的 Markdown。公式与图表就地显示。",
  offline: "断网照常工作。",
  open: "开源，MIT 许可。",
  local: "文件留在你自己的文件夹里。",
  download: "下载",
  platforms: "macOS、Windows 与 Linux 版已发布。",
  needs: "macOS 14 或更新，Apple 芯片或 Intel。",
  forMacArm: "下载 Mac 版（Apple 芯片）",
  forMacIntel: "下载 Mac 版（Intel）",
  forWindows: "下载 Windows 版",
  forLinux: "下载 Linux 版",
  whole: "一个安装包，七个应用都在里面。",
  apart: "只需要其中几个应用？到 ",
  apartEnd: " 单独下载。",
  macArm: "Mac，Apple 芯片",
  macIntel: "Mac，Intel",
  windows: "Windows",
  linux: "Linux",
  installer: "完整安装包",
  packages: "GitHub 上的安装包",
  every: "全部平台",
  like: "喜欢的话，欢迎在 X 关注 ",
  likeOn: "，并给项目",
  star: "点个 Star",
  likeEnd: "。",
  count: "已下载 {n} 次。",
  source: "源码",
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
  privacy: "隐私政策",
  terms: "服务条款",
};

export type Key = keyof typeof en;
export const TEXT: Record<Lang, typeof en> = { en, zh };

/** An element whose text follows the language. */
export function said<K extends keyof HTMLElementTagNameMap>(tag: K, key: Key): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  el.dataset.t = key;
  return el;
}

/** `key` in `lang`, with `n` written for {n}. */
export function word(lang: Lang, key: Key, n?: number): string {
  const text = TEXT[lang][key];
  return n === undefined ? text : text.replace("{n}", n.toLocaleString(lang === "zh" ? "zh-CN" : "en"));
}

/** The language the page is written in. */
export function spoken(): Lang {
  return document.documentElement.lang === "zh-CN" ? "zh" : "en";
}

/** Writes every keyed text in `lang`. */
export function speak(lang: Lang): void {
  document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
  for (const el of document.querySelectorAll<HTMLElement>("[data-t]")) {
    const { n } = el.dataset;
    el.textContent = word(lang, el.dataset.t as Key, n === undefined ? undefined : Number(n));
  }
}
