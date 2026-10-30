// GitHub redirects this to the newest release.
export const RELEASES = "https://github.com/ELY-Open-Source/ElyOffice/releases/latest";

/** The newest release's full installers; one still building lacks some. */
export type Installers = Partial<Record<"macArm" | "macIntel" | "windows", string>>;

/** The release's own source archive and its patched upstreams. */
export type Sources = Partial<Record<"own" | "upstream", string>>;

export type Release = { version: string; installers: Installers; sources: Sources; downloads: number };

/** A version's release page: never a moving branch. */
export const releasePage = (version: string) => `https://github.com/ELY-Open-Source/ElyOffice/releases/tag/v${version}`;

/** The newest release and all downloads, from the worker. */
export async function latest(): Promise<Release> {
  const res = await fetch("/release.json");
  if (!res.ok) throw new Error(`release.json: ${res.status} ${await res.text()}`);
  return (await res.json()) as Release;
}
