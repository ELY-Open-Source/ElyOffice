// Answers /release.json with the newest release and downloads.
const REPO = "https://api.github.com/repos/ELY-Open-Source/ElyOffice";
// The old home keeps the downloads made before the move.
const GITEA = "https://git.zacharyzhang.com/api/v1/repos/ZacharyZhang-NY/ElyOffice";
const PAGE = 100;
const GITEA_PAGE = 50;
// A slow Gitea must not hold up the page.
const GITEA_WAIT = 5000;

type Cached = RequestInit & { cf: { cacheTtl: number; cacheEverything: boolean } };
type Asset = { name: string; download_count: number; browser_download_url: string };
type Release = { tag_name: string; assets: Asset[] };
// A token lifts GitHub's limit: `wrangler secret put GITHUB_TOKEN`.
type Env = { GITHUB_TOKEN?: string };

const cf = { cacheTtl: 300, cacheEverything: true };

async function read<T>(env: Env, path: string): Promise<T> {
  const headers: Record<string, string> = {
    accept: "application/vnd.github+json",
    "user-agent": "ely-office-site",
    "x-github-api-version": "2022-11-28",
  };
  if (env.GITHUB_TOKEN) headers.authorization = `Bearer ${env.GITHUB_TOKEN}`;
  const init: Cached = { headers, cf };
  const res = await fetch(`${REPO}${path}`, init);
  if (!res.ok) throw new Error(`GitHub answered ${res.status} for ${path}`);
  return (await res.json()) as T;
}

async function readGitea<T>(path: string): Promise<T> {
  const init: Cached = {
    headers: { accept: "application/json", "user-agent": "ely-office-site" },
    cf,
    signal: AbortSignal.timeout(GITEA_WAIT),
  };
  const res = await fetch(`${GITEA}${path}`, init);
  if (!res.ok) throw new Error(`Gitea answered ${res.status} for ${path}`);
  return (await res.json()) as T;
}

// The full installers: every app in one download.
const BUNDLES = {
  macArm: (v: string) => `ElyOffice-${v}-arm64.dmg`,
  macIntel: (v: string) => `ElyOffice-${v}-x86_64.dmg`,
  windows: (v: string) => `ElyOffice-${v}-windows-x86_64-setup.exe`,
};
export type Bundle = keyof typeof BUNDLES;

// Own and upstream source archives; their names carry the commit.
const SOURCES = { own: "-source.tar.gz", upstream: "-upstream-source.tar.gz" };
export type Source = keyof typeof SOURCES;

/** `name` is version `v`'s archive ending `end`, a commit between. */
function archive(name: string, v: string, end: string): boolean {
  const start = `ElyOffice-${v}-`;
  return name.startsWith(start) && name.endsWith(end) && /^[0-9a-f]+$/.test(name.slice(start.length, -end.length));
}

type Newest = { version: string; installers: Partial<Record<Bundle, string>>; sources: Partial<Record<Source, string>> };

/** Where `asset` of `tag` downloads; elsewhere is an error. */
function url(tag: string, asset: Asset): string {
  const url = `https://github.com/ELY-Open-Source/ElyOffice/releases/download/${tag}/${asset.name}`;
  if (asset.browser_download_url !== url) throw new Error(`${asset.name} downloads from ${asset.browser_download_url}`);
  return url;
}

/** The newest release's version, without v, installers and sources. */
async function newest(env: Env): Promise<Newest> {
  const { tag_name: tag, assets } = await read<Release>(env, "/releases/latest");
  if (!/^v\d+\.\d+\.\d+$/.test(tag)) throw new Error(`GitHub's latest tag is ${JSON.stringify(tag)}`);
  const version = tag.slice(1);
  const installers: Partial<Record<Bundle, string>> = {};
  // A building release lacks some; those link to the release.
  for (const [bundle, name] of Object.entries(BUNDLES) as [Bundle, (v: string) => string][]) {
    const asset = assets.find((a) => a.name === name(version));
    if (asset) installers[bundle] = url(tag, asset);
  }
  const sources: Partial<Record<Source, string>> = {};
  for (const [source, end] of Object.entries(SOURCES) as [Source, string][]) {
    const found = assets.filter((a) => archive(a.name, version, end));
    if (found.length > 1) throw new Error(`${tag} has ${found.length} ${source} source archives`);
    if (found.length) sources[source] = url(tag, found[0]);
  }
  return { version, installers, sources };
}

/** Downloads of `releases`; update manifests are checks, not downloads. */
function counted(releases: Release[]): number {
  let total = 0;
  for (const { assets } of releases) {
    for (const a of assets) {
      if (!Number.isInteger(a.download_count)) throw new Error(`${a.name} has no download count`);
      if (!a.name.endsWith(".json")) total += a.download_count;
    }
  }
  return total;
}

/** Downloads of every GitHub release. */
async function downloads(env: Env): Promise<number> {
  let total = 0;
  for (let page = 1; ; page++) {
    const releases = await read<Release[]>(env, `/releases?per_page=${PAGE}&page=${page}`);
    total += counted(releases);
    if (releases.length < PAGE) return total;
  }
}

/** Gitea releases' downloads; 0 when Gitea is down or odd. */
async function giteaDownloads(): Promise<number> {
  try {
    let total = 0;
    for (let page = 1; ; page++) {
      const releases = await readGitea<Release[]>(`/releases?limit=${GITEA_PAGE}&page=${page}`);
      if (!Array.isArray(releases)) throw new Error("Gitea listed no releases");
      total += counted(releases);
      if (releases.length < GITEA_PAGE) return total;
    }
  } catch (e) {
    // The page shows GitHub's count alone.
    console.warn(`no Gitea downloads: ${e}`);
    return 0;
  }
}

export default {
  async fetch(_request: Request, env: Env): Promise<Response> {
    try {
      const [{ version, installers, sources }, n, old] = await Promise.all([newest(env), downloads(env), giteaDownloads()]);
      return Response.json({ version, installers, sources, downloads: n + old }, { headers: { "cache-control": "public, max-age=300" } });
    } catch (e) {
      return new Response(String(e), { status: 502 });
    }
  },
};
