# E2E Differential Testing — Plan 1: Core Pipeline

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A working differential e2e harness: for each corpus extension, load the instrumented original in Chromium and the instrumented converted build in Firefox, run identical probes, diff API traces, fail CI on unallowed divergence.

**Architecture:** TS harness in `e2e/` (pnpm). A plain-JS spy shim is injected into both builds by one TS injector. Shim streams API-call events to a localhost telemetry server. Playwright drives Chromium, selenium-webdriver drives Firefox. A normalizer + LCS diff compares per-context traces; `allowed_diffs` globs filter expected divergence. Corpus v1: LatexToCalc (local submodule) + OneNote Web Clipper (CWS regression entry).

**Tech Stack:** TypeScript, pnpm, tsx, vitest, Playwright (Chromium only), selenium-webdriver + geckodriver (Selenium Manager auto-fetch), adm-zip, picomatch. Rust converter invoked as a subprocess — no Rust changes.

**Spec:** `docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md`. This plan covers the pipeline core. Plan 2 (later): mitmproxy snapshots + domain discovery. Plan 3 (later): three-way baseline run, README table/badge, V8 coverage, clipboard readback.

## Global Constraints

- pnpm only (never npm/yarn); Python tooling via uv only
- `e2e/` is self-contained; nothing in it is imported by the Rust crate
- Shim must be transparent: wrap existing properties only, NEVER add missing namespaces (feature detection must be unaffected)
- Shim-internal messages (marker key `__c2m__`) are never recorded in traces
- Both sides get identical shim + identical manifest additions (host permission for telemetry) so instrumentation itself cannot cause divergence
- Node ≥ 20, `"type": "module"` everywhere in `e2e/`
- All commands below run from `e2e/` unless a path is shown

---

### Task 1: Scaffold `e2e/` package + hello-extension test fixture

**Files:**
- Create: `e2e/package.json`, `e2e/tsconfig.json`, `e2e/.gitignore`
- Create: `e2e/testdata/hello-extension/manifest.json`, `e2e/testdata/hello-extension/background.js`, `e2e/testdata/hello-extension/content.js`, `e2e/testdata/hello-extension/popup.html`, `e2e/testdata/hello-extension/popup.js`

The hello extension is a minimal MV3 extension used by spikes and driver/injector tests. It exercises: background boot, storage write, content script, a command, a popup.

- [ ] **Step 1: Create package files**

`e2e/package.json`:
```json
{
  "name": "chrome2moz-e2e",
  "private": true,
  "type": "module",
  "scripts": {
    "test": "vitest run",
    "e2e": "tsx src/run.ts"
  },
  "devDependencies": {
    "@types/adm-zip": "^0.5.7",
    "@types/node": "^24.0.0",
    "@types/picomatch": "^4.0.2",
    "@types/selenium-webdriver": "^4.1.28",
    "adm-zip": "^0.5.16",
    "picomatch": "^4.0.2",
    "playwright": "^1.54.0",
    "selenium-webdriver": "^4.34.0",
    "tsx": "^4.20.0",
    "typescript": "^5.8.0",
    "vitest": "^3.2.0"
  }
}
```

`e2e/tsconfig.json`:
```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true
  },
  "include": ["src", "tests"]
}
```

`e2e/.gitignore`:
```
node_modules/
.cache/
results/
```

- [ ] **Step 2: Create hello extension**

`e2e/testdata/hello-extension/manifest.json`:
```json
{
  "manifest_version": 3,
  "name": "C2M Hello",
  "version": "1.0",
  "background": { "service_worker": "background.js" },
  "content_scripts": [{ "matches": ["http://127.0.0.1/*"], "js": ["content.js"] }],
  "action": { "default_popup": "popup.html" },
  "permissions": ["storage"],
  "commands": {
    "hello-command": {
      "suggested_key": { "default": "Ctrl+Shift+9" },
      "description": "test command"
    }
  }
}
```

`e2e/testdata/hello-extension/background.js`:
```js
chrome.runtime.onInstalled.addListener(() => {
  chrome.storage.local.set({ installed: true });
});
chrome.commands.onCommand.addListener((cmd) => {
  chrome.storage.local.set({ lastCommand: cmd });
});
```

`e2e/testdata/hello-extension/content.js`:
```js
document.documentElement.dataset.c2mHello = "1";
```

`e2e/testdata/hello-extension/popup.html`:
```html
<!DOCTYPE html><html><body><span id="ok">hello</span><script src="popup.js"></script></body></html>
```

`e2e/testdata/hello-extension/popup.js`:
```js
chrome.storage.local.get("installed", () => {});
```

- [ ] **Step 3: Install and verify**

Run: `cd e2e && pnpm install && pnpm exec playwright install chromium && pnpm test`
Expected: install succeeds; vitest reports "no test files found" (exit 0 with `--passWithNoTests`; add that flag to the `test` script if needed).

- [ ] **Step 4: Commit**

```bash
git add e2e/
git commit -m "feat(e2e): scaffold harness package and hello-extension fixture"
```

---

### Task 2: Spike A — Chromium: load unpacked extension, detect SW, fetch to localhost

**Files:**
- Create: `e2e/spikes/spike-chromium.ts` (throwaway, committed for reference)
- Create: `e2e/spikes/RESULTS.md`

Goal: prove (1) Playwright persistent context loads an unpacked MV3 extension, (2) we can get the extension id from the service worker URL, (3) `fetch("http://127.0.0.1:PORT/...")` succeeds from the background SW and from a content script on a page served from localhost.

- [ ] **Step 1: Write spike script**

`e2e/spikes/spike-chromium.ts`:
```ts
import { chromium } from "playwright";
import http from "node:http";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

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
  .listen(41800);

const extDir = resolve("testdata/hello-extension");
// For the spike, temporarily add this to background.js first line:
//   fetch("http://127.0.0.1:41800/from-bg").catch(() => {});
// and to content.js:
//   fetch("http://127.0.0.1:41800/from-content").catch(() => {});
const ctx = await chromium.launchPersistentContext(mkdtempSync(join(tmpdir(), "c2m-")), {
  headless: false,
  args: [`--disable-extensions-except=${extDir}`, `--load-extension=${extDir}`, "--no-first-run"],
});
const sw = ctx.serviceWorkers()[0] ?? (await ctx.waitForEvent("serviceworker"));
console.log("extension id:", new URL(sw.url()).host);
const page = await ctx.newPage();
await page.goto("http://127.0.0.1:41800/page");
await page.waitForTimeout(2000);
console.log("telemetry hits:", hits);
await ctx.close();
server.close();
```

- [ ] **Step 2: Add the two fetch lines to the hello extension as noted, run spike**

Run: `pnpm exec tsx spikes/spike-chromium.ts`
Expected: prints a 32-char extension id and `telemetry hits: [ '/from-bg', '/from-content' ]` (order may vary).

- [ ] **Step 3: Record findings, revert hello-extension edits**

Write what worked / any deviations (e.g. needed `--headless=new`, CORS preflight behavior) into `e2e/spikes/RESULTS.md` under `## Chromium`. Revert the temporary fetch lines in `background.js`/`content.js`.

- [ ] **Step 4: Commit**

```bash
git add e2e/spikes/
git commit -m "spike(e2e): chromium extension load + localhost telemetry confirmed"
```

---

### Task 3: Spike B — Firefox: temporary add-on install, internal UUID, localhost fetch

**Files:**
- Modify: `e2e/spikes/RESULTS.md`
- Create: `e2e/spikes/spike-firefox.ts`

Goal: prove (1) selenium-webdriver installs a zipped MV2-style event-page extension as a temporary add-on, (2) we can recover the `moz-extension://` UUID from the profile's `prefs.js`, (3) background + content fetches to localhost work. Requires a Firefox-compatible variant of the hello extension (background.scripts + gecko id), which is exactly what the converter produces — hand-write it here.

- [ ] **Step 1: Create Firefox variant of hello extension inline in the spike**

