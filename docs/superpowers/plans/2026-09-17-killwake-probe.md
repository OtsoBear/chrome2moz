# Kill/Wake Background Probe Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the kill/wake probe (spec Probes #5, "the #1 real conversion failure class") so the harness force-terminates the extension background, fires a wake event, and diffs wake behavior and restored state across Chrome (converted-from) and Firefox (converted-to).

**Architecture:** Extend `BrowserSession` with `killBackground()`. Chromium terminates the extension service worker via a CDP `ServiceWorker.stopAllWorkers`; Firefox attempts event-page idle termination (spiked first, honest `kill-unsupported` fallback if headless automation cannot force or confirm it). A new `killWakeProbe` reads background state, kills, wakes by opening a fixture tab (a top-level-registered `tabs.onUpdated` listener wakes the background), and reads state again. State and wake are made fully trace-observable through the existing spy shim: the gate extension's content script reads `chrome.storage.local` on each fixture page, so `storage.local.get:resolve` carries the boot counter and last-wake reason into the normal per-context diff, no new telemetry channel.

**Tech Stack:** Playwright CDP (Chromium), Selenium + geckodriver (Firefox), TypeScript, vitest, the existing e2e harness.

**Spec:** `docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md` § Probes #5 and § Phasing (v1.1); GitHub issue #5 (first bullet).

## Global Constraints

- **Headless always.** Chromium `--headless=new`, Firefox `-headless`. Run `grep -n headless e2e/src/chromeDriver.ts e2e/src/firefoxDriver.ts` before any corpus run; never headed. (Prior session: ~200 popups.)
- **pnpm only** in `e2e/`; **uv only** for Python. **No em dashes** anywhere.
- **Never fake parity** (spec): when a side cannot kill or confirm kill, the probe records `kill-unsupported` with the reason and does not assert the reboot on that side; it never reports equivalence it did not observe.
- Firefox/geckodriver via Selenium Manager; Playwright Chromium cached. Build converter with `cargo build --release` when running the corpus.
- Work only in worktree `/Users/otsov/.superset/worktrees/1b400713-93e8-4131-93b2-695e1e5e7562/killwake-probe` on branch `feature/e2e-killwake-probe`. Never cd into sibling worktrees or the main checkout.

---

### Task 1: Spike Firefox event-page termination

**Files:**
- Create: `e2e/spikes/spike-killwake.ts`
- Modify: `e2e/spikes/RESULTS.md` (append a "Kill/Wake" section)

**Interfaces:**
- Produces: a documented verdict on whether Firefox's converted event-page background can be force-terminated (or reliably idle-terminated) and observed to reboot under headless Selenium, plus the confirmed Chromium CDP stop mechanism. This verdict decides whether Task 4's gate needs a Firefox `kill-unsupported` allowance.

- [ ] **Step 1: Write the spike.** In `e2e/spikes/spike-killwake.ts`, load `testdata/hello-extension` (converted by hand as `drivers.integration.test.ts` does), in each browser: boot the background, then attempt to terminate it. Chromium: `const cdp = await ctx.newCDPSession(page); await cdp.send("ServiceWorker.enable"); await cdp.send("ServiceWorker.stopAllWorkers");` then confirm the worker restarts on the next event. Firefox: set `extensions.background.idle.timeout` to `1000` on the profile, leave the event page idle > 1s, then fire an event (open a tab) and check whether the background re-ran its top-level code (e.g. a `storage.local` boot counter incremented). Log outcomes to console.

- [ ] **Step 2: Run the spike (both browsers, headless).**

Run: `cd e2e && E2E_INTEGRATION=1 pnpm exec tsx spikes/spike-killwake.ts`
Observe: does the Chromium SW stop and reboot? Does the Firefox event page terminate and reboot within the idle window?

- [ ] **Step 3: Record the verdict** in `e2e/spikes/RESULTS.md` under a new "## Kill/Wake" heading, following the existing format: exact mechanism per browser, whether reboot was observed, and (for Firefox) whether idle termination is confirmable under headless automation or must be treated as `kill-unsupported`.

- [ ] **Step 4: Commit.**

```bash
cd /Users/otsov/.superset/worktrees/1b400713-93e8-4131-93b2-695e1e5e7562/killwake-probe
git add e2e/spikes/spike-killwake.ts e2e/spikes/RESULTS.md
git commit -m "spike(e2e): kill/wake background termination per browser"
```

---

### Task 2: Add killBackground to the drivers

