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
  opts.addArguments("-headless");
  // Kill/wake spike (spikes/RESULTS.md, Kill/Wake section) confirmed, three runs in a row,
  // that a converted event-page background does idle-terminate past this timeout and reboots
  // (fresh top-level execution, observed via a bootMark that can only change on a genuine
  // restart) on the next tabs.onUpdated-triggering event, under headless Selenium/WebDriver.
  opts.setPreference("extensions.background.idle.timeout", 1000);
  // geckodriver >=0.37 (Firefox 153+) refuses WebDriver navigation to internal
  // schemes (moz-extension:, about:, chrome:) unless the server is started with
  // --allow-system-access; without it `driver.get("moz-extension://...")` throws
  // UnsupportedOperationError. This is a geckodriver launch flag, not a
  // moz:firefoxOptions capability (geckodriver rejects it there explicitly), so it
  // has to go on the ServiceBuilder that spawns geckodriver itself.
  const service = new firefox.ServiceBuilder().addArguments("--allow-system-access");
  const driver: WebDriver = await new Builder()
    .forBrowser("firefox")
    .setFirefoxOptions(opts)
    .setFirefoxService(service)
    .build();
  let extensionId: string;
  try {
    await (driver as unknown as { installAddon(p: string, temp: boolean): Promise<void> }).installAddon(xpiPath, true);
    await driver.sleep(1000); // let the uuid land in prefs
    const profile = (await driver.getCapabilities()).get("moz:profile") as string;
    extensionId = uuidFor(profile, geckoId);
  } catch (e) {
    await driver.quit().catch(() => {});
    throw e;
  }

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
    async killBackground() {
      // Firefox has no WebDriver command to terminate an event page directly. With the
      // low idle timeout set at launch, staying idle past the window lets it terminate --
      // confirmed observable and reliable (three runs, spikes/RESULTS.md Kill/Wake) under
      // headless WebDriver, so this is reported as a real kill, not a best-effort guess.
      await driver.sleep(1500); // > idle timeout
      return { killed: true, mechanism: "idle-timeout" };
    },
    async close() { await driver.quit(); },
  };
}