`e2e/spikes/spike-firefox.ts`:
```ts
import firefox from "selenium-webdriver/firefox.js";
import { Builder } from "selenium-webdriver";
import AdmZip from "adm-zip";
import http from "node:http";
import { readFileSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

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
  .listen(41801);

const manifest = {
  manifest_version: 3,
  name: "C2M Hello FF",
  version: "1.0",
  background: { scripts: ["background.js"] },
  content_scripts: [{ matches: ["http://127.0.0.1/*"], js: ["content.js"] }],
  browser_specific_settings: { gecko: { id: "c2m-hello@test" } },
  permissions: ["storage"],
  host_permissions: ["http://127.0.0.1/*"]
};
const zip = new AdmZip();
zip.addFile("manifest.json", Buffer.from(JSON.stringify(manifest)));
zip.addFile("background.js", Buffer.from('fetch("http://127.0.0.1:41801/from-bg").catch(()=>{});'));
zip.addFile("content.js", Buffer.from('fetch("http://127.0.0.1:41801/from-content").catch(()=>{});'));
const xpi = join(mkdtempSync(join(tmpdir(), "c2m-")), "hello.xpi");
writeFileSync(xpi, zip.toBuffer());

const driver = await new Builder().forBrowser("firefox").setFirefoxOptions(new firefox.Options()).build();
try {
  await (driver as any).installAddon(xpi, true);
  const caps = await driver.getCapabilities();
  const profile = caps.get("moz:profile") as string;
  await driver.get("http://127.0.0.1:41801/page");
  await driver.sleep(3000);
  const prefs = readFileSync(join(profile, "prefs.js"), "utf8");
  const m = prefs.match(/extensions\.webextensions\.uuids.*?"({.*?})\\?"/);
  console.log("uuids pref line found:", !!m);
  console.log("telemetry hits:", hits);
} finally {
  await driver.quit();
  server.close();
}
```

- [ ] **Step 2: Run spike**

Run: `pnpm exec tsx spikes/spike-firefox.ts`
Expected: no install error, `uuids pref line found: true`, hits include `/from-bg` and `/from-content`. Note: the uuids pref value is JSON-escaped inside prefs.js; the real parser (Task 8) must unescape it. If MV3 `background.scripts` fails in the installed Firefox version, retry with `"type": "module"` removed / MV2 fallback and record which shape worked.

- [ ] **Step 3: Record findings in `RESULTS.md` under `## Firefox`**

Include: exact Firefox version tested, whether MV3 event pages worked, the prefs.js escaping format observed.

- [ ] **Step 4: Commit**

```bash
git add e2e/spikes/
git commit -m "spike(e2e): firefox temporary addon + uuid recovery + localhost telemetry confirmed"
```

---

### Task 4: Spike C — synthetic key chords triggering extension commands

**Files:**
- Modify: `e2e/spikes/RESULTS.md`
- Create: `e2e/spikes/spike-commands.ts`

Goal: determine whether synthetic input triggers `chrome.commands.onCommand` in Chromium (Playwright `keyboard.press`) and Firefox (WebDriver Actions). The hello extension writes `lastCommand` to storage on command; the spike reads it back.

- [ ] **Step 1: Write spike**

`e2e/spikes/spike-commands.ts`:
```ts
import { chromium } from "playwright";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const extDir = resolve("testdata/hello-extension");
const ctx = await chromium.launchPersistentContext(mkdtempSync(join(tmpdir(), "c2m-")), {
  headless: false,
  args: [`--disable-extensions-except=${extDir}`, `--load-extension=${extDir}`, "--no-first-run"],
});
const sw = ctx.serviceWorkers()[0] ?? (await ctx.waitForEvent("serviceworker"));
const page = await ctx.newPage();
await page.goto("https://example.com");
await page.keyboard.press("Control+Shift+9");
await page.waitForTimeout(1000);
const stored = await sw.evaluate(() => chrome.storage.local.get("lastCommand"));
console.log("chromium onCommand fired:", stored);
await ctx.close();
```
Firefox half: after Task 3's install flow, send the chord with
```ts
await driver.actions().keyDown(Key.CONTROL).keyDown(Key.SHIFT).sendKeys("9").keyUp(Key.SHIFT).keyUp(Key.CONTROL).perform();
```
against a variant of the FF hello extension that includes the same `commands` manifest block and an `onCommand` listener writing to storage; read back via a content-script relay (content script fetches `/cmd-fired` on a storage.onChanged event) since Firefox has no SW evaluate.

- [ ] **Step 2: Run both halves, record verdict in `RESULTS.md` under `## Commands`**