**Files:**
- Modify: `e2e/src/chromeDriver.ts` (`BrowserSession` interface + Chromium impl)
- Modify: `e2e/src/firefoxDriver.ts` (Firefox impl)
- Test: `e2e/tests/drivers.integration.test.ts` (integration, gated on `E2E_INTEGRATION`)

**Interfaces:**
- Produces: `BrowserSession.killBackground(): Promise<KillResult>` where `type KillResult = { killed: boolean; mechanism: string; note?: string }`. Chromium returns `{ killed: true, mechanism: "cdp:ServiceWorker.stopAllWorkers" }` on success. Firefox returns `{ killed: true, mechanism: "idle-timeout" }` if the spike confirmed it, else `{ killed: false, mechanism: "idle-timeout", note: "event-page termination not confirmable under headless WebDriver" }`.

- [ ] **Step 1: Extend the interface.** In `e2e/src/chromeDriver.ts`, add to the `BrowserSession` interface:

```typescript
  killBackground(): Promise<KillResult>;
```

and export the type near the interface:

```typescript
export type KillResult = { killed: boolean; mechanism: string; note?: string };
```

- [ ] **Step 2: Write the failing integration test.** Add to `e2e/tests/drivers.integration.test.ts`:

```typescript
itIf("chromium killBackground stops the extension service worker", async () => {
  const dir = mkdtempSync(join(tmpdir(), "c2m-kill-"));
  cpSync(resolve("testdata/hello-extension"), dir, { recursive: true });
  instrumentExtension(dir, "chrome-orig", 41994);
  const t = await startTelemetry(41994);
  const s = await launchChrome(dir);
  await new Promise((r) => setTimeout(r, 2000));
  const res = await s.killBackground();
  expect(res.killed).toBe(true);
  expect(res.mechanism).toContain("ServiceWorker");
  await s.close();
  await t.close();
}, 60000);
```

- [ ] **Step 3: Run it, confirm it fails.**

Run: `cd e2e && E2E_INTEGRATION=1 pnpm test -- drivers`
Expected: FAIL (`killBackground` not implemented).

- [ ] **Step 4: Implement Chromium killBackground.** In `e2e/src/chromeDriver.ts`, inside the object returned by `launchChrome`, add (using the closed-over `ctx` and the current `page`):

```typescript
    async killBackground() {
      try {
        const target = ctx.pages()[0] ?? page;
        const cdp = await ctx.newCDPSession(target);
        await cdp.send("ServiceWorker.enable");
        await cdp.send("ServiceWorker.stopAllWorkers");
        await cdp.detach().catch(() => {});
        return { killed: true, mechanism: "cdp:ServiceWorker.stopAllWorkers" };
      } catch (e) {
        return { killed: false, mechanism: "cdp:ServiceWorker.stopAllWorkers", note: String(e) };
      }
    },
```

- [ ] **Step 5: Implement Firefox killBackground** per the spike verdict. In `e2e/src/firefoxDriver.ts`, set the idle-timeout pref at launch (`opts.setPreference("extensions.background.idle.timeout", 1000);`) and add to the returned object:

```typescript
    async killBackground() {
      // Firefox has no WebDriver command to terminate an event page. With a low idle timeout
      // set at launch, staying idle past the window lets it terminate; whether that is
      // confirmable under headless automation is decided by the kill/wake spike. Report
      // honestly rather than claiming a kill we cannot observe.
      await driver.sleep(1500); // > idle timeout
      // If the spike confirmed idle termination + reboot, return killed:true. Otherwise:
      return { killed: false, mechanism: "idle-timeout", note: "event-page termination not confirmable under headless WebDriver (see spikes/RESULTS.md Kill/Wake)" };
    },
```

Set `killed: true` here only if Task 1's spike actually confirmed Firefox idle termination + reboot; otherwise leave `killed: false` with the note.

- [ ] **Step 6: Run the integration test.**

Run: `cd e2e && E2E_INTEGRATION=1 pnpm test -- drivers`
Expected: the Chromium killBackground test PASSES. Run `pnpm typecheck` too.

- [ ] **Step 7: Commit.**

```bash
git add e2e/src/chromeDriver.ts e2e/src/firefoxDriver.ts e2e/tests/drivers.integration.test.ts
git commit -m "feat(e2e): killBackground driver capability (Chromium CDP; Firefox idle)"
```

---

### Task 3: The killWakeProbe

**Files:**
- Modify: `e2e/src/probes.ts` (add `killWakeProbe`, append it to `ALL_PROBES`)
- Test: `e2e/tests/probes.test.ts` (unit test the result-shaping logic if it is factored pure; otherwise rely on the Task 4 gate)

