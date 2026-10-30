import "./style.css";
import "./iso.css";
import "./tour.css";
import { tour } from "./tour.ts";
import { props, reveal } from "./regions.ts";
import { night } from "./night.ts";
import { rhythm } from "./rhythm.ts";
import { playground } from "./play.ts";
import { film } from "./film.ts";
import { said, speak, word, type Lang } from "./text.ts";
import { RELEASES, latest, releasePage } from "./release.ts";
import { downloads } from "./download.ts";
import { iconLink } from "./icons.ts";

const REPO = "https://github.com/ELY-Open-Source/ElyOffice";
const X = "https://x.com/ZacharyZhang";

const app = document.querySelector<HTMLElement>("#app");
if (!app) throw new Error("index.html has no #app");

const hero = document.createElement("header");
const title = document.createElement("h1");
title.textContent = "Ely Office";
// Until the release is read, its page holds the sources.
const source = iconLink("github", "source", RELEASES);
const every = said("a", "every");
every.href = "#download";
const links = document.createElement("div");
links.className = "links";
links.append(every, source);
const heroCount = said("p", "count");
heroCount.hidden = true;
const heroGet = downloads(false);
const like = document.createElement("p");
like.className = "small like";
const follow = document.createElement("a");
follow.href = X;
follow.textContent = "@ZacharyZhang";
const star = said("a", "star");
star.href = REPO;
like.append(said("span", "like"), follow, said("span", "likeOn"), star, said("span", "likeEnd"));
hero.append(title, said("p", "tagline"), said("p", "platforms"), heroCount, heroGet.block, links, like);
const { section, host, svg } = tour(hero);
rhythm(host, svg);

const shelf = props();

const footer = document.createElement("footer");
footer.id = "download";
const version = document.createElement("h2");
version.textContent = "Ely Office";
const count = said("p", "count");
count.hidden = true;
const get = downloads(true);
const legal = document.createElement("ul");
for (const [key, href] of [["privacy", "/privacy"], ["terms", "/termofservice"]] as const) {
  const a = said("a", key);
  a.href = href;
  const li = document.createElement("li");
  li.append(a);
  legal.append(li);
}
const own = said("a", "source");
const upstream = said("a", "upstream");
const sources = document.createElement("p");
sources.className = "sources";
sources.append(own, upstream);
for (const a of [own, upstream]) a.href = RELEASES;
footer.append(version, count, said("p", "platforms"), get.block, said("p", "needs"), sources, legal);

const toggle = said("button", "other");
toggle.className = "lang";
let lang: Lang = navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
/** Writes the page in `next`; the toggle names the other. */
const say = (next: Lang) => {
  lang = next;
  speak(lang);
  toggle.lang = lang === "en" ? "zh-CN" : "en";
};
toggle.addEventListener("click", () => say(lang === "en" ? "zh" : "en"));

const main = document.createElement("main");
main.append(section, playground(), shelf, film());
app.append(night(), toggle, main, footer);
say(lang);
reveal([shelf]);
void latest().then(({ version: v, installers, sources: archives, downloads: n }) => {
  version.textContent = `Ely Office ${v}`;
  // A missing archive sends readers to its release page.
  source.href = own.href = archives.own ?? releasePage(v);
  upstream.href = archives.upstream ?? releasePage(v);
  heroGet.found(installers);
  get.found(installers);
  for (const el of [heroCount, count]) {
    el.dataset.n = String(n);
    el.textContent = word(lang, "count", n);
    el.hidden = false;
  }
});
