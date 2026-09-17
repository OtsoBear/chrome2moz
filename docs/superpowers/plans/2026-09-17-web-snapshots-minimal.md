# Web Snapshot Corpus (Plan 2, Minimal Cut) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let real corpus extensions get content-script coverage on the domains they actually target, by serving a per-extension HTML snapshot under the real hostname through mitmproxy (offline, deterministic, CI-safe), with both browsers proxied and TLS accepted, and domain discovery from the manifest.

**Architecture:** A mitmproxy serve-addon returns a stored HTML capture for each discovered host (no live network at run time; this is the deterministic minimum). Domain discovery unions `content_scripts.matches` + `host_permissions` + `extra_domains`, caps at 20, and drops wildcard-only patterns. Both drivers gain an optional proxy (Playwright `proxy` + `ignoreHTTPSErrors`; Firefox proxy prefs + `acceptInsecureCerts`, so mitmproxy's CA needs no NSS import). `run.ts` starts mitmdump for entries that have a snapshot, proxies both browsers, and navigates the snapshotted URLs so existing probes (content/ping) fire. Live flow recording is an optional enhancement layered on the same addon, not required for the deliverable.

**Tech Stack:** mitmproxy (uv-managed), TypeScript drivers (Playwright, Selenium), vitest, the existing e2e harness. Python addon for mitmproxy.

**Spec:** `docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md` § "Web snapshots (differential fixtures)" and § Spikes item 3; GitHub issue #3.

## Global Constraints

- **Headless always.** Chromium `--headless=new`, Firefox `-headless`. Run `grep -n headless e2e/src/chromeDriver.ts e2e/src/firefoxDriver.ts` before any corpus run; never headed. (Prior session: ~200 popups.)
- **pnpm only** in `e2e/`; **uv only** for Python and mitmproxy (`uv tool install mitmproxy`; never pip).
- **No em dashes** anywhere.
- **CI is replay/serve-only:** no live network at corpus-run time. The serve-addon reads stored files; live recording (if implemented) is a separate manual `pnpm snapshot` step never run in CI.
- Firefox/geckodriver via Selenium Manager; Playwright Chromium cached. Build converter with `cargo build --release` when running the corpus.
- Work only in worktree `/Users/otsov/.superset/worktrees/1b400713-93e8-4131-93b2-695e1e5e7562/web-snapshots` on branch `feature/e2e-web-snapshots`. Never cd into sibling worktrees or the main checkout.
- **Risk + fallback:** this is the riskiest item. If the proxy + serve path is not working end to end within roughly 60 minutes of Task 4, ship what works (domain discovery + the serve-addon + drivers proxy option, with the OneNote snapshot injecting on at least one browser) and state precisely in the PR what is real vs. pending. Never claim injection you did not observe.

---

### Task 1: Spike 3 — both browsers proxied through mitmproxy, TLS accepted, content script injects

**Files:**
- Create: `e2e/snapshots/serve_addon.py`, `e2e/snapshots/index.json` (seed), `e2e/snapshots/spike/example.test.html`
- Create: `e2e/spikes/spike-snapshot.ts`
- Modify: `e2e/spikes/RESULTS.md` (append "## Web snapshots / mitmproxy")

**Interfaces:**
- Produces: the confirmed mitmdump invocation and flags for this mitmproxy version, proof that Chromium (Playwright proxy + `ignoreHTTPSErrors`) and Firefox (proxy prefs + `acceptInsecureCerts`) both load `https://example.test/` served by the addon, and proof that an extension content script injects on that served page in both browsers.

- [ ] **Step 1: Install mitmproxy via uv.**

Run: `uv tool install mitmproxy`
Run: `mitmdump --version`
Record the version (flag names below may differ by version; the spike confirms them).

- [ ] **Step 2: Write the serve-addon.** `e2e/snapshots/serve_addon.py`:

```python
"""mitmproxy serve-addon: returns a stored HTML capture for each snapshotted host, offline.
Selected by the C2M_SNAPSHOT_ID env var (the corpus entry id). Document requests to a
snapshotted host get the stored HTML; any other request to that host gets an empty 200 so
subresource loads do not hang. Hosts not in the snapshot are refused (no live network)."""
import json
import os
from mitmproxy import http

BASE = os.path.dirname(__file__)
ENTRY = os.environ.get("C2M_SNAPSHOT_ID", "")
_index = json.load(open(os.path.join(BASE, "index.json")))
_hosts = {}
for e in _index.get(ENTRY, []):
    _hosts[e["host"]] = os.path.join(BASE, ENTRY, e["file"])


def request(flow: http.HTTPFlow) -> None:
    host = flow.request.pretty_host
    if host not in _hosts:
        flow.response = http.Response.make(204, b"", {})
        return
    accept = flow.request.headers.get("accept", "")
    is_doc = flow.request.method == "GET" and ("text/html" in accept or flow.request.path in ("/", ""))
    if is_doc:
        with open(_hosts[host], "rb") as f:
            body = f.read()
        flow.response = http.Response.make(200, body, {"Content-Type": "text/html; charset=utf-8"})
    else:
        flow.response = http.Response.make(200, b"", {"Content-Type": "application/octet-stream"})
```

`e2e/snapshots/index.json` (seed with the spike host):

```json
{ "spike": [ { "host": "example.test", "file": "example.test.html", "sha256": "" } ] }
```

`e2e/snapshots/spike/example.test.html`:

```html
<!DOCTYPE html><html><head><title>snapshot spike</title></head>
<body><h1 id="snapshot">served by mitmproxy</h1></body></html>
```

- [ ] **Step 3: Write the spike driver.** `e2e/spikes/spike-snapshot.ts`: start `mitmdump` on a port with `C2M_SNAPSHOT_ID=spike` and `-s snapshots/serve_addon.py` (confirm the quiet flag `-q` and whether a listen-port flag `-p` is needed for this version). Launch a Chromium persistent context with `proxy: { server: "http://127.0.0.1:<port>" }` and `ignoreHTTPSErrors: true`, load `https://example.test/`, and assert the page has `#snapshot`. Launch Firefox with proxy prefs (`network.proxy.type=1`, `network.proxy.http`/`network.proxy.ssl` = `127.0.0.1`, matching ports, `network.proxy.allow_hijacking_localhost=true`) and `acceptInsecureCerts`, load `https://example.test/`, assert `#snapshot`. Load a tiny content-script extension (reuse `testdata/hello-extension` with `matches: ["https://example.test/*"]`) and confirm the content script's telemetry hit appears for the served page in both browsers.

- [ ] **Step 4: Run the spike.**

Run: `cd e2e && E2E_INTEGRATION=1 pnpm exec tsx spikes/spike-snapshot.ts`
Determine the exact mitmdump flags that work; iterate the addon/flags until both browsers load the served page and the content script injects.

- [ ] **Step 5: Record the verdict** in `e2e/spikes/RESULTS.md` under "## Web snapshots / mitmproxy": the working mitmdump command line, the proxy config per browser, TLS approach (accept-insecure, no NSS import), and confirmation that content scripts inject on the served host in both browsers.

- [ ] **Step 6: Commit.**

```bash
cd /Users/otsov/.superset/worktrees/1b400713-93e8-4131-93b2-695e1e5e7562/web-snapshots
git add e2e/snapshots e2e/spikes/spike-snapshot.ts e2e/spikes/RESULTS.md
git commit -m "spike(e2e): mitmproxy serve-addon + both browsers proxied, TLS accepted"
```

---

### Task 2: Domain discovery

**Files:**
- Create: `e2e/src/domains.ts`
- Test: `e2e/tests/domains.test.ts`

**Interfaces:**
- Produces: `discoverDomains(manifest: Record<string, any>, extraDomains?: string[]): string[]` returning up to 20 concrete hostnames from `content_scripts[].matches` + `host_permissions` + `extraDomains`, dropping `<all_urls>` and wildcard-only host patterns (`*://*/*`), and de-wildcarding `*.example.com` to `example.com`.

- [ ] **Step 1: Write failing tests.** `e2e/tests/domains.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { discoverDomains } from "../src/domains.js";

describe("discoverDomains", () => {
  it("extracts hosts from content_scripts.matches and host_permissions", () => {
    const m = {
      content_scripts: [{ matches: ["https://onenote.officeapps.live.com/*"] }],
      host_permissions: ["https://www.onenote.com/*"],
    };
    expect(discoverDomains(m).sort()).toEqual(["onenote.officeapps.live.com", "www.onenote.com"]);
  });

  it("drops <all_urls> and wildcard-only host patterns", () => {
    const m = { host_permissions: ["<all_urls>", "*://*/*", "https://x.example.com/*"] };
    expect(discoverDomains(m)).toEqual(["x.example.com"]);
  });

  it("de-wildcards a leading *. and de-dupes, capping at 20", () => {
    const m = { content_scripts: [{ matches: ["https://*.example.com/*", "https://example.com/*"] }] };
    expect(discoverDomains(m)).toEqual(["example.com"]);
    const many = { host_permissions: Array.from({ length: 30 }, (_, i) => `https://h${i}.test/*`) };
    expect(discoverDomains(many).length).toBe(20);
  });

  it("includes extra_domains", () => {
    expect(discoverDomains({}, ["manual.test"])).toEqual(["manual.test"]);
  });
});
```

- [ ] **Step 2: Run, confirm fail.** `cd e2e && pnpm test -- domains` (FAIL, module missing).

- [ ] **Step 3: Implement `e2e/src/domains.ts`:**

```typescript
// Domain discovery for web snapshots: the hosts an extension actually targets, so a snapshot
// can be served under the real hostname and the extension's content scripts inject.
const HOST_RE = /^(?:\*|[a-z][a-z0-9+.-]*):\/\/([^/*]+|\*\.[^/*]+)(?:\/.*)?$/i;

function hostOf(pattern: string): string | null {
  if (pattern === "<all_urls>") return null;
  const m = pattern.match(HOST_RE);
  if (!m) return null;
  let host = m[1];
  if (host === "*") return null; // wildcard-only host, keep to standard fixtures
  if (host.startsWith("*.")) host = host.slice(2); // *.example.com -> example.com
  return host;
}

export function discoverDomains(manifest: Record<string, any>, extraDomains: string[] = []): string[] {
  const out = new Set<string>();
  const patterns: string[] = [];
  for (const cs of manifest.content_scripts ?? []) for (const p of cs.matches ?? []) patterns.push(p);
  for (const p of manifest.host_permissions ?? []) patterns.push(p);
  for (const p of patterns) { const h = hostOf(p); if (h) out.add(h); }
  for (const d of extraDomains) out.add(d);
  return [...out].slice(0, 20);
}
```

- [ ] **Step 4: Run, confirm pass.** `cd e2e && pnpm test -- domains` (PASS). Then `pnpm typecheck`.

- [ ] **Step 5: Commit.**

```bash
git add e2e/src/domains.ts e2e/tests/domains.test.ts
git commit -m "feat(e2e): domain discovery for web snapshots"
```

---

### Task 3: Drivers accept a proxy; snapshot loader

**Files:**
- Modify: `e2e/src/chromeDriver.ts` (`launchChrome(extDir, opts?)` proxy), `e2e/src/firefoxDriver.ts` (`launchFirefox(xpi, geckoId, opts?)` proxy)
- Create: `e2e/src/snapshots.ts` (index loader + mitmdump lifecycle)
- Test: `e2e/tests/domains.test.ts` (extend for index loading) or a small `e2e/tests/snapshots.test.ts`

**Interfaces:**
- Produces: `launchChrome(extDir: string, opts?: { proxyServer?: string }): Promise<BrowserSession>` and `launchFirefox(xpi: string, geckoId: string, opts?: { proxyServer?: string }): Promise<BrowserSession>` (both keep working with no opts). `snapshots.ts` exports `loadSnapshotIndex(): Record<string, {host:string;file:string;sha256:string}[]>`, `hasSnapshot(id): boolean`, and `startSnapshotServer(id: string, port: number): Promise<{ proxyServer: string; close(): Promise<void> }>` which spawns `mitmdump` with the serve-addon and `C2M_SNAPSHOT_ID=id`.

- [ ] **Step 1: Add the proxy option to Chromium.** In `e2e/src/chromeDriver.ts`, change the signature to `launchChrome(extDir: string, opts: { proxyServer?: string } = {})` and pass into `launchPersistentContext` options: when `opts.proxyServer` is set, add `proxy: { server: opts.proxyServer }` and `ignoreHTTPSErrors: true`. Keep `--headless=new` and the existing args unchanged.

- [ ] **Step 2: Add the proxy option to Firefox.** In `e2e/src/firefoxDriver.ts`, change to `launchFirefox(xpi: string, geckoId: string, opts: { proxyServer?: string } = {})`. When `opts.proxyServer` is set (parse host:port from `http://host:port`), set prefs `network.proxy.type=1`, `network.proxy.http`/`network.proxy.ssl` = host, matching `_port` prefs, `network.proxy.allow_hijacking_localhost=true`, and call `opts_.setAcceptInsecureCerts(true)`. Keep `-headless` and `--allow-system-access` unchanged.

- [ ] **Step 3: Write `e2e/src/snapshots.ts`.** Loads `e2e/snapshots/index.json`, exposes `hasSnapshot(id)`, and `startSnapshotServer(id, port)` that spawns `mitmdump` (via `child_process.spawn`) with the flags confirmed in Task 1, env `C2M_SNAPSHOT_ID=id`, waits until the port is listening, and returns `{ proxyServer: "http://127.0.0.1:" + port, close }` where `close` kills the process. Include the discovered snapshot URLs helper `snapshotUrls(id): string[]` returning `https://<host>/` for each entry.

- [ ] **Step 4: Unit test the index loader.** In `e2e/tests/snapshots.test.ts`, assert `loadSnapshotIndex()` parses the seed and `hasSnapshot("spike")` is true, `hasSnapshot("nope")` is false. (Do not spawn mitmdump in unit tests; the process lifecycle is covered by the Task 5 e2e run.)

- [ ] **Step 5: Typecheck + run unit tests.**

Run: `cd e2e && pnpm typecheck && pnpm test -- snapshots domains`
Expected: PASS.

- [ ] **Step 6: Commit.**

```bash
git add e2e/src/chromeDriver.ts e2e/src/firefoxDriver.ts e2e/src/snapshots.ts e2e/tests/snapshots.test.ts
git commit -m "feat(e2e): proxy option on drivers + snapshot server lifecycle"
```

---

### Task 4: Wire snapshots into the run

**Files:**
- Modify: `e2e/src/run.ts` (start the snapshot server for entries that have a snapshot; proxy both drivers; navigate snapshot URLs)
- Modify: `e2e/src/probes.ts` (`contentProbe`/`pingProbe` also navigate snapshot URLs from the context)
- Modify: `e2e/src/corpus.ts` (`CorpusEntry` gains optional `extra_domains?: string[]`)

**Interfaces:**
- Consumes: Tasks 2-3. `run.ts` computes whether an entry has a snapshot; if so, starts `startSnapshotServer`, launches both drivers with `{ proxyServer }`, and passes the snapshot URLs into the `ProbeContext` (add `snapshotUrls: string[]` to `ProbeContext`). `contentProbe`/`pingProbe` navigate those URLs (in addition to the fixtures) so content scripts matching the real host inject.

- [ ] **Step 1: Extend `ProbeContext`** in `e2e/src/probes.ts` with `snapshotUrls: string[]`. In `contentProbe`, after the fixture loop, for each `url` in `p.snapshotUrls` where `contentScriptsCoverUrl(p.manifest, url)`, `await p.chrome.open(url); await p.firefox.open(url); await settle(2500);`. In `pingProbe`, prefer a snapshot URL that content scripts cover, if any, over the fixture (so OneNote's relay can actually round-trip).

- [ ] **Step 2: Wire `run.ts`.** In `runOne`, before launching browsers: `const snap = hasSnapshot(entry.id) ? await startSnapshotServer(entry.id, SNAPSHOT_PORT) : null;` (add `const SNAPSHOT_PORT = 41980;`). Launch `launchChrome(chromeDir, { proxyServer: snap?.proxyServer })` and `launchFirefox(xpi, geckoId, { proxyServer: snap?.proxyServer })`. Build `snapshotUrls` via `snap ? snapshotUrls(entry.id) : []` and pass into each `probe({... , snapshotUrls})`. In the `finally`, `await snap?.close();`. Import `hasSnapshot`, `startSnapshotServer`, `snapshotUrls` from `./snapshots.js`.

- [ ] **Step 3: Add `extra_domains`** to `CorpusEntry` in `e2e/src/corpus.ts` (optional `string[]`), used by the snapshot build script (Task 5). No behavior change in `run.ts` beyond typing.

- [ ] **Step 4: Typecheck.**

Run: `cd e2e && pnpm typecheck`
Expected: clean.

- [ ] **Step 5: Commit.**

```bash
git add e2e/src/run.ts e2e/src/probes.ts e2e/src/corpus.ts
git commit -m "feat(e2e): run extensions against served web snapshots"
```

---

### Task 5: OneNote snapshot + build script; verify injection; CI

**Files:**
- Create: `e2e/snapshots/gojbdfnpnhogfdgjbigejoaolejmgdhk/onenote.officeapps.live.com.html`, update `e2e/snapshots/index.json`
- Create: `e2e/src/snapshotBuild.ts` + a `"snapshot"` script in `e2e/package.json`
- Modify: `.github/workflows/e2e.yml` (install mitmproxy via uv)
- Modify: `docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md` (Web snapshots section + Spike 3)

**Interfaces:**
- Produces: OneNote's content scripts (`matches: ["...onenote.officeapps.live.com..."]`) inject on the served snapshot in both browsers, so `contentProbe`/`pingProbe` no longer skip for that entry.

- [ ] **Step 1: Create the OneNote snapshot.** Determine OneNote's `content_scripts.matches` host (inspect `e2e/results/gojbdfnpnhogfdgjbigejoaolejmgdhk/source/manifest.json`). Create a minimal but real-enough HTML capture at `e2e/snapshots/gojbdfnpnhogfdgjbigejoaolejmgdhk/<host>.html` (logged-out landing page structure is fine per spec: a valid HTML doc with `<head>`/`<body>` and a title). Add the entry to `e2e/snapshots/index.json` with its sha256 (compute with `shasum -a 256`). Optionally add `extra_domains` to the corpus entry if the content scripts need a second host.

- [ ] **Step 2: Write the build script (optional live capture path).** `e2e/src/snapshotBuild.ts` + `"snapshot": "tsx src/snapshotBuild.ts"` in `e2e/package.json`: for `--only <id>`, discover domains, and (manual, live-network) either save a real capture per host or write the minimal HTML placeholder plus its sha256 into `index.json`. This script is never run in CI. Keep it small; the minimal deliverable is the stored HTML from Step 1, so a placeholder-writing implementation that fills sha256 is sufficient.

- [ ] **Step 3: Build + confirm headless + run OneNote against the snapshot.**

Run: `cargo build --release`
Run: `grep -n headless e2e/src/chromeDriver.ts e2e/src/firefoxDriver.ts`
Run: `cd e2e && pnpm e2e --only gojbdfnpnhogfdgjbigejoaolejmgdhk`
Read the report and traces. Confirm `contentProbe` now reports `ran` (not skipped) and the content-script telemetry appears for the OneNote host on BOTH browsers. Re-triage any NEW divergences the newly-injected content scripts surface with tight patterns + notes (do not blanket-allow). OneNote stays `quarantined: true` unless it now fully passes.

- [ ] **Step 4: CI stays replay/serve-only.** In `.github/workflows/e2e.yml`, add a step before "Differential e2e" to install mitmproxy via uv (e.g. `uv tool install mitmproxy` after setting up uv, or `pipx`-free `uv tool`), and ensure `mitmdump` is on PATH for the `xvfb-run ... pnpm e2e` step. No live network is used (serve-addon only).

- [ ] **Step 5: Update the spec.** In `docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md`, change the "Web snapshots" section and Spike 3 from deferred/not-run to implemented (minimal cut): domain discovery, mitmproxy serve-addon under the real hostname, both browsers proxied with accept-insecure TLS, CI serve-only. Note what is deferred (live flow record/replay archives, GH-release upload, hash-pinned distribution) vs. shipped (stored HTML capture served offline).

- [ ] **Step 6: Full verification.**

Run: `cd e2e && pnpm typecheck && pnpm test && pnpm e2e`
Expected: harness typecheck/unit green; full corpus run: LatexToCalc PASS, offscreen-gate PASS, OneNote reported with content scripts now injecting.

- [ ] **Step 7: Commit.**

```bash
git add e2e/snapshots e2e/src/snapshotBuild.ts e2e/package.json .github/workflows/e2e.yml e2e/corpus.json docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md
git commit -m "feat(e2e): OneNote web snapshot + serve-addon wiring; CI mitmproxy (#3)"
```

---

### Task 6: Push and open the PR

- [ ] **Step 1: Push.**

```bash
git push -u origin feature/e2e-web-snapshots
```

- [ ] **Step 2: Open the PR (do NOT merge).**

```bash
gh pr create --base main --head feature/e2e-web-snapshots \
  --title "feat(e2e): web snapshot corpus, mitmproxy serve (Plan 2 minimal)" \
  --body "Minimal Plan 2 (issue #3): domain discovery (content_scripts + host_permissions + extra_domains, cap 20, wildcard-only dropped); a mitmproxy serve-addon returns a stored HTML capture under the real hostname (offline, CI serve-only); both drivers proxied with accept-insecure TLS (no NSS import). run.ts serves the snapshot so content scripts matching the real host inject. OneNote's content scripts now inject on both browsers (was: skipped). Spike 3 verdict + working mitmdump flags in spikes/RESULTS.md. Deferred: live flow record/replay archives, GH-release hash-pinned distribution. Both browsers headless."
```

Report the PR URL, the Spike 3 verdict per browser, and whether OneNote's content script injects on both browsers.

## Self-Review

- Spec coverage: Spike 3 first -> Task 1; domain discovery -> Task 2; serve-addon + drivers proxy + TLS -> Tasks 1/3; run wiring -> Task 4; OneNote snapshot + CI serve-only + spec update -> Task 5. Covered. Deferred items (live record/replay archives, GH-release upload, hash pinning) are named in the PR and spec, matching the "minimal cut" scope.
- Placeholder scan: the optional live-capture build script is explicitly reducible to a placeholder-writer that fills sha256 (Task 5 Step 2); no TODO/TBD. PR specifics filled at Step 2.
- Type consistency: `discoverDomains(manifest, extraDomains?)` used in Tasks 2/5; `launchChrome(extDir, opts?)` / `launchFirefox(xpi, geckoId, opts?)` threaded through `run.ts`; `startSnapshotServer`/`hasSnapshot`/`snapshotUrls` from `snapshots.ts` used in `run.ts`; `ProbeContext.snapshotUrls` added in `probes.ts` and set in `run.ts`; `CorpusEntry.extra_domains?` added in `corpus.ts`.
- Fallback stated in Global Constraints: if the full serve path is not working within ~60 minutes, ship discovery + addon + drivers proxy with OneNote injecting on at least one browser and state real vs. pending in the PR. Never claim unobserved injection.
