// GitHub redirects this to the newest release.
export const RELEASES = "https://github.com/ELY-Open-Source/ElyOffice/releases/latest";

/** Full installers in the newest release; one still building lacks some. */
export type Installers = Partial<Record<"macArm" | "macIntel" | "windows", string>>;

export type Release = { version: string; installers: Installers; downloads: number };

/** The newest release and all downloads, from the worker. */
export async function latest(): Promise<Release> {
  const res = await fetch("/release.json");
  if (!res.ok) throw new Error(`release.json: ${res.status} ${await res.text()}`);
  return (await res.json()) as Release;
}
