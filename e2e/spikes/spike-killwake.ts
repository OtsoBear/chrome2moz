// Spike: can we force-terminate the extension background in each browser under headless
// automation, and observe it reboot on the next event? Confirms the Chromium CDP mechanism
// and settles whether Firefox's converted event-page idle-termination is confirmable under
// headless WebDriver (per the kill/wake plan, Task 1). Always headless -- see Global
// Constraints in docs/superpowers/plans/2026-09-17-killwake-probe.md.
import { chromium } from "playwright";
import { Builder } from "selenium-webdriver";
import firefox from "selenium-webdriver/firefox.js";
import AdmZip from "adm-zip";
import http from "node:http";
import { cpSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

// Background top-level code that both sides share (functionally): record a boot into
// storage.local (persists across background restarts) with a random per-boot mark, and
// register a top-level tabs.onUpdated listener so that opening a new tab is the wake
// trigger -- this is exactly the killwake-gate design from Task 4, prototyped here to
// decide whether it is observable at all under headless automation.
// bootMark is computed ONCE, synchronously, at top-level -- it changes only when the whole
// script re-executes (a genuine worker/event-page restart). If it instead changed inside
// recordBoot() (regenerated on every call), it could not distinguish "the same live worker
// handled another event" from "the worker restarted and handled an event" -- both would show
// a new mark on every call. boots counts every recordBoot() call (restart or not); bootMark
// is the actual reboot signal.
const BACKGROUND_JS = `
const bootMark = Date.now() + ":" + Math.random();
async function recordBoot(reason) {
  const cur = await chrome.storage.local.get(["boots"]);
  const boots = (cur.boots || 0) + 1;
  await chrome.storage.local.set({ boots, bootMark, lastWake: reason });
}
recordBoot("startup");
chrome.tabs.onUpdated.addListener((_tabId, info) => {
  if (info.status === "complete") recordBoot("tabs.onUpdated");
});
`;

// Firefox-only: the content script has no direct hook back into Node, so report boot state
// by fetching a query string to the spike's local HTTP server -- same telemetry shape the
// existing spikes and the real shim already use for observability, just simplified for this
// throwaway measurement.
const CONTENT_JS_REPORT = (port: number) => `
chrome.storage.local.get(["boots", "bootMark", "lastWake"]).then((v) => {
  fetch("http://127.0.0.1:${port}/boot?boots=" + v.boots + "&mark=" + v.bootMark + "&wake=" + v.lastWake).catch(() => {});
});
`;

async function chromiumSpike() {
  console.log("\n=== Chromium: CDP ServiceWorker.stopAllWorkers ===");
  const dir = mkdtempSync(join(tmpdir(), "c2m-kill-chrome-"));
  cpSync(resolve("testdata/hello-extension"), dir, { recursive: true });
  const manifestPath = join(dir, "manifest.json");
  const m = JSON.parse(readFileSync(manifestPath, "utf8"));
  m.permissions = Array.from(new Set([...(m.permissions ?? []), "tabs"]));
  writeFileSync(manifestPath, JSON.stringify(m));
  writeFileSync(join(dir, "background.js"), BACKGROUND_JS);

  const ctx = await chromium.launchPersistentContext(mkdtempSync(join(tmpdir(), "c2m-chrome-prof-"), ), {
    headless: false,
    args: [
      "--headless=new",
      `--disable-extensions-except=${dir}`,
      `--load-extension=${dir}`,
      "--no-first-run",
    ],
  });
  try {
    const sw0 = ctx.serviceWorkers()[0] ?? (await ctx.waitForEvent("serviceworker", { timeout: 15000 }));
    // Settle generously first: launchPersistentContext opens an implicit initial about:blank
    // tab, which itself fires a tabs.onUpdated the extension's listener will catch -- let that
    // land before taking the "before" reading, so the delta after the explicit wake trigger
    // below is attributable to exactly that one trigger.
    await new Promise((r) => setTimeout(r, 3000));
    const before = await sw0.evaluate(() => (globalThis as any).chrome.storage.local.get(["boots", "bootMark"]));
    console.log("boot state before kill:", before);

    // ServiceWorker.* is a page-target-scoped CDP domain (browser-level sessions don't expose
    // it), so attach to the already-existing implicit about:blank tab -- confirmed above (boots
    // stayed 1 through the settle wait) that merely having this tab open does not itself fire
    // tabs.onUpdated, so this doesn't confound the boot count.
    const target = ctx.pages()[0];
    const cdp = await ctx.newCDPSession(target);
    await cdp.send("ServiceWorker.enable");
    await cdp.send("ServiceWorker.stopAllWorkers");
    await cdp.detach().catch(() => {});

    // Poll for the old worker to disappear from the context's live-worker list (Playwright
    // removes a worker's handle once its target actually detaches).
    let clearedAfterMs = -1;
    for (let waited = 0; waited <= 3000; waited += 250) {
      if (!ctx.serviceWorkers().includes(sw0)) { clearedAfterMs = waited; break; }
      await new Promise((r) => setTimeout(r, 250));
    }
    console.log(
      "old worker handle cleared from context.serviceWorkers():",
      clearedAfterMs >= 0 ? `yes, within ${clearedAfterMs}ms` : "no, still present after 3000ms",
    );

    // Wake trigger: open exactly one new tab. If the SW registration (and its top-level
    // listeners) survived termination, Chrome spins up a service worker to deliver
    // tabs.onUpdated -- new object identity if genuinely a new worker instance.
    const wakePromise = ctx.waitForEvent("serviceworker", { timeout: 10000 }).catch(() => null);
    await ctx.newPage();
    await new Promise((r) => setTimeout(r, 1500));
    const sw1 = ctx.serviceWorkers()[0] ?? (await wakePromise);
    if (!sw1) {
      console.log("verdict: NO service worker observed after wake trigger -- reboot NOT confirmed");
    } else {
      const after = await sw1.evaluate(() => (globalThis as any).chrome.storage.local.get(["boots", "bootMark", "lastWake"]));
      console.log("boot state after wake:", after);
      const newObjectIdentity = sw1 !== sw0;
      const freshTopLevelRun = after.bootMark !== before.bootMark;
      // A genuinely killed-and-respawned worker fires its top-level `recordBoot("startup")`
      // AND then delivers the pending tabs.onUpdated to the freshly-registered listener --
      // boots advances by 2 for one wake trigger (not 1, which would mean a still-alive
      // worker merely handled one more event without restarting).
      const twoCallDelta = after.boots === before.boots + 2;
      console.log("new Playwright worker object identity:", newObjectIdentity, "(Playwright may reuse the wrapper across a respawn; not authoritative on its own)");
      console.log("bootMark changed (fresh top-level script execution):", freshTopLevelRun);
      console.log("boots advanced by 2 (startup + the waking event):", twoCallDelta, after.boots, "vs before", before.boots);
      console.log(
        "VERDICT: Chromium kill+reboot confirmed =",
        freshTopLevelRun && twoCallDelta && after.lastWake === "tabs.onUpdated",
      );
    }
  } finally {
    await ctx.close();
  }
}

async function firefoxSpike() {
  console.log("\n=== Firefox: event-page idle termination ===");
  const port = 41811;
  const hits: string[] = [];
  const server = http
    .createServer((req, res) => {
      if (req.url === "/page") {
        res.setHeader("content-type", "text/html");
        res.end("<html><body>fixture</body></html>");
        return;
      }
      hits.push(req.url ?? "");
      res.setHeader("access-control-allow-origin", "*");
      res.end("ok");
    })
    .listen(port);

  const dir = mkdtempSync(join(tmpdir(), "c2m-kill-ff-"));
  cpSync(resolve("testdata/hello-extension"), dir, { recursive: true });
  const manifestPath = join(dir, "manifest.json");
  const m = JSON.parse(readFileSync(manifestPath, "utf8"));
  m.background = { scripts: ["background.js"] };
  m.browser_specific_settings = { gecko: { id: "c2m-killwake@test" } };
  m.permissions = Array.from(new Set([...(m.permissions ?? []), "tabs"]));
  m.host_permissions = Array.from(new Set([...(m.host_permissions ?? []), `http://127.0.0.1/*`]));
  writeFileSync(manifestPath, JSON.stringify(m));
  writeFileSync(join(dir, "background.js"), BACKGROUND_JS);
  writeFileSync(join(dir, "content.js"), CONTENT_JS_REPORT(port));

  const xpi = join(dir, "..", "killwake.xpi");
  const zip = new AdmZip();
  zip.addLocalFolder(dir);
  zip.writeZip(xpi);

  const opts = new firefox.Options();
  opts.addArguments("-headless");
  opts.setPreference("extensions.background.idle.timeout", 1000);
  const service = new firefox.ServiceBuilder().addArguments("--allow-system-access");
  const driver = await new Builder().forBrowser("firefox").setFirefoxOptions(opts).setFirefoxService(service).build();
  try {
    await (driver as unknown as { installAddon(p: string, temp: boolean): Promise<void> }).installAddon(xpi, true);
    await driver.sleep(1000);

    // Initial boot: navigate the fixture page so the content script reports boot state. Give
    // it extra settle time first so any implicit initial-tab tabs.onUpdated (the browser opens
    // about:blank on launch, same as Chromium) lands before this baseline read.
    await driver.sleep(2000);
    await driver.get(`http://127.0.0.1:${port}/page`);
    await driver.sleep(2000);
    const parseHit = (h: string) => {
      const m = h.match(/boots=(\d+)&mark=([^&]+)&wake=(\w+)/);
      return m ? { boots: Number(m[1]), mark: m[2], wake: m[3] } : null;
    };
    const before = parseHit(hits[hits.length - 1] ?? "");
    console.log("hits after initial boot:", [...hits], "parsed:", before);

    // Idle window: past the 1000ms idle.timeout pref, stay idle (no navigation, no extension
    // activity) long enough for Firefox to consider terminating the event page, if it does at
    // all under this automation setup.
    await driver.sleep(4000);

    // Wake trigger: open a new tab (fires tabs.onUpdated if the background is alive or if its
    // listener registration survived a termination) then navigate the fixture page again so
    // the content script reports the (possibly rebooted) boot state.
    hits.length = 0;
    await driver.switchTo().newWindow("tab");
    await driver.get(`http://127.0.0.1:${port}/page`);
    await driver.sleep(2000);
    const after = parseHit(hits[hits.length - 1] ?? "");
    console.log("hits after wake trigger:", [...hits], "parsed:", after);

    if (!before || !after) {
      console.log("VERDICT: could not parse boot state from either side -- inconclusive, treat as kill-unsupported");
    } else {
      const freshTopLevelRun = after.mark !== before.mark;
      console.log("bootMark changed (fresh top-level script execution):", freshTopLevelRun, `(${before.mark} -> ${after.mark})`);
      console.log("boots delta:", after.boots - before.boots, `(${before.boots} -> ${after.boots})`);
      console.log("VERDICT: Firefox idle-termination + reboot confirmed =", freshTopLevelRun);
    }
  } finally {
    await driver.quit();
    server.close();
  }
}

await chromiumSpike();
await firefoxSpike();
