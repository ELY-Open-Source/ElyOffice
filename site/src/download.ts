// Full installers: the visitor's platform first, then every platform.
import { said, spoken, word, type Key } from "./text.ts";
import { ICONS } from "./icons.ts";
import { RELEASES, type Installers } from "./release.ts";

export type Platform = "macArm" | "macIntel" | "windows" | "linux";
const PLATFORMS: Platform[] = ["macArm", "macIntel", "windows", "linux"];

const BUTTON: Record<Platform, Key> = { macArm: "forMacArm", macIntel: "forMacIntel", windows: "forWindows", linux: "forLinux" };
const CARD: Record<Platform, Key> = { macArm: "macArm", macIntel: "macIntel", windows: "windows", linux: "linux" };
const GLYPH: Record<Platform, keyof typeof ICONS> = { macArm: "apple", macIntel: "apple", windows: "windows", linux: "linux" };

type Hints = { platform: string; getHighEntropyValues(hints: string[]): Promise<{ architecture?: string }> };

/** The visitor's platform by user agent; Macs guess Apple silicon. */
export function guess(ua = navigator.userAgent): Platform {
  if (/Windows/.test(ua)) return "windows";
  if (/Macintosh|Mac OS X/.test(ua) && !/iPhone|iPad/.test(ua)) return "macArm";
  if (/Linux|X11|CrOS/.test(ua) && !/Android/.test(ua)) return "linux";
  return "windows";
}

/** Chromium on Macs names the chip; others keep the guess. */
async function chip(): Promise<Platform | null> {
  const hints = (navigator as Navigator & { userAgentData?: Hints }).userAgentData;
  if (hints?.platform !== "macOS") return null;
  const { architecture } = await hints.getHighEntropyValues(["architecture"]);
  if (architecture === "x86") return "macIntel";
  if (architecture === "arm") return "macArm";
  return null;
}

/** Where `p` downloads: its installer, else the release page. */
const target = (p: Platform, installers: Installers | null) => (p === "linux" ? RELEASES : installers?.[p] ?? RELEASES);

/** Download block: this platform's button, a note on single apps. */
export function downloads(cards: boolean) {
  let platform = guess();
  let installers: Installers | null = null;

  const block = document.createElement("div");
  block.className = "downloads";
  const button = document.createElement("a");
  button.className = "button";
  button.innerHTML = ICONS.download;
  button.firstElementChild!.setAttribute("aria-hidden", "true");
  const label = said("span", BUTTON[platform]);
  button.append(label);
  const whole = said("p", "whole");
  whole.className = "small";

  const apart = document.createElement("p");
  apart.className = "small";
  const releases = document.createElement("a");
  releases.href = RELEASES;
  releases.textContent = "GitHub Releases";
  apart.append(said("span", "apart"), releases, said("span", "apartEnd"));
  block.append(button, whole, apart);

  const links = PLATFORMS.map((p) => {
    const card = document.createElement("a");
    card.className = "card";
    card.dataset.platform = p;
    card.innerHTML = ICONS[GLYPH[p]];
    card.firstElementChild!.setAttribute("aria-hidden", "true");
    card.append(said("span", CARD[p]), said("span", p === "linux" ? "packages" : "installer"));
    return card;
  });
  if (cards) {
    const grid = document.createElement("div");
    grid.className = "cards";
    grid.append(...links);
    block.append(grid);
  }

  const point = () => {
    label.dataset.t = BUTTON[platform];
    label.textContent = word(spoken(), BUTTON[platform]);
    button.dataset.platform = platform;
    button.href = target(platform, installers);
    whole.hidden = platform === "linux";
    for (const card of links) card.href = target(card.dataset.platform as Platform, installers);
  };
  point();
  void chip().then((p) => {
    if (!p) return;
    platform = p;
    point();
  });

  return {
    block,
    /** Points every link at the newest release's installers. */
    found(next: Installers) {
      installers = next;
      point();
    },
  };
}
