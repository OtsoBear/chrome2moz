import { chromium, type BrowserContext, type Page } from "playwright";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

export interface BrowserSession {
  extensionId: string;
  open(url: string): Promise<void>;
  pressChord(chord: string): Promise<void>;
  openExtensionPage(relPath: string): Promise<void>;
  screenshot(outPath: string): Promise<void>;
  close(): Promise<void>;
}

function mapChord(chord: string): string {
  return chord
    .split("+")
    .map((k) => ({ Ctrl: "Control", MacCtrl: "Control", Command: "Meta", Alt: "Alt", Shift: "Shift" }[k] ?? k))
    .join("+");
}

export async function launchChrome(extDir: string): Promise<BrowserSession> {
  const ctx: BrowserContext = await chromium.launchPersistentContext(
    mkdtempSync(join(tmpdir(), "c2m-chrome-")),
    {
      // headless: true alone (Playwright's own headless mode) never surfaced the extension's
      // service worker within the 15s wait below. --headless=new passed as an explicit arg
      // (with headless left false so Playwright doesn't also inject its own headless flag)
      // does start the service worker reliably — this is the form that actually works.
      headless: false,
      args: [
        "--headless=new",
        `--disable-extensions-except=${extDir}`,
        `--load-extension=${extDir}`,
        "--no-first-run",
      ],
    },
  );
  let extensionId: string;
  let page: Page;
  try {
    const sw = ctx.serviceWorkers()[0] ?? (await ctx.waitForEvent("serviceworker", { timeout: 15000 }));
    extensionId = new URL(sw.url()).host;
    page = ctx.pages()[0] ?? (await ctx.newPage());
  } catch (e) {
    await ctx.close().catch(() => {});
    throw e;
  }

  return {
    extensionId,
    async open(url) { page = await ctx.newPage(); await page.goto(url, { waitUntil: "domcontentloaded" }); },
    async pressChord(chord) { await page.keyboard.press(mapChord(chord)); },
    async openExtensionPage(relPath) {
      page = await ctx.newPage();
      await page.goto(`chrome-extension://${extensionId}/${relPath}`, { waitUntil: "domcontentloaded" });
    },
    async screenshot(outPath) { await page.screenshot({ path: outPath }); },
    async close() { await ctx.close(); },
  };
}