**Interfaces:**
- Consumes: `ProbeContext` (has `chrome`, `firefox`, `telemetry`, `manifest`, `fixtureUrl`, `resultsDir`) and `killBackground()` from Task 2.
- Produces: a `ProbeResult` named `kill-wake`. Behavior: open a fixture tab on both sides (initial boot, content script reads `storage.local` -> traced), call `killBackground()` on both, open a SECOND fixture tab on both (wake trigger; a top-level `tabs.onUpdated` listener in the gate reboots the background, content script reads `storage.local` again -> traced). Status: `ran` when both sides were killed and rebooted; `skipped` with a note naming the side when a side reported `kill-unsupported` (so the diff's boots divergence on that side is expected, handled by an allowance in the gate, not by faking parity); `failed` only if a killed side did NOT reboot (a real wake regression).

- [ ] **Step 1: Implement the probe.** In `e2e/src/probes.ts`, add:

```typescript
export async function killWakeProbe(p: ProbeContext): Promise<ProbeResult> {
  const hasBackground = !!(p.manifest.background && (p.manifest.background.service_worker || p.manifest.background.scripts));
  if (!hasBackground) return { name: "kill-wake", status: "skipped", note: "no background" };
  const settle = (ms: number) => new Promise((r) => setTimeout(r, ms));

  // Initial boot: content script on the fixture page reads storage.local (traced as
  // storage.local.get:resolve on both sides).
  const url = p.fixtureUrl("basic.html");
  await p.chrome.open(url);
  await p.firefox.open(url);
  await settle(2000);

  const ck = await p.chrome.killBackground();
  const fk = await p.firefox.killBackground();
  await settle(800);

  // Wake: opening a second tab fires tabs.onUpdated, which a top-level listener uses to reboot
  // and re-read/rewrite storage.local (traced again with an incremented boot counter).
  await p.chrome.open(url);
  await p.firefox.open(url);
  await settle(2500);

  const note = `chrome:${JSON.stringify(ck)} firefox:${JSON.stringify(fk)}`;
  if (!ck.killed || !fk.killed) {
    return { name: "kill-wake", status: "skipped", note: `kill-unsupported on a side -- ${note}` };
  }
  return { name: "kill-wake", status: "ran", note };
}
```

and add `killWakeProbe` to the `ALL_PROBES` array.

- [ ] **Step 2: Typecheck.**

Run: `cd e2e && pnpm typecheck`
Expected: clean.

- [ ] **Step 3: Commit.**

```bash
git add e2e/src/probes.ts
git commit -m "feat(e2e): kill/wake background probe"
```

---

### Task 4: The killwake-gate extension + corpus entry, verify

**Files:**
- Create: `e2e/testdata/killwake-extension/manifest.json`, `background.js`, `content.js`
- Modify: `e2e/corpus.json` (new entry), `docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md` (Probes #5, Phasing)

**Interfaces:**
- Consumes: Tasks 2-3. The gate background counts boots in `storage.local` and re-reads/updates on `tabs.onUpdated`; the content script reads `storage.local` on each fixture page so the boot counter reaches the trace via `storage.local.get:resolve`.

- [ ] **Step 1: Create the extension.** `e2e/testdata/killwake-extension/manifest.json`:

```json
{
  "manifest_version": 3,
  "name": "C2M KillWake Gate",
  "version": "1.0",
  "background": { "service_worker": "background.js" },
  "permissions": ["storage", "tabs"],
  "content_scripts": [
    { "matches": ["http://127.0.0.1/*"], "js": ["content.js"], "run_at": "document_idle" }
  ]
}
```

`e2e/testdata/killwake-extension/background.js` (top-level runs on every (re)boot; a top-level tabs.onUpdated listener wakes it after termination):

```javascript
// Boot counter in storage.local (persists across background restarts on BOTH browsers,
// unlike storage.session, so the wake behavior is what is under test, not the session shim).
// In-memory bootMark resets on every restart; both signals reach the trace via the content
// script's storage.local.get.
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
```

`e2e/testdata/killwake-extension/content.js`:

```javascript
// Read the boot state on every fixture page load; the spy shim records
// storage.local.get:resolve [{boots, lastWake, ...}] on both browsers, so the diff compares
// how the background rebooted after the kill.
chrome.storage.local.get(["boots", "lastWake"]).then(() => {});
```

- [ ] **Step 2: Add the corpus entry.** Append to `e2e/corpus.json` `extensions`. If the Task 1 spike confirmed Firefox idle termination, use `allowed_diffs: []`. If Firefox kill is unsupported, allow the boots divergence with a documented harness-limitation note:

```json
{
  "id": "killwake-gate",
  "name": "C2M KillWake Gate",
  "source": "local:testdata/killwake-extension",
  "allowed_diffs": [],
  "quarantined": false,
  "_notes": {
    "_purpose": "Kill/wake probe gate (spec Probes #5). Background counts boots in storage.local; killWakeProbe boots it, kills the background (Chromium CDP ServiceWorker.stopAllWorkers; Firefox idle), then opens a second fixture tab to wake it via tabs.onUpdated. The content script reads storage.local on each page so the boot counter reaches the trace. Both sides killed+rebooted => boots go 1 -> 2 identically and MATCH.",
    "_firefox_kill_caveat": "FILL ONLY IF the spike found Firefox event-page termination unsupported under headless WebDriver: then Firefox does not reboot, boots stays 1 on that side, and storage.local.get:resolve#\"boots\":1 is allowed firefox-only with this reason. Chromium provides the real kill/wake coverage. Remove this allowance if/when Firefox idle termination becomes confirmable."
  }
}
```

Keep `_firefox_kill_caveat` only if needed; delete it and keep `allowed_diffs: []` if the spike confirmed Firefox kill.

- [ ] **Step 3: Build + confirm headless + run the gate.**

Run: `cargo build --release`
Run: `grep -n headless e2e/src/chromeDriver.ts e2e/src/firefoxDriver.ts`
Run: `cd e2e && pnpm e2e --only killwake-gate`
Read `e2e/results/killwake-gate/trace-chrome.json` / `trace-firefox.json`. Confirm Chromium shows two `storage.local.get:resolve` reads with `boots` going 1 then 2 and `lastWake:"tabs.onUpdated"`. Confirm the Firefox side per the spike verdict (either matching 1 then 2, or staying at 1 with the documented allowance). Expected: `PASS  C2M KillWake Gate`.

- [ ] **Step 4: Update the spec.** In `docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md`, change Probes #5 and the Phasing v1.1 line from "not yet implemented" to implemented, describing the CDP stop on Chromium, the Firefox idle mechanism and its confirmed limitation (if any), the tabs.onUpdated wake, and the storage.local boot-counter observable.

- [ ] **Step 5: Full verification.**

Run: `cd e2e && pnpm typecheck && pnpm test && pnpm e2e`
Expected: harness typecheck/unit green; full corpus `PASS LatexToCalc`, `PASS C2M Offscreen Gate`, `PASS C2M KillWake Gate`, OneNote reported.

- [ ] **Step 6: Commit.**

```bash
git add e2e/testdata/killwake-extension e2e/corpus.json docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md
git commit -m "test(e2e): kill/wake gate extension + corpus entry (#5)"
```

---

### Task 5: Push and open the PR

- [ ] **Step 1: Push.**

```bash
git push -u origin feature/e2e-killwake-probe
```

- [ ] **Step 2: Open the PR (do NOT merge).**

```bash
gh pr create --base main --head feature/e2e-killwake-probe \
  --title "feat(e2e): kill/wake background probe" \
  --body "Implements the kill/wake probe (spec Probes #5, the #1 real conversion failure class). New killBackground driver capability: Chromium via CDP ServiceWorker.stopAllWorkers (confirmed), Firefox via event-page idle timeout (spike verdict in spikes/RESULTS.md; honest kill-unsupported fallback if not confirmable under headless WebDriver). killWakeProbe boots, kills, and wakes the background via tabs.onUpdated by opening a second fixture tab; the killwake-gate extension's storage.local boot counter makes wake + state restoration fully trace-observable. Both browsers headless."
```

Report the PR URL and the Firefox kill spike verdict (killed+rebooted, or kill-unsupported with the documented allowance).

## Self-Review

- Spec coverage: kill mechanism per browser -> Task 2; wake + diff -> Task 3; gate + spec update -> Task 4; Firefox feasibility settled before building -> Task 1. Covered.
- Placeholder scan: the `_firefox_kill_caveat` and the Firefox `killed:` value are explicit spike-gated conditionals with concrete instructions, not TODOs; PR body specifics filled at Step 2.
- Type consistency: `KillResult` defined in `chromeDriver.ts`, used by both drivers and the probe; `killWakeProbe` added to `ALL_PROBES`; `ProbeResult` status values (`ran`/`skipped`/`failed`) match `probes.ts`. `ctx.newCDPSession` is Playwright `BrowserContext`.
- Never-fake-parity: a `kill-unsupported` side yields probe `skipped` + a documented, side-scoped allowance, never a silent equivalence claim.