Expected outcomes and the decision each implies:
- Both fire → commands probe is in scope as designed
- One/none fire → commands probe ships as `skipped: dispatch-unsupported` in reports (spec's noted fallback); note exactly which side failed. Do NOT block the plan on this — later tasks treat the commands probe as optional.

- [ ] **Step 3: Commit**

```bash
git add e2e/spikes/
git commit -m "spike(e2e): command chord dispatch verdict"
```

---

### Task 5: CRX download + CRX3→ZIP parsing

**Files:**
- Create: `e2e/src/crx.ts`, `e2e/src/fetchCrx.ts`
- Test: `e2e/tests/crx.test.ts`

**Interfaces:**
- Produces: `crxToZip(buf: Buffer): Buffer` (throws on unknown format); `fetchExtension(id: string, pinnedVersion: string | null, cacheDir: string): Promise<{ zipPath: string; version: string }>`

- [ ] **Step 1: Write failing tests**

`e2e/tests/crx.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import AdmZip from "adm-zip";
import { crxToZip } from "../src/crx.js";

function fakeCrx3(zipBuf: Buffer, headerLen = 8): Buffer {
  const head = Buffer.alloc(12 + headerLen);
  head.write("Cr24", 0, "ascii");
  head.writeUInt32LE(3, 4);
  head.writeUInt32LE(headerLen, 8);
  return Buffer.concat([head, zipBuf]);
}

function zipWithManifest(): Buffer {
  const z = new AdmZip();
  z.addFile("manifest.json", Buffer.from(JSON.stringify({ version: "2.1.0" })));
  return z.toBuffer();
}

describe("crxToZip", () => {
  it("strips a CRX3 header", () => {
    const zip = zipWithManifest();
    const out = crxToZip(fakeCrx3(zip));
    expect(out.equals(zip)).toBe(true);
  });
  it("passes through a plain zip", () => {
    const zip = zipWithManifest();
    expect(crxToZip(zip).equals(zip)).toBe(true);
  });
  it("rejects garbage", () => {
    expect(() => crxToZip(Buffer.from("nope"))).toThrow(/not a CRX/);
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `pnpm test`
Expected: FAIL — cannot find module `../src/crx.js`.

- [ ] **Step 3: Implement**

`e2e/src/crx.ts`:
```ts
export function crxToZip(buf: Buffer): Buffer {
  if (buf.length >= 4 && buf.readUInt32BE(0) === 0x504b0304) return buf; // already ZIP
  if (buf.length < 12 || buf.toString("ascii", 0, 4) !== "Cr24") {
    throw new Error("not a CRX or ZIP file");
  }
  const version = buf.readUInt32LE(4);
  if (version === 3) {
    const headerLen = buf.readUInt32LE(8);
    return buf.subarray(12 + headerLen);
  }
  if (version === 2) {
    const pubLen = buf.readUInt32LE(8);
    const sigLen = buf.readUInt32LE(12);
    return buf.subarray(16 + pubLen + sigLen);
  }
  throw new Error(`unsupported CRX version ${version}`);
}
```

`e2e/src/fetchCrx.ts`:
```ts
import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import AdmZip from "adm-zip";
import { crxToZip } from "./crx.js";

const CRX_URL = (id: string) =>
  `https://clients2.google.com/service/update2/crx?response=redirect&prodversion=126.0&acceptformat=crx3&x=id%3D${id}%26uc`;

export async function fetchExtension(
  id: string,
  pinnedVersion: string | null,
  cacheDir: string,
): Promise<{ zipPath: string; version: string }> {
  mkdirSync(cacheDir, { recursive: true });
  if (pinnedVersion) {
    const cached = join(cacheDir, `${id}-${pinnedVersion}.zip`);
    if (existsSync(cached)) return { zipPath: cached, version: pinnedVersion };
  }
  const res = await fetch(CRX_URL(id), { redirect: "follow" });
  if (!res.ok) throw new Error(`CRX download failed for ${id}: HTTP ${res.status}`);
  const zip = crxToZip(Buffer.from(await res.arrayBuffer()));
  const manifest = JSON.parse(new AdmZip(zip).readAsText("manifest.json"));
  const version: string = manifest.version;
  if (pinnedVersion && version !== pinnedVersion) {
    throw new Error(
      `${id}: CWS serves ${version} but corpus pins ${pinnedVersion} and no cached copy exists. ` +
        `Re-pin the corpus entry or restore the cached archive.`,
    );
  }
  const out = join(cacheDir, `${id}-${version}.zip`);
  writeFileSync(out, zip);
  return { zipPath: out, version };
}
```

- [ ] **Step 4: Run tests**

Run: `pnpm test`
Expected: 3 passing.

- [ ] **Step 5: Manual smoke of the real endpoint**

Run: `pnpm exec tsx -e "import('./src/fetchCrx.js').then(m => m.fetchExtension('gojbdfnpnhogfdgjbigejoaolejmgdhk', null, '.cache/crx').then(r => console.log(r)))"`
Expected: prints a zip path + OneNote Web Clipper's current version. Record that version — Task 12 pins it in corpus.json.

- [ ] **Step 6: Commit**

```bash
git add e2e/src/crx.ts e2e/src/fetchCrx.ts e2e/tests/crx.test.ts
git commit -m "feat(e2e): CRX download and CRX3-to-zip parsing"
```

---

### Task 6: Telemetry server

**Files:**
- Create: `e2e/src/telemetry.ts`
- Test: `e2e/tests/telemetry.test.ts`

**Interfaces:**
- Produces:
  - `startTelemetry(port: number): Promise<Telemetry>`
  - `Telemetry.getEvents(side: Side): TraceEvent[]`
  - `Telemetry.pushCommand(side: Side, cmd: object): void`
  - `Telemetry.takeCommandResults(side: Side): object[]`
  - `Telemetry.clear(): void`, `Telemetry.close(): Promise<void>`
  - `type Side = "chrome-orig" | "firefox-conv"` (Plan 3 adds `"firefox-orig"`)
  - `type TraceEvent = { seq: number; ctx: string; api: string; args: unknown[] }`

HTTP surface consumed by the shim: `POST /trace` `{side, events: TraceEvent[]}`; `GET /cmd?side=S` → 200 JSON command or 204; `POST /cmdresult` `{side, result}`. All responses carry permissive CORS headers and OPTIONS preflight support (content-type json from extension pages triggers preflight).

- [ ] **Step 1: Write failing tests**

`e2e/tests/telemetry.test.ts`:
```ts
import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { startTelemetry, type Telemetry } from "../src/telemetry.js";

let t: Telemetry;
const base = "http://127.0.0.1:41999";

beforeAll(async () => { t = await startTelemetry(41999); });
afterAll(async () => { await t.close(); });

describe("telemetry server", () => {
  it("collects posted trace events by side", async () => {
    await fetch(`${base}/trace`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ side: "chrome-orig", events: [{ seq: 0, ctx: "background", api: "storage.local.set", args: [{}] }] }),
    });
    expect(t.getEvents("chrome-orig")).toHaveLength(1);
    expect(t.getEvents("firefox-conv")).toHaveLength(0);
  });
  it("serves queued commands once, then 204", async () => {
    t.pushCommand("chrome-orig", { type: "ping" });
    const r1 = await fetch(`${base}/cmd?side=chrome-orig`);
    expect(r1.status).toBe(200);
    expect(await r1.json()).toEqual({ type: "ping" });
    const r2 = await fetch(`${base}/cmd?side=chrome-orig`);
    expect(r2.status).toBe(204);
  });
  it("collects command results", async () => {
    await fetch(`${base}/cmdresult`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ side: "firefox-conv", result: { type: "ping", ok: true } }),
    });
    expect(t.takeCommandResults("firefox-conv")).toEqual([{ type: "ping", ok: true }]);
  });
  it("answers CORS preflight", async () => {
    const r = await fetch(`${base}/trace`, { method: "OPTIONS" });
    expect(r.status).toBe(204);
    expect(r.headers.get("access-control-allow-origin")).toBe("*");
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `pnpm test`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

`e2e/src/telemetry.ts`:
```ts
import http from "node:http";

export type Side = "chrome-orig" | "firefox-conv" | "firefox-orig";
export type TraceEvent = { seq: number; ctx: string; api: string; args: unknown[] };

export interface Telemetry {
  getEvents(side: Side): TraceEvent[];
  pushCommand(side: Side, cmd: object): void;
  takeCommandResults(side: Side): object[];
  clear(): void;
  close(): Promise<void>;
}

const CORS = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET,POST,OPTIONS",
  "access-control-allow-headers": "content-type",
};

export function startTelemetry(port: number): Promise<Telemetry> {
  const events = new Map<string, TraceEvent[]>();
  const commands = new Map<string, object[]>();
  const results = new Map<string, object[]>();
  const get = <T>(m: Map<string, T[]>, k: string) => m.get(k) ?? (m.set(k, []), m.get(k)!);

  const server = http.createServer((req, res) => {
    const url = new URL(req.url ?? "/", `http://127.0.0.1:${port}`);
    if (req.method === "OPTIONS") { res.writeHead(204, CORS); res.end(); return; }
    if (req.method === "GET" && url.pathname === "/cmd") {
      const q = get(commands, url.searchParams.get("side") ?? "");
      const cmd = q.shift();
      if (!cmd) { res.writeHead(204, CORS); res.end(); return; }
      res.writeHead(200, { ...CORS, "content-type": "application/json" });
      res.end(JSON.stringify(cmd));
      return;
    }
    if (req.method === "POST") {
      let body = "";
      req.on("data", (c) => (body += c));
      req.on("end", () => {
        try {
          const data = JSON.parse(body);
          if (url.pathname === "/trace") get(events, data.side).push(...data.events);
          else if (url.pathname === "/cmdresult") get(results, data.side).push(data.result);
        } catch { /* malformed posts are dropped */ }
        res.writeHead(200, CORS);
        res.end("ok");
      });
      return;
    }
    res.writeHead(404, CORS);
    res.end();
  });

  return new Promise((resolve) =>
    server.listen(port, "127.0.0.1", () =>
      resolve({
        getEvents: (s) => [...get(events, s)],
        pushCommand: (s, c) => { get(commands, s).push(c); },
        takeCommandResults: (s) => get(results, s).splice(0),
        clear: () => { events.clear(); commands.clear(); results.clear(); },
        close: () => new Promise((r) => server.close(() => r())),
      }),
    ),
  );
}
```

- [ ] **Step 4: Run tests**

Run: `pnpm test`
Expected: all passing.

- [ ] **Step 5: Commit**

```bash
git add e2e/src/telemetry.ts e2e/tests/telemetry.test.ts
git commit -m "feat(e2e): telemetry server for shim traces and command channel"
```

---

### Task 7: Spy shim

**Files:**
- Create: `e2e/shim/shim.js` (plain JS, no build step)
- Test: `e2e/tests/shim.test.ts`

**Interfaces:**
- Consumes: telemetry HTTP surface from Task 6
- Produces: `e2e/shim/shim.js` with placeholders `__C2M_SIDE__` (string), `__C2M_PORT__` (number literal), replaced by the injector (Task 8). Exposes nothing; self-initializing IIFE. Internal message marker key: `__c2m__`.

Behavior:
- Wraps every *existing* function-valued property of `chrome.*`/`browser.*` namespaces (recursive walk, depth ≤ 3), recording `{seq, ctx, api, args}` on call, `api + ":resolve"/":reject"` for promise results, `api + ":throw"` for sync throws
- Wraps `addListener` on event objects (property has `addListener`) to record `api + ":fired"` with the event args when the extension's listener runs
- Wraps `globalThis.fetch` recording `net.fetch` with `[method, url]` — but never records telemetry-server URLs (self-filtering)
- Records `error` / `unhandledrejection` global events as `runtime.error`
- Skips recording any message whose first arg is an object with key `__c2m__`; the shim's own ping relay uses that marker
- Background context polls `GET /cmd` every 500ms; supports `{type:"ping"}`: sends `{__c2m__:"ping"}` to the active tab via `tabs.sendMessage`, content shim replies `{__c2m__:"pong"}`, background posts `{type:"ping", ok:true|false}` to `/cmdresult`
- Batches events, flushes every 250ms via `fetch` POST `/trace`
- NEVER adds a property that does not already exist (transparency)

- [ ] **Step 1: Write failing tests (shim logic against a fake `chrome` object in Node)**

`e2e/tests/shim.test.ts`:
```ts
import { describe, it, expect, beforeEach, vi } from "vitest";
import { readFileSync } from "node:fs";
import vm from "node:vm";

function loadShim(fakeChrome: any) {
  const src = readFileSync(new URL("../shim/shim.js", import.meta.url), "utf8")
    .replaceAll("__C2M_SIDE__", "chrome-orig")
    .replaceAll("__C2M_PORT__", "41999");
  const posted: any[] = [];
  const sandbox: any = {
    chrome: fakeChrome,
    fetch: vi.fn(async (url: string, init?: any) => {
      if (init?.body) posted.push(JSON.parse(init.body));
      return { ok: true, status: 204, json: async () => ({}) };
    }),
    setInterval: (fn: () => void) => fn, // capture, don't schedule
    console,
  };
  sandbox.globalThis = sandbox;
  sandbox.self = sandbox;
  vm.createContext(sandbox);
  vm.runInContext(src, sandbox);
  return { sandbox, posted, flush: () => sandbox.__c2m_test_flush__() };
}

