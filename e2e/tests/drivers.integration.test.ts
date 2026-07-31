import { describe, it, expect } from "vitest";
import { cpSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { launchChrome } from "../src/chromeDriver.js";
import { launchFirefox } from "../src/firefoxDriver.js";
import { instrumentExtension, zipDir } from "../src/injector.js";
import { startTelemetry } from "../src/telemetry.js";

const itIf = process.env.E2E_INTEGRATION ? it : it.skip;

describe("drivers + shim end to end", () => {
  itIf("chromium session produces background trace events", async () => {
    const t = await startTelemetry(41996);
    const dir = mkdtempSync(join(tmpdir(), "c2m-int-"));
    cpSync(resolve("testdata/hello-extension"), dir, { recursive: true });
    instrumentExtension(dir, "chrome-orig", 41996);
    const s = await launchChrome(dir);
    await new Promise((r) => setTimeout(r, 3000));
    await s.close();
    const apis = t.getEvents("chrome-orig").map((e) => e.api);
    expect(apis).toContain("storage.local.set"); // from onInstalled handler
    await t.close();
  }, 60000);

  itIf("firefox session produces background trace events", async () => {
    const t = await startTelemetry(41995);
    const dir = mkdtempSync(join(tmpdir(), "c2m-int-"));
    cpSync(resolve("testdata/hello-extension"), dir, { recursive: true });
    // convert the Chrome-shaped manifest by hand for the fixture: scripts background + gecko id
    const { readFileSync, writeFileSync } = await import("node:fs");
    const m = JSON.parse(readFileSync(join(dir, "manifest.json"), "utf8"));
    m.background = { scripts: ["background.js"] };
    m.browser_specific_settings = { gecko: { id: "c2m-hello@test" } };
    writeFileSync(join(dir, "manifest.json"), JSON.stringify(m));
    instrumentExtension(dir, "firefox-conv", 41995);
    const xpi = join(dir, "..", "hello.xpi");
    zipDir(dir, xpi);
    const s = await launchFirefox(xpi, "c2m-hello@test");
    await new Promise((r) => setTimeout(r, 3000));
    await s.close();
    const apis = t.getEvents("firefox-conv").map((e) => e.api);
    expect(apis).toContain("storage.local.set");
    await t.close();
  }, 60000);
});
