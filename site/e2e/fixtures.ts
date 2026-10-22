import { test as base } from "@playwright/test";

export { expect, type Page } from "@playwright/test";

export const VERSION = "9.9.9";
export const DOWNLOADS = 12345;
const DL = `https://github.com/ELY-Open-Source/ElyOffice/releases/download/v${VERSION}`;
export const INSTALLERS = {
  macArm: `${DL}/ElyOffice-${VERSION}-arm64.dmg`,
  macIntel: `${DL}/ElyOffice-${VERSION}-x86_64.dmg`,
  windows: `${DL}/ElyOffice-${VERSION}-windows-x86_64-setup.exe`,
};

// Every page reads this version.
export const test = base.extend<{ release: void }>({
  release: [
    async ({ context }, use) => {
      await context.route("**/release.json", (r) => r.fulfill({ json: { version: VERSION, installers: INSTALLERS, downloads: DOWNLOADS } }));
      await use();
    },
    { auto: true },
  ],
});