describe("shim", () => {
  let calls: any[];
  let fake: any;
  beforeEach(() => {
    calls = [];
    fake = {
      storage: { local: { set: (v: any) => { calls.push(["set", v]); return Promise.resolve(); } } },
      runtime: {
        sendMessage: (m: any) => { calls.push(["send", m]); return Promise.resolve(); },
        onMessage: { addListener: (cb: any) => calls.push(["listen", cb]) },
      },
    };
  });

  it("records wrapped API calls and still calls through", async () => {
    const { flush, posted, sandbox } = loadShim(fake);
    await sandbox.chrome.storage.local.set({ a: 1 });
    flush();
    const apis = posted.flatMap((p: any) => p.events.map((e: any) => e.api));
    expect(apis).toContain("storage.local.set");
    expect(calls).toContainEqual(["set", { a: 1 }]);
  });

  it("does not record shim-internal messages", async () => {
    const { flush, posted, sandbox } = loadShim(fake);
    await sandbox.chrome.runtime.sendMessage({ __c2m__: "ping" });
    flush();
    const apis = posted.flatMap((p: any) => p.events.map((e: any) => e.api));
    expect(apis).not.toContain("runtime.sendMessage");
  });

  it("never adds missing namespaces (transparency)", () => {
    const { sandbox } = loadShim(fake);
    expect(sandbox.chrome.offscreen).toBeUndefined();
    expect(sandbox.chrome.tabGroups).toBeUndefined();
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `pnpm test`
Expected: FAIL — shim.js not found.

- [ ] **Step 3: Implement `e2e/shim/shim.js`**

```js
/* chrome2moz e2e spy shim. Injected first into every extension context.
   Placeholders __C2M_SIDE__ / __C2M_PORT__ are replaced at injection time. */
(() => {
  const g = globalThis;
  if (g.__c2m_shim__) return;
  g.__c2m_shim__ = true;

  const SIDE = "__C2M_SIDE__";
  const PORT = __C2M_PORT__;
  const BASE = "http://127.0.0.1:" + PORT;
  const MARK = "__c2m__";

  const ctx = (() => {
    try {
      if (typeof location === "undefined") return "background";
      const p = location.protocol;
      if (p === "chrome-extension:" || p === "moz-extension:") return "extpage:" + location.pathname;
      if (p === "http:" || p === "https:") return "content";
      return "background";
    } catch { return "background"; }
  })();

  let seq = 0;
  const buf = [];
  const record = (api, args) => { buf.push({ seq: seq++, ctx, api, args }); };

  const MAXSTR = 200;
  const norm = (v, depth = 0) => {
    if (depth > 4) return "[deep]";
    if (v === undefined) return null;
    if (typeof v === "function") return "[fn]";
    if (typeof v === "string") return v.length > MAXSTR ? v.slice(0, MAXSTR) + "…" : v;
    if (v === null || typeof v !== "object") return v;
    if (Array.isArray(v)) return v.slice(0, 20).map((x) => norm(x, depth + 1));
    const o = {};
    for (const k of Object.keys(v).slice(0, 30)) {
      try { o[k] = norm(v[k], depth + 1); } catch { o[k] = "[err]"; }
    }
    return o;
  };
  const normArgs = (args) => Array.from(args, (a) => norm(a));

  const rawFetch = g.fetch ? g.fetch.bind(g) : null;
  const flush = () => {
    if (!buf.length || !rawFetch) return;
    const events = buf.splice(0);
    try {
      rawFetch(BASE + "/trace", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ side: SIDE, events }),
      }).catch(() => {});
    } catch {}
  };
  g.__c2m_test_flush__ = flush;
  const timer = typeof setInterval === "function" ? setInterval(flush, 250) : null;

  const isInternal = (args) =>
    args.length > 0 && args[0] && typeof args[0] === "object" && MARK in args[0];

  const wrapFn = (ns, path, key) => {
    const orig = ns[key];
    const api = path + "." + key;
    const wrapped = function (...args) {
      if (!isInternal(args)) record(api, normArgs(args));
      let r;
      try { r = orig.apply(this === wrapped ? ns : this, args); }
      catch (e) { record(api + ":throw", [String(e)]); throw e; }
      if (r && typeof r.then === "function") {
        r.then((v) => record(api + ":resolve", [norm(v)]), (e) => record(api + ":reject", [String(e)]));
      }
      return r;
    };
    try { ns[key] = wrapped; } catch {}
  };

  const wrapEvent = (ev, path) => {
    const origAdd = ev.addListener;
    if (typeof origAdd !== "function") return;
    try {
      ev.addListener = function (cb, ...rest) {
        const wrappedCb = function (...args) {
          if (!isInternal(args)) record(path + ":fired", normArgs(args));
          return cb.apply(this, args);
        };
        return origAdd.call(ev, wrappedCb, ...rest);
      };
    } catch {}
  };

  const SKIP = new Set(["csi", "loadTimes"]);
  const walk = (ns, path, depth) => {
    if (!ns || typeof ns !== "object" || depth > 3) return;
    for (const key of Object.keys(ns)) {
      if (SKIP.has(key)) continue;
      let v;
      try { v = ns[key]; } catch { continue; }
      const p = path + "." + key;
      if (typeof v === "function") wrapFn(ns, path, key);
      else if (v && typeof v === "object") {
        if (typeof v.addListener === "function") wrapEvent(v, p);
        else walk(v, p, depth + 1);
      }
    }
  };

  const root = typeof browser !== "undefined" ? browser : typeof chrome !== "undefined" ? chrome : null;
  if (root) {
    for (const top of Object.keys(root)) {
      let v;
      try { v = root[top]; } catch { continue; }
      if (v && typeof v === "object") {
        if (typeof v.addListener === "function") wrapEvent(v, top);
        else walk(v, top, 0);
      } else if (typeof v === "function") wrapFn(root, "", top);
    }
    // In Chrome, `browser` may be an alias of `chrome`; wrapping once via the root reference covers both.
  }

  if (rawFetch) {
    g.fetch = function (input, init) {
      const url = typeof input === "string" ? input : input && input.url ? input.url : String(input);
      if (!url.startsWith(BASE)) record("net.fetch", [init && init.method ? init.method : "GET", norm(url)]);
      return rawFetch(input, init);
    };
  }

  try {
    g.addEventListener?.("error", (e) => record("runtime.error", [norm(String(e && e.message))]));
    g.addEventListener?.("unhandledrejection", (e) => record("runtime.error", [norm(String(e && e.reason))]));
  } catch {}

  // Command channel + ping relay (background only)
  if (ctx === "background" && root && rawFetch) {
    if (root.runtime && root.runtime.onMessage) {
      try {
        root.runtime.onMessage.addListener((msg, _s, sendResponse) => {
          if (msg && typeof msg === "object" && msg[MARK] === "ping") { sendResponse({ [MARK]: "pong" }); return true; }
        });
      } catch {}
    }
    const poll = async () => {
      try {
        const res = await rawFetch(BASE + "/cmd?side=" + SIDE);
        if (res.status !== 200) return;
        const cmd = await res.json();
        if (cmd.type === "ping") {
          let ok = false;
          try {
            const tabs = await root.tabs.query({ active: true });
            const reply = await root.tabs.sendMessage(tabs[0].id, { [MARK]: "ping" });
            ok = !!(reply && reply[MARK] === "pong");
          } catch {}
          await rawFetch(BASE + "/cmdresult", {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({ side: SIDE, result: { type: "ping", ok } }),
          });
        }
      } catch {}
    };
    if (typeof setInterval === "function") setInterval(poll, 500);
  } else if (ctx === "content" && root && root.runtime && root.runtime.onMessage) {
    try {
      root.runtime.onMessage.addListener((msg, _s, sendResponse) => {
        if (msg && typeof msg === "object" && msg[MARK] === "ping") { sendResponse({ [MARK]: "pong" }); return true; }
      });
    } catch {}
  }

  void timer;
})();
```

Note: the content-script ping listener registers through the *wrapped* `addListener`, but internal messages are filtered by `isInternal` so nothing is recorded.

- [ ] **Step 4: Run tests**

Run: `pnpm test`
Expected: all passing. If the `this === wrapped` call-through binding breaks a test, bind `orig` to `ns` unconditionally — Chrome API methods don't rely on dynamic `this`.

- [ ] **Step 5: Commit**

```bash
git add e2e/shim/shim.js e2e/tests/shim.test.ts
git commit -m "feat(e2e): transparent spy shim with trace batching and command channel"
```

---

### Task 8: Injector

**Files:**
- Create: `e2e/src/injector.ts`
- Test: `e2e/tests/injector.test.ts`

**Interfaces:**
- Consumes: `e2e/shim/shim.js`
- Produces: `instrumentExtension(dir: string, side: Side, port: number): void` — mutates an unpacked extension dir in place:
  - writes `__c2m_shim.js` (placeholders replaced)
  - background: Chrome shape (`service_worker`) → rewrites to a generated `__c2m_bg.js` containing `importScripts("__c2m_shim.js", "<orig>")` (or, if `"type": "module"`, `import "./__c2m_shim.js"; import "./<orig>";`); Firefox shape (`scripts: []`) → unshifts `"__c2m_shim.js"`
  - every `content_scripts[].js` → unshifts `"__c2m_shim.js"`
  - every HTML file referenced by `action.default_popup`, `options_ui.page`, `options_page`, `sidebar_action.default_panel` → `<script src="__c2m_shim.js"></script>` inserted immediately after the opening `<head>` tag (or prepended to the file if no `<head>`)
  - adds `"http://127.0.0.1/*"` to `host_permissions` (creates the array if absent — both sides get it, so symmetric)
  - also produces `zipDir(dir: string, outFile: string): void` (adm-zip wrapper) for building the Firefox `.xpi`

- [ ] **Step 1: Write failing tests**

`e2e/tests/injector.test.ts`:
```ts
import { describe, it, expect, beforeEach } from "vitest";
import { mkdtempSync, writeFileSync, readFileSync, mkdirSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { instrumentExtension } from "../src/injector.js";

function makeExt(manifest: object, files: Record<string, string> = {}): string {
  const dir = mkdtempSync(join(tmpdir(), "c2m-inj-"));
  writeFileSync(join(dir, "manifest.json"), JSON.stringify(manifest));
  for (const [name, content] of Object.entries(files)) writeFileSync(join(dir, name), content);
  return dir;
}
const readManifest = (dir: string) => JSON.parse(readFileSync(join(dir, "manifest.json"), "utf8"));

describe("instrumentExtension", () => {
  it("rewrites a Chrome service worker background through a wrapper", () => {
    const dir = makeExt(
      { manifest_version: 3, background: { service_worker: "bg.js" } },
      { "bg.js": "// bg" },
    );
    instrumentExtension(dir, "chrome-orig", 41999);
    const m = readManifest(dir);
    expect(m.background.service_worker).toBe("__c2m_bg.js");
    const wrapper = readFileSync(join(dir, "__c2m_bg.js"), "utf8");
    expect(wrapper).toContain('importScripts("__c2m_shim.js", "bg.js")');
    expect(existsSync(join(dir, "__c2m_shim.js"))).toBe(true);
  });

  it("unshifts the shim into a Firefox scripts background and content scripts", () => {
    const dir = makeExt(
      {
        manifest_version: 3,
        background: { scripts: ["bg.js"] },
        content_scripts: [{ matches: ["<all_urls>"], js: ["cs.js"] }],
      },
      { "bg.js": "", "cs.js": "" },
    );
    instrumentExtension(dir, "firefox-conv", 41999);
    const m = readManifest(dir);
    expect(m.background.scripts).toEqual(["__c2m_shim.js", "bg.js"]);
    expect(m.content_scripts[0].js).toEqual(["__c2m_shim.js", "cs.js"]);
  });

  it("injects a script tag into the popup and adds the telemetry host permission", () => {
    const dir = makeExt(
      { manifest_version: 3, action: { default_popup: "popup.html" } },
      { "popup.html": "<html><head><title>x</title></head><body></body></html>" },
    );
    instrumentExtension(dir, "chrome-orig", 41999);
    const html = readFileSync(join(dir, "popup.html"), "utf8");
    expect(html).toContain('<head><script src="__c2m_shim.js"></script>');
    expect(readManifest(dir).host_permissions).toContain("http://127.0.0.1/*");
  });

  it("replaces shim placeholders with side and port", () => {
    const dir = makeExt({ manifest_version: 3 });
    instrumentExtension(dir, "firefox-conv", 42123);
    const shim = readFileSync(join(dir, "__c2m_shim.js"), "utf8");
    expect(shim).toContain('"firefox-conv"');
    expect(shim).toContain("42123");
    expect(shim).not.toContain("__C2M_SIDE__");
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `pnpm test`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `e2e/src/injector.ts`**

```ts
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import AdmZip from "adm-zip";
import type { Side } from "./telemetry.js";

const SHIM_SRC = join(dirname(fileURLToPath(import.meta.url)), "..", "shim", "shim.js");
const SHIM_NAME = "__c2m_shim.js";

export function instrumentExtension(dir: string, side: Side, port: number): void {
  const shim = readFileSync(SHIM_SRC, "utf8")
    .replaceAll("__C2M_SIDE__", side)
    .replaceAll("__C2M_PORT__", String(port));
  writeFileSync(join(dir, SHIM_NAME), shim);

  const manifestPath = join(dir, "manifest.json");
  const m = JSON.parse(readFileSync(manifestPath, "utf8"));

  if (m.background?.service_worker) {
    const orig = m.background.service_worker;
    const isModule = m.background.type === "module";
    const wrapper = isModule
      ? `import "./${SHIM_NAME}";\nimport "./${orig}";\n`
      : `importScripts("${SHIM_NAME}", "${orig}");\n`;
    writeFileSync(join(dir, "__c2m_bg.js"), wrapper);
    m.background.service_worker = "__c2m_bg.js";
  } else if (Array.isArray(m.background?.scripts)) {
    m.background.scripts.unshift(SHIM_NAME);
  }

  for (const cs of m.content_scripts ?? []) {
    if (Array.isArray(cs.js)) cs.js.unshift(SHIM_NAME);
  }

  const htmlEntries = [
    m.action?.default_popup,
    m.browser_action?.default_popup,
    m.options_ui?.page,
    m.options_page,
    m.sidebar_action?.default_panel,
  ].filter((p): p is string => typeof p === "string");
  for (const rel of htmlEntries) {
    const p = join(dir, rel.split("?")[0]);
    if (!existsSync(p)) continue;
    let html = readFileSync(p, "utf8");
    const tag = `<script src="${SHIM_NAME}"></script>`;
    if (html.includes(tag)) continue;
    if (/<head[^>]*>/i.test(html)) html = html.replace(/<head[^>]*>/i, (h) => h + tag);
    else html = tag + html;
    writeFileSync(p, html);
  }

  m.host_permissions = Array.from(new Set([...(m.host_permissions ?? []), "http://127.0.0.1/*"]));
  writeFileSync(manifestPath, JSON.stringify(m, null, 2));
}

export function zipDir(dir: string, outFile: string): void {
  const zip = new AdmZip();
  zip.addLocalFolder(dir);
  zip.writeZip(outFile);
}
```

- [ ] **Step 4: Run tests**

Run: `pnpm test`
Expected: all passing.

- [ ] **Step 5: Commit**

```bash
git add e2e/src/injector.ts e2e/tests/injector.test.ts
git commit -m "feat(e2e): shim injector for both extension builds"
```

---

### Task 9: Browser drivers

**Files:**
- Create: `e2e/src/chromeDriver.ts`, `e2e/src/firefoxDriver.ts`
- Test: `e2e/tests/drivers.integration.test.ts` (tagged integration — excluded from plain `pnpm test`, run explicitly)

**Interfaces:**
- Consumes: instrumented extension dirs (Task 8), telemetry (Task 6), hello extension (Task 1)
- Produces:
  - `launchChrome(extDir: string): Promise<ChromeSession>` where `ChromeSession = { extensionId: string; open(url: string): Promise<void>; pressChord(chord: string): Promise<void>; openExtensionPage(relPath: string): Promise<void>; screenshot(outPath: string): Promise<void>; close(): Promise<void> }`
  - `launchFirefox(xpiPath: string, geckoId: string): Promise<FirefoxSession>` with the same session interface (`extensionId` = internal UUID)
  - Chord format: manifest style, e.g. `"Ctrl+Shift+9"`; drivers map to their input APIs (`Ctrl` → `Control`, `MacCtrl` → `Control`, `Command` → `Meta`)

- [ ] **Step 1: Implement `e2e/src/chromeDriver.ts`**

```ts
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
      headless: false,
      args: [`--disable-extensions-except=${extDir}`, `--load-extension=${extDir}`, "--no-first-run"],
    },
  );
  const sw = ctx.serviceWorkers()[0] ?? (await ctx.waitForEvent("serviceworker", { timeout: 15000 }));
  const extensionId = new URL(sw.url()).host;
  let page: Page = ctx.pages()[0] ?? (await ctx.newPage());

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
```

- [ ] **Step 2: Implement `e2e/src/firefoxDriver.ts`**

```ts
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
```

- [ ] **Step 3: Write the integration test**

`e2e/tests/drivers.integration.test.ts` (guarded so `pnpm test` skips it unless `E2E_INTEGRATION=1`):
```ts
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
```

- [ ] **Step 4: Run integration tests**

Run: `E2E_INTEGRATION=1 pnpm exec vitest run tests/drivers.integration.test.ts`
Expected: both pass, real browsers open and close. Debug with spike learnings if traces are empty (most likely causes: CORS preflight, shim not first in load order, MV3 background shape in Firefox).

- [ ] **Step 5: Commit**

```bash
git add e2e/src/chromeDriver.ts e2e/src/firefoxDriver.ts e2e/tests/drivers.integration.test.ts
git commit -m "feat(e2e): chromium and firefox session drivers with shim integration test"
```

---

### Task 10: Fixture pages + static server

**Files:**
- Create: `e2e/src/fixtureServer.ts`, `e2e/fixtures/basic.html`, `e2e/fixtures/form.html`

**Interfaces:**
- Produces: `startFixtures(port: number): Promise<{ url(name: string): string; close(): Promise<void> }>` — serves `e2e/fixtures/*` at `http://127.0.0.1:<port>/<name>`

- [ ] **Step 1: Create fixture pages**

`e2e/fixtures/basic.html`:
```html
<!DOCTYPE html>
<html><head><title>C2M basic fixture</title></head>
<body>
  <h1 id="title">Basic fixture</h1>
  <p contenteditable="true" id="editable">editable text</p>
  <textarea id="ta">some latex: \frac{1}{2}</textarea>
</body></html>
```

`e2e/fixtures/form.html`:
```html
<!DOCTYPE html>
<html><head><title>C2M form fixture</title></head>
<body>
  <form id="f"><input id="text" type="text" value="hello"><input id="pw" type="password">
  <select id="sel"><option>a</option><option>b</option></select>
  <button id="submit" type="submit">Submit</button></form>
</body></html>
```

- [ ] **Step 2: Implement `e2e/src/fixtureServer.ts`**

```ts
import http from "node:http";
import { readFileSync, existsSync } from "node:fs";
import { join, dirname, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const FIXTURES = join(dirname(fileURLToPath(import.meta.url)), "..", "fixtures");

export function startFixtures(port: number): Promise<{ url(name: string): string; close(): Promise<void> }> {
  const server = http.createServer((req, res) => {
    const name = normalize(req.url ?? "/").replace(/^([/\\.])+/, "");
    const p = join(FIXTURES, name || "basic.html");
    if (!p.startsWith(FIXTURES) || !existsSync(p)) { res.writeHead(404); res.end(); return; }
    res.setHeader("content-type", "text/html");
    res.end(readFileSync(p));
  });
  return new Promise((resolve) =>
    server.listen(port, "127.0.0.1", () =>
      resolve({
        url: (name) => `http://127.0.0.1:${port}/${name}`,
        close: () => new Promise((r) => server.close(() => r())),
      }),
    ),
  );
}
```

- [ ] **Step 3: Quick check**

Run: `pnpm exec tsx -e "import('./src/fixtureServer.js').then(async m => { const f = await m.startFixtures(41990); const r = await fetch(f.url('basic.html')); console.log(r.status); await f.close(); })"`
Expected: `200`.

- [ ] **Step 4: Commit**

```bash
git add e2e/src/fixtureServer.ts e2e/fixtures/
git commit -m "feat(e2e): fixture pages and static server"
```

---

### Task 11: Trace normalizer + diff engine

**Files:**
- Create: `e2e/src/diff.ts`
- Test: `e2e/tests/diff.test.ts`

**Interfaces:**
- Consumes: `TraceEvent` from Task 6
- Produces:
  - `normalizeTrace(events: TraceEvent[]): NormalizedEvent[]` where `NormalizedEvent = { ctx: string; api: string; args: string }` (args JSON-stringified after normalization)
  - `diffTraces(a: NormalizedEvent[], b: NormalizedEvent[], allowedDiffs: string[]): Divergence[]` where `Divergence = { side: "a" | "b"; event: NormalizedEvent; allowed: boolean }`
  - Normalization rules: numeric values under keys matching `/^(tabId|windowId|frameId|requestId|id)$/` → sequential placeholders (`"<id:1>"`) via first-seen mapping; ISO datetimes and 10/13-digit epoch numbers in strings → `"<time>"`; `chrome-extension://<id>` and `moz-extension://<uuid>` URL prefixes → `"<ext>"`; `chrome.` / `browser.` prefix differences already absent (shim records relative api paths)
  - Diff: LCS over `ctx + api + args` string keys per context group; events present on one side only become divergences; `allowed` = any picomatch pattern in `allowedDiffs` matches `event.api`

- [ ] **Step 1: Write failing tests**

`e2e/tests/diff.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { normalizeTrace, diffTraces } from "../src/diff.js";

const ev = (api: string, args: unknown[] = [], ctx = "background") =>
  ({ seq: 0, ctx, api, args });

describe("normalizeTrace", () => {
  it("maps ids to stable placeholders by first appearance", () => {
    const [a, b] = normalizeTrace([ev("tabs.sendMessage", [{ tabId: 731 }]), ev("tabs.sendMessage", [{ tabId: 731 }])]);
    expect(a.args).toContain("<id:1>");
    expect(a.args).toBe(b.args);
  });
  it("scrubs extension-origin urls and timestamps", () => {
    const [n] = normalizeTrace([ev("net.fetch", ["GET", "chrome-extension://abcdefgh/popup.html?t=1753791234567"])]);
    expect(n.args).not.toContain("abcdefgh");
    expect(n.args).not.toContain("1753791234567");
  });
});

describe("diffTraces", () => {
  it("returns empty for identical traces", () => {
    const a = normalizeTrace([ev("storage.local.set", [{ a: 1 }])]);
    expect(diffTraces(a, a, [])).toEqual([]);
  });
  it("flags one-sided events with the side that has them", () => {
    const a = normalizeTrace([ev("storage.local.set", [{}]), ev("management.uninstallSelf")]);
    const b = normalizeTrace([ev("storage.local.set", [{}])]);
    const d = diffTraces(a, b, []);
    expect(d).toHaveLength(1);
    expect(d[0].side).toBe("a");
    expect(d[0].event.api).toBe("management.uninstallSelf");
    expect(d[0].allowed).toBe(false);
  });
  it("marks divergences matching allowed_diffs globs", () => {
    const a = normalizeTrace([ev("tabGroups.query", [{}])]);
    const d = diffTraces(a, [], ["tabGroups.*"]);
    expect(d[0].allowed).toBe(true);
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `pnpm test`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `e2e/src/diff.ts`**

```ts
import picomatch from "picomatch";
import type { TraceEvent } from "./telemetry.js";

export type NormalizedEvent = { ctx: string; api: string; args: string };
export type Divergence = { side: "a" | "b"; event: NormalizedEvent; allowed: boolean };

const ID_KEY = /^(tabId|windowId|frameId|requestId|id)$/;
const EXT_URL = /(chrome|moz)-extension:\/\/[a-z0-9-]+/gi;
const EPOCH = /\b1[0-9]{9}(?:[0-9]{3})?\b/g;
const ISO = /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[^"\s]*/g;

export function normalizeTrace(events: TraceEvent[]): NormalizedEvent[] {
  const idMap = new Map<number, string>();
  const mapId = (n: number) => {
    if (!idMap.has(n)) idMap.set(n, `<id:${idMap.size + 1}>`);
    return idMap.get(n)!;
  };
  const walk = (v: unknown): unknown => {
    if (Array.isArray(v)) return v.map(walk);
    if (v && typeof v === "object") {
      const o: Record<string, unknown> = {};
      for (const [k, val] of Object.entries(v)) {
        o[k] = ID_KEY.test(k) && typeof val === "number" ? mapId(val) : walk(val);
      }
      return o;
    }
    if (typeof v === "string") return v.replace(EXT_URL, "<ext>").replace(ISO, "<time>").replace(EPOCH, "<time>");
    return v;
  };
  return events.map((e) => ({
    ctx: e.ctx,
    api: e.api,
    args: JSON.stringify(walk(e.args)),
  }));
}

function lcsKeep(a: string[], b: string[]): boolean[][] {
  const n = a.length, m = b.length;
  const dp: number[][] = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0));
  for (let i = n - 1; i >= 0; i--)
    for (let j = m - 1; j >= 0; j--)
      dp[i][j] = a[i] === b[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
  const inA = new Array(n).fill(false), inB = new Array(m).fill(false);
  let i = 0, j = 0;
  while (i < n && j < m) {
    if (a[i] === b[j]) { inA[i] = true; inB[j] = true; i++; j++; }
    else if (dp[i + 1][j] >= dp[i][j + 1]) i++;
    else j++;
  }
  return [inA, inB];
}

export function diffTraces(a: NormalizedEvent[], b: NormalizedEvent[], allowedDiffs: string[]): Divergence[] {
  const isAllowed = allowedDiffs.length ? picomatch(allowedDiffs) : () => false;
  const out: Divergence[] = [];
  const ctxs = new Set([...a, ...b].map((e) => e.ctx));
  for (const ctx of ctxs) {
    const ea = a.filter((e) => e.ctx === ctx);
    const eb = b.filter((e) => e.ctx === ctx);
    const key = (e: NormalizedEvent) => `${e.api} ${e.args}`;
    const [inA, inB] = lcsKeep(ea.map(key), eb.map(key));
    ea.forEach((e, i) => { if (!inA[i]) out.push({ side: "a", event: e, allowed: isAllowed(e.api) }); });
    eb.forEach((e, i) => { if (!inB[i]) out.push({ side: "b", event: e, allowed: isAllowed(e.api) }); });
  }
  return out;
}
```

- [ ] **Step 4: Run tests**

Run: `pnpm test`
Expected: all passing.

- [ ] **Step 5: Commit**

```bash
git add e2e/src/diff.ts e2e/tests/diff.test.ts
git commit -m "feat(e2e): trace normalizer and LCS diff engine with allowed_diffs globs"
```

---

### Task 12: Corpus, probes, and runner

**Files:**
- Create: `e2e/corpus.json`, `e2e/src/corpus.ts`, `e2e/src/probes.ts`, `e2e/src/convert.ts`, `e2e/src/run.ts`

**Interfaces:**
- Consumes: everything above
- Produces: `pnpm e2e [--only <id>]` → runs the corpus, writes `e2e/results/<id>/report.json` + screenshots, prints a summary table, exits non-zero if any non-quarantined extension has an unallowed divergence that reproduces on retry

- [ ] **Step 1: Create `e2e/corpus.json`**

```json
{
  "extensions": [
    {
      "id": "latextocalc",
      "name": "LatexToCalc",
      "source": "local:../LatexToCalc",
      "allowed_diffs": [],
      "quarantined": false
    },
    {
      "id": "gojbdfnpnhogfdgjbigejoaolejmgdhk",
      "name": "OneNote Web Clipper",
      "source": "cws",
      "version": "PIN_FROM_TASK_5_STEP_5",
      "allowed_diffs": ["management.uninstallSelf*"],
      "quarantined": false
    }
  ]
}
```
Replace `PIN_FROM_TASK_5_STEP_5` with the version recorded in Task 5 Step 5. If `../LatexToCalc` (repo-root submodule) has its `manifest.json` in a subdirectory, point `source` at that subdirectory — check with `find LatexToCalc -maxdepth 2 -name manifest.json` from the repo root.

- [ ] **Step 2: Implement `e2e/src/corpus.ts`**

```ts
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

export type CorpusEntry = {
  id: string;
  name: string;
  source: string; // "cws" | "local:<path relative to e2e/>"
  version?: string;
  allowed_diffs: string[];
  quarantined: boolean;
};

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");

export function loadCorpus(): CorpusEntry[] {
  return JSON.parse(readFileSync(join(ROOT, "corpus.json"), "utf8")).extensions;
}
export const e2eRoot = ROOT;
```

- [ ] **Step 3: Implement `e2e/src/convert.ts`** (subprocess wrapper for the Rust CLI)

```ts
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import AdmZip from "adm-zip";
import { e2eRoot } from "./corpus.js";

const REPO_ROOT = join(e2eRoot, "..");

export function buildConverter(): void {
  execFileSync("cargo", ["build", "--release", "--features", "cli"], { cwd: REPO_ROOT, stdio: "inherit" });
}

/** Converts an unpacked Chrome extension dir; returns dir of the unpacked Firefox build. */
export function convert(inputDir: string, outDir: string): string {
  mkdirSync(outDir, { recursive: true });
  execFileSync(join(REPO_ROOT, "target", "release", "chrome2moz"), ["convert", "-i", inputDir, "-o", outDir, "--yes"], {
    stdio: "inherit",
  });
  if (existsSync(join(outDir, "manifest.json"))) return outDir;
  const xpi = readdirSync(outDir).find((f) => f.endsWith(".xpi") || f.endsWith(".zip"));
  if (!xpi) throw new Error(`converter produced neither manifest.json nor an .xpi in ${outDir}`);
  const unpacked = join(outDir, "unpacked");
  new AdmZip(join(outDir, xpi)).extractAllTo(unpacked, true);
  return unpacked;
}
```
Verify the actual CLI flags with `./target/release/chrome2moz convert --help` while implementing; adjust the args array if the interface differs (e.g. no `--yes`).

- [ ] **Step 4: Implement `e2e/src/probes.ts`**

```ts
import type { BrowserSession } from "./chromeDriver.js";
import type { Telemetry, Side } from "./telemetry.js";

export type ProbeContext = {
  chrome: BrowserSession;
  firefox: BrowserSession;
  telemetry: Telemetry;
  manifest: Record<string, any>;
  fixtureUrl: (name: string) => string;
  resultsDir: string;
};
export type ProbeResult = { name: string; status: "ran" | "skipped"; note?: string };

const settle = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function installProbe(_p: ProbeContext): Promise<ProbeResult> {
  await settle(4000); // background boot + onInstalled traces flow in
  return { name: "install", status: "ran" };
}

export async function contentProbe(p: ProbeContext): Promise<ProbeResult> {
  const hasContent = Array.isArray(p.manifest.content_scripts) && p.manifest.content_scripts.length > 0;
  if (!hasContent) return { name: "content", status: "skipped", note: "no content_scripts" };
  for (const fixture of ["basic.html", "form.html"]) {
    const url = p.fixtureUrl(fixture);
    await p.chrome.open(url);
    await p.firefox.open(url);
    await settle(2500);
  }
  return { name: "content", status: "ran" };
}

export async function commandsProbe(p: ProbeContext): Promise<ProbeResult> {
  const commands = Object.entries(p.manifest.commands ?? {}) as [string, any][];
  if (!commands.length) return { name: "commands", status: "skipped", note: "no commands" };
  if (process.env.C2M_COMMANDS_UNSUPPORTED) return { name: "commands", status: "skipped", note: "dispatch-unsupported (spike C)" };
  const url = p.fixtureUrl("basic.html");
  await p.chrome.open(url);
  await p.firefox.open(url);
  for (const [, def] of commands) {
    const chord = def?.suggested_key?.default;
    if (!chord) continue;
    await p.chrome.pressChord(chord);
    await p.firefox.pressChord(chord);
    await settle(1000);
  }
  return { name: "commands", status: "ran" };
}

export async function popupProbe(p: ProbeContext): Promise<ProbeResult> {
  const popup = p.manifest.action?.default_popup ?? p.manifest.browser_action?.default_popup;
  if (!popup) return { name: "popup", status: "skipped", note: "no popup" };
  await p.chrome.openExtensionPage(popup);
  await p.firefox.openExtensionPage(popup);
  await settle(2500);
  await p.chrome.screenshot(`${p.resultsDir}/popup-chrome.png`);
  await p.firefox.screenshot(`${p.resultsDir}/popup-firefox.png`);
  return { name: "popup", status: "ran" };
}

export async function pingProbe(p: ProbeContext): Promise<ProbeResult> {
  const url = p.fixtureUrl("basic.html");
  await p.chrome.open(url);
  await p.firefox.open(url);
  await settle(500);
  p.telemetry.pushCommand("chrome-orig", { type: "ping" });
  p.telemetry.pushCommand("firefox-conv", { type: "ping" });
  await settle(3000);
  const cr = p.telemetry.takeCommandResults("chrome-orig");
  const fr = p.telemetry.takeCommandResults("firefox-conv");
  return { name: "ping", status: "ran", note: `chrome:${JSON.stringify(cr)} firefox:${JSON.stringify(fr)}` };
}

export const ALL_PROBES = [installProbe, contentProbe, commandsProbe, popupProbe, pingProbe];
```

- [ ] **Step 5: Implement `e2e/src/run.ts`**

```ts
import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import AdmZip from "adm-zip";
import { loadCorpus, e2eRoot, type CorpusEntry } from "./corpus.js";
import { fetchExtension } from "./fetchCrx.js";
import { buildConverter, convert } from "./convert.js";
import { instrumentExtension, zipDir } from "./injector.js";
import { startTelemetry } from "./telemetry.js";
import { startFixtures } from "./fixtureServer.js";
import { launchChrome } from "./chromeDriver.js";
import { launchFirefox } from "./firefoxDriver.js";
import { normalizeTrace, diffTraces } from "./diff.js";
import { ALL_PROBES, type ProbeResult } from "./probes.js";

const TELEMETRY_PORT = 41999;
const FIXTURE_PORT = 41990;

type ExtReport = {
  id: string; name: string; quarantined: boolean;
  probes: ProbeResult[];
  divergences: { side: string; api: string; args: string; ctx: string; allowed: boolean }[];
  pass: boolean;
};

async function prepareSource(entry: CorpusEntry, work: string): Promise<string> {
  const src = join(work, "source");
  if (entry.source.startsWith("local:")) {
    cpSync(join(e2eRoot, entry.source.slice(6)), src, { recursive: true });
  } else {
    const { zipPath } = await fetchExtension(entry.id, entry.version ?? null, join(e2eRoot, ".cache", "crx"));
    new AdmZip(zipPath).extractAllTo(src, true);
  }
  if (!existsSync(join(src, "manifest.json"))) throw new Error(`${entry.id}: no manifest.json at source root`);
  return src;
}

async function runOne(entry: CorpusEntry): Promise<ExtReport> {
  const work = join(e2eRoot, "results", entry.id);
  rmSync(work, { recursive: true, force: true });
  mkdirSync(work, { recursive: true });

  const source = await prepareSource(entry, work);
  const manifest = JSON.parse(readFileSync(join(source, "manifest.json"), "utf8"));

  const convertedDir = convert(source, join(work, "converted"));
  const convManifest = JSON.parse(readFileSync(join(convertedDir, "manifest.json"), "utf8"));
  const geckoId: string = convManifest.browser_specific_settings?.gecko?.id;
  if (!geckoId) throw new Error(`${entry.id}: converted build has no gecko id`);

  const chromeDir = join(work, "chrome-instrumented");
  cpSync(source, chromeDir, { recursive: true });
  instrumentExtension(chromeDir, "chrome-orig", TELEMETRY_PORT);
  instrumentExtension(convertedDir, "firefox-conv", TELEMETRY_PORT);
  const xpi = join(work, "converted.xpi");
  zipDir(convertedDir, xpi);

  const telemetry = await startTelemetry(TELEMETRY_PORT);
  const fixtures = await startFixtures(FIXTURE_PORT);
  const probes: ProbeResult[] = [];
  let chrome, firefox;
  try {
    chrome = await launchChrome(chromeDir);
    firefox = await launchFirefox(xpi, geckoId);
    for (const probe of ALL_PROBES) {
      probes.push(await probe({ chrome, firefox, telemetry, manifest, fixtureUrl: fixtures.url, resultsDir: work }));
    }
  } finally {
    await chrome?.close().catch(() => {});
    await firefox?.close().catch(() => {});
    await fixtures.close();
  }

  const a = normalizeTrace(telemetry.getEvents("chrome-orig"));
  const b = normalizeTrace(telemetry.getEvents("firefox-conv"));
  await telemetry.close();
  const divergences = diffTraces(a, b, entry.allowed_diffs).map((d) => ({
    side: d.side === "a" ? "chrome-only" : "firefox-only",
    api: d.event.api, args: d.event.args, ctx: d.event.ctx, allowed: d.allowed,
  }));

  const report: ExtReport = {
    id: entry.id, name: entry.name, quarantined: entry.quarantined, probes, divergences,
    pass: divergences.every((d) => d.allowed),
  };
  writeFileSync(join(work, "report.json"), JSON.stringify(report, null, 2));
  writeFileSync(join(work, "trace-chrome.json"), JSON.stringify(a, null, 2));
  writeFileSync(join(work, "trace-firefox.json"), JSON.stringify(b, null, 2));
  return report;
}

const only = process.argv.includes("--only") ? process.argv[process.argv.indexOf("--only") + 1] : null;
buildConverter();
const entries = loadCorpus().filter((e) => !only || e.id === only);
let failed = false;
for (const entry of entries) {
  let report = await runOne(entry);
  if (!report.pass && !entry.quarantined) {
    console.log(`↻ ${entry.name}: divergence found, retrying once to confirm...`);
    report = await runOne(entry); // flake control: must reproduce
  }
  const mark = report.pass ? "PASS" : entry.quarantined ? "FAIL (quarantined)" : "FAIL";
  console.log(`${mark}  ${entry.name}  probes: ${report.probes.map((p) => `${p.name}:${p.status}`).join(" ")}  unallowed divergences: ${report.divergences.filter((d) => !d.allowed).length}`);
  for (const d of report.divergences.filter((x) => !x.allowed).slice(0, 20)) {
    console.log(`   ${d.side} [${d.ctx}] ${d.api} ${d.args.slice(0, 120)}`);
  }
  if (!report.pass && !entry.quarantined) failed = true;
}
process.exit(failed ? 1 : 0);
```

- [ ] **Step 6: First real run — LatexToCalc only**

Run: `pnpm e2e --only latextocalc`
Expected on first attempt: probably FAIL with real divergences. Triage each one:
- Shim/harness artifacts (asymmetric injection, telemetry noise) → fix the harness
- Legitimate converter-behavior differences that are by design (e.g. shimmed no-op APIs) → add precise `allowed_diffs` patterns with a comment-style `"_note"` key in corpus.json explaining each
- Actual converter bugs → file a GitHub issue per bug, quarantine only if it can't be fixed now
Iterate until the run is PASS (or quarantined with filed issues).

- [ ] **Step 7: Second real run — OneNote Web Clipper**

Run: `pnpm e2e --only gojbdfnpnhogfdgjbigejoaolejmgdhk`
Expected: the Firefox-detection branch produces divergences beyond `management.uninstallSelf*` (different code path after detection). Tune `allowed_diffs` for the *detection-branch* consequences only — a diff on `management.uninstallSelf` being *absent* in Firefox-converted (because the converter disabled it) is the regression signal this entry exists for; keep patterns tight so a converter regression (uninstall actually firing → error trace) still fails.

- [ ] **Step 8: Full run**

Run: `pnpm e2e`
Expected: `PASS` for both entries, exit 0.

- [ ] **Step 9: Commit**

```bash
git add e2e/corpus.json e2e/src/ e2e/results/.gitkeep 2>/dev/null || true
git add e2e/
git commit -m "feat(e2e): corpus, probes, and differential runner"
```

---

### Task 13: CI workflow

**Files:**
- Create: `.github/workflows/e2e.yml`
- Modify: `README.md` (add e2e section under Contributing)

- [ ] **Step 1: Create `.github/workflows/e2e.yml`**

```yaml
name: E2E Differential Tests

on:
  pull_request:
  push:
    branches: [main]

jobs:
  e2e:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: true

      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable

      - uses: pnpm/action-setup@v4
        with:
          version: 10

      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
          cache-dependency-path: e2e/pnpm-lock.yaml

      - uses: browser-actions/setup-firefox@v1
        with:
          firefox-version: latest

      - name: Install harness deps
        working-directory: e2e
        run: |
          pnpm install --frozen-lockfile
          pnpm exec playwright install --with-deps chromium

      - name: Cache CRX archives
        uses: actions/cache@v4
        with:
          path: e2e/.cache/crx
          key: crx-${{ hashFiles('e2e/corpus.json') }}

      - name: Unit tests
        working-directory: e2e
        run: pnpm test

      - name: Differential e2e
        working-directory: e2e
        run: xvfb-run --auto-servernum pnpm e2e

      - name: Upload results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: e2e-results
          path: e2e/results/
```

- [ ] **Step 2: Add README section**

Append under `## Contributing` in `README.md`:
```markdown
### E2E differential tests

`e2e/` runs every corpus extension through the converter, loads the original in
Chromium and the converted build in Firefox, and diffs their API-call traces.
CI fails on unallowed divergence.

```bash
cd e2e && pnpm install && pnpm e2e            # full corpus
pnpm e2e --only latextocalc                    # one extension
```

Corpus lives in `e2e/corpus.json`. See
`docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md`.
```

- [ ] **Step 3: Verify workflow locally as far as possible**

Run: `cd e2e && pnpm test && pnpm e2e`
Expected: green. Then push the branch and confirm the workflow runs on the PR; iterate on CI-only issues (xvfb, snap-free Firefox from setup-firefox, geckodriver via Selenium Manager). Known CI risk: ubuntu-latest's snap Firefox breaks geckodriver profile access — `browser-actions/setup-firefox` avoids this; if geckodriver still can't find the binary, set `MOZ_FIREFOX_BINARY`/`webdriver.firefox.bin` via `firefox.Options().setBinary(process.env.FIREFOX_BIN!)` guarded by an env check.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/e2e.yml README.md
git commit -m "ci: run differential e2e suite on every PR"
```

---

### Task 14: Wrap-up — spec sync + follow-up issues

**Files:**
- Modify: `docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md`

- [ ] **Step 1: Sync spec with reality**

Update the spec with any deviations discovered during implementation (commands-probe verdict from Spike C, converter CLI flag differences, prefs.js parsing details). Keep it accurate as the reference doc.

- [ ] **Step 2: File follow-up issues**

Create GitHub issues (via `gh issue create`) for: Plan 2 (snapshots + domain discovery + mitmproxy replay), Plan 3 (three-way baseline + README table/badge + V8 coverage + clipboard readback), plus any converter bugs found in Task 12.

- [ ] **Step 3: Final verification**

Run: `cd e2e && pnpm test && pnpm e2e` and `cargo test` from repo root.
Expected: all green.

- [ ] **Step 4: Commit**

```bash
git add docs/
git commit -m "docs: sync e2e spec with implemented pipeline"
```
