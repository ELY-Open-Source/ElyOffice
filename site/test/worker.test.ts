import { afterEach, test } from "node:test";
import assert from "node:assert/strict";
import worker from "../worker/index.ts";

const real = globalThis.fetch;
afterEach(() => {
  globalThis.fetch = real;
});

const DL = "https://github.com/ELY-Open-Source/ElyOffice/releases/download/v0.1.9";
const asset = (name: string, download_count: number) => ({ name, download_count, browser_download_url: `${DL}/${name}` });
const github = {
  tag_name: "v0.1.9",
  assets: [
    asset("ElyDoc-0.1.9-arm64.dmg", 3),
    asset("ely-doc.json", 50),
    asset("ElyOffice-0.1.9-arm64.dmg", 20),
    asset("ElyOffice-0.1.9-x86_64.dmg", 4),
    asset("ElyOffice-0.1.9-windows-x86_64-setup.exe", 30),
  ],
};
const installers = {
  macArm: `${DL}/ElyOffice-0.1.9-arm64.dmg`,
  macIntel: `${DL}/ElyOffice-0.1.9-x86_64.dmg`,
  windows: `${DL}/ElyOffice-0.1.9-windows-x86_64-setup.exe`,
};
const gitea = [
  { tag_name: "v0.1.8", assets: [{ name: "ElyDoc-0.1.8-arm64.dmg", download_count: 40 }, { name: "ely-doc.json", download_count: 900 }] },
  { tag_name: "v0.1.7", assets: [{ name: "ElyDoc-0.1.7-arm64.dmg", download_count: 7 }] },
];

/** Answers GitHub with `newest` and Gitea with `old`. */
function hosts(old: () => Response, newest = github) {
  globalThis.fetch = (async (input: RequestInfo | URL) => {
    const url = String(input);
    if (url.startsWith("https://git.zacharyzhang.com/")) return old();
    if (url.endsWith("/releases/latest")) return Response.json(newest);
    return Response.json([newest]);
  }) as typeof fetch;
}

async function answer() {
  const warn = console.warn;
  console.warn = () => {};
  try {
    const res = await worker.fetch(new Request("https://office.elygpui.com/release.json"), {});
    assert.equal(res.status, 200);
    return await res.json();
  } finally {
    console.warn = warn;
  }
}

test("downloads add Gitea's history to GitHub's, without manifests", async () => {
  hosts(() => Response.json(gitea));
  assert.deepEqual(await answer(), { version: "0.1.9", installers, downloads: 3 + 20 + 4 + 30 + 40 + 7 });
});

test("a release still building names only the installers it has", async () => {
  const building = { ...github, assets: github.assets.filter((a) => !a.name.endsWith(".dmg")) };
  hosts(() => Response.json([]), building);
  assert.deepEqual(await answer(), { version: "0.1.9", installers: { windows: installers.windows }, downloads: 30 });
});

test("an installer served from elsewhere is an error", async () => {
  const moved = { ...github, assets: [{ ...github.assets[4], browser_download_url: "https://example.com/setup.exe" }] };
  hosts(() => Response.json([]), moved);
  const res = await worker.fetch(new Request("https://office.elygpui.com/release.json"), {});
  assert.equal(res.status, 502);
  assert.match(await res.text(), /setup\.exe downloads from https:\/\/example\.com/);
});

test("an unreachable, failing or odd Gitea leaves GitHub's count", async () => {
  const cases: (() => Response)[] = [
    () => {
      throw new TypeError("fetch failed");
    },
    () => new Response("down", { status: 502 }),
    () => new Response("<html>", { status: 200 }),
    () => Response.json({ message: "not found" }),
    () => Response.json([{ tag_name: "v0.1.8", assets: [{ name: "ElyDoc.dmg" }] }]),
  ];
  for (const old of cases) {
    hosts(old);
    assert.deepEqual(await answer(), { version: "0.1.9", installers, downloads: 3 + 20 + 4 + 30 });
  }
});
