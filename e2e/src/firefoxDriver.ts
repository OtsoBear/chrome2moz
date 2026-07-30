import { Builder, Key, type WebDriver } from "selenium-webdriver";
import firefox from "selenium-webdriver/firefox.js";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { BrowserSession } from "./chromeDriver.js";

function uuidFor(profileDir: string, geckoId: string): string {
  const prefs = readFileSync(join(profileDir, "prefs.js"), "utf8");
  const line = prefs.match(/user_pref\("extensions\.webextensions\.uuids",\s*"(.*)"\);/);
  if (!line) throw new Error("uuids pref not found in prefs.js");
  const map = JSON.parse(line[1].replace(/\\(.)/g, "$1"));
  const uuid = map[geckoId];
  if (!uuid) throw new Error(`no uuid for ${geckoId}; known: ${Object.keys(map).join(", ")}`);
  return uuid;
}

const KEYMAP: Record<string, string> = {
  Ctrl: Key.CONTROL, MacCtrl: Key.CONTROL, Command: Key.META, Alt: Key.ALT, Shift: Key.SHIFT,
};

export async function launchFirefox(xpiPath: string, geckoId: string): Promise<BrowserSession> {
  const opts = new firefox.Options();
  const driver: WebDriver = await new Builder().forBrowser("firefox").setFirefoxOptions(opts).build();
  await (driver as unknown as { installAddon(p: string, temp: boolean): Promise<void> }).installAddon(xpiPath, true);
  await driver.sleep(1000); // let the uuid land in prefs
  const profile = (await driver.getCapabilities()).get("moz:profile") as string;
  const extensionId = uuidFor(profile, geckoId);

  return {
    extensionId,
    async open(url) { await driver.switchTo().newWindow("tab"); await driver.get(url); },
    async pressChord(chord) {
      const keys = chord.split("+");
      const mods = keys.slice(0, -1).map((k) => KEYMAP[k] ?? k);
      const last = keys[keys.length - 1].toLowerCase();
      let a = driver.actions();
      for (const m of mods) a = a.keyDown(m);
      a = a.sendKeys(last);
      for (const m of [...mods].reverse()) a = a.keyUp(m);
      await a.perform();
    },
    async openExtensionPage(relPath) {
      await driver.switchTo().newWindow("tab");
      await driver.get(`moz-extension://${extensionId}/${relPath}`);
    },
    async screenshot(outPath) {
      const b64 = await driver.takeScreenshot();
      const { writeFileSync } = await import("node:fs");
      writeFileSync(outPath, Buffer.from(b64, "base64"));
    },
    async close() { await driver.quit(); },
  };
}
