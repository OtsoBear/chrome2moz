# Clipboard Readback Observable Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the e2e harness a clipboard observable so a regression in an extension's clipboard output is caught, by tracing `navigator.clipboard.writeText`/`readText` (including the read-back value) symmetrically on both browsers and proving it with a deterministic gate extension.

**Architecture:** The spy shim (`e2e/shim/shim.js`) already wraps `chrome.*`/`browser.*` and `fetch` by direct property reassignment. Add the same treatment for `navigator.clipboard.writeText`/`readText`, so every clipboard call (and its resolved read-back value) becomes a normal trace event that flows through the existing per-context diff with zero `diff.ts` changes. Grant clipboard permissions to both drivers so headless writes/reads resolve rather than rejecting on user-activation grounds. A new `clipboard-gate` testdata extension writes a fixed string via a content script on the fixture page and reads it back, giving a permanent MATCH gate.

**Tech Stack:** Plain-JS spy shim; TypeScript drivers (Playwright for Chromium, Selenium for Firefox); vitest; the existing e2e harness.

**Spec:** `docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md` § Observables ("Clipboard readback (`navigator.clipboard.readText()` from a test page; permissions pre-granted) not implemented, deferred to Plan 3") and GitHub issue #4 section 3.

## Global Constraints

- **Headless always.** Chromium `--headless=new` (`e2e/src/chromeDriver.ts`), Firefox `-headless` (`e2e/src/firefoxDriver.ts`). Run `grep -n headless e2e/src/chromeDriver.ts e2e/src/firefoxDriver.ts` before any corpus run; never headed. (Prior session: ~200 popups.)
- **pnpm only** in `e2e/`; **uv only** for Python. **No em dashes** anywhere.
- `127.0.0.1` is a secure context in both browsers, so `navigator.clipboard` is available on the fixture pages.
- Firefox/geckodriver via Selenium Manager; Playwright Chromium cached. Build converter with `cargo build --release` when running the corpus.
- Work only in worktree `/Users/otsov/.superset/worktrees/1b400713-93e8-4131-93b2-695e1e5e7562/clipboard-readback` on branch `feature/e2e-clipboard-readback`. Never cd into sibling worktrees or the main checkout.
- **Transparency rule (spec):** the spy must only wrap existing properties, never add a missing one. `navigator.clipboard` and its methods are wrapped only when already present.

---

### Task 1: Trace clipboard calls in the spy shim

**Files:**
- Modify: `e2e/shim/shim.js` (add clipboard wrapping after the existing `fetch` wrap block, before the `error`/`unhandledrejection` listeners)
- Test: `e2e/tests/shim.test.ts`

**Interfaces:**
- Produces trace events: `clipboard.writeText [text]` and `clipboard.writeText:resolve` / `:reject`; `clipboard.readText []` and `clipboard.readText:resolve [value]` / `:reject`. `value` is the read-back clipboard content (the readback observable). Normalized through the shim's existing `norm` (200-char cap, ext-url/epoch scrub), so it flows through the diff like any other event.

- [ ] **Step 1: Write the failing test.** Add to `e2e/tests/shim.test.ts` inside `describe("shim", ...)`:

```typescript
it("records navigator.clipboard.writeText/readText calls and their resolved value", async () => {
  const store = { v: "" };
  fake = {
    ...fake,
  };
  // Provide a navigator.clipboard on the sandbox global for the shim to wrap.
  const clip = {
    writeText: (t: string) => { store.v = t; return Promise.resolve(); },
    readText: () => Promise.resolve(store.v),
  };
  const { sandbox, flush, posted } = loadShim(fake, {});
  (sandbox as any).navigator = { clipboard: clip };
  // Re-run the shim body is not needed; instead assert the shim wrapped what existed at load.
  // If the sandbox has no navigator at load, wrapping is skipped (transparency). So set
  // navigator BEFORE load: see helper change below.
  await (sandbox as any).navigator.clipboard.writeText("c2m-clip-xyz");
  const got = await (sandbox as any).navigator.clipboard.readText();
  flush();
  const apis = posted.flatMap((p: any) => p.events.map((e: any) => e.api));
  expect(got).toBe("c2m-clip-xyz");
  expect(apis).toContain("clipboard.writeText");
  expect(apis).toContain("clipboard.readText:resolve");
});
```

Because the shim wraps `navigator.clipboard` at load time, `navigator` must exist in the sandbox before `vm.runInContext`. Update `loadShim` in the same file to accept `opts.navigator` and set `sandbox.navigator = opts.navigator` before `vm.createContext`/`runInContext`, then in the test pass `loadShim(fake, { navigator: { clipboard: clip } })` and drop the post-load assignment.

- [ ] **Step 2: Run the test, confirm it fails.**

Run: `cd e2e && pnpm test -- shim`
Expected: FAIL (no `clipboard.*` events recorded).

- [ ] **Step 3: Implement the clipboard wrap.** In `e2e/shim/shim.js`, after the `if (rawFetch) { g.fetch = ... }` block and before the `g.addEventListener?.("error", ...)` block, add:

```javascript
  // Clipboard observable: wrap navigator.clipboard.writeText/readText the same way as fetch
  // (direct reassignment, record then delegate). readText:resolve carries the read-back value,
  // which is the clipboard readback observable. Transparency: only wraps methods that exist.
  try {
    const clip = g.navigator && g.navigator.clipboard;
    if (clip) {
      if (typeof clip.writeText === "function") {
        const origWrite = clip.writeText.bind(clip);
        try {
          clip.writeText = function (text) {
            try { record("clipboard.writeText", [norm(text)]); } catch {}
            const r = origWrite(text);
            if (r && typeof r.then === "function") r.then(
              () => { try { record("clipboard.writeText:resolve", []); } catch {} },
              (e) => { try { record("clipboard.writeText:reject", [String(e)]); } catch {} },
            );
            return r;
          };
        } catch {}
      }
      if (typeof clip.readText === "function") {
        const origRead = clip.readText.bind(clip);
        try {
          clip.readText = function () {
            try { record("clipboard.readText", []); } catch {}
            const r = origRead();
            if (r && typeof r.then === "function") r.then(
              (v) => { try { record("clipboard.readText:resolve", [norm(v)]); } catch {} },
              (e) => { try { record("clipboard.readText:reject", [String(e)]); } catch {} },
            );
            return r;
          };
        } catch {}
      }
    }
  } catch {}
```

- [ ] **Step 4: Run the test, confirm it passes.**

Run: `cd e2e && pnpm test -- shim`
Expected: PASS (all shim tests, including the existing transparency test which must still show `chrome.offscreen`/`tabGroups` undefined).

- [ ] **Step 5: Typecheck + commit.**

Run: `cd e2e && pnpm typecheck`

```bash
cd /Users/otsov/.superset/worktrees/1b400713-93e8-4131-93b2-695e1e5e7562/clipboard-readback
git add e2e/shim/shim.js e2e/tests/shim.test.ts
git commit -m "feat(e2e): trace navigator.clipboard writeText/readText in the spy shim"
```

---

### Task 2: Pre-grant clipboard permissions in both drivers

**Files:**
- Modify: `e2e/src/chromeDriver.ts` (grant clipboard permissions on the persistent context)
- Modify: `e2e/src/firefoxDriver.ts` (set clipboard test prefs so headless read/write resolve without a user gesture)

**Interfaces:**
- Produces: on both browsers, `navigator.clipboard.writeText`/`readText` from an extension content script on the fixture page RESOLVE (rather than reject with NotAllowedError), so the `:resolve` events are symmetric across sides.

- [ ] **Step 1: Grant permissions in Chromium.** In `e2e/src/chromeDriver.ts`, after `launchPersistentContext(...)` returns `ctx` and before waiting for the service worker, add:

```typescript
  // Clipboard readback observable: headless writeText/readText need explicit permission,
  // otherwise they reject with NotAllowedError and desync from Firefox.
  await ctx.grantPermissions(["clipboard-read", "clipboard-write"]).catch(() => {});
```

- [ ] **Step 2: Set clipboard prefs in Firefox.** In `e2e/src/firefoxDriver.ts`, on the `firefox.Options()` object (`opts`) before building the driver, add:

```typescript
  // Clipboard readback observable: the testing pref bypasses the transient-user-activation
  // requirement so headless writeText/readText resolve (matching Chromium's granted state).
  opts.setPreference("dom.events.asyncClipboard.readText", true);
  opts.setPreference("dom.events.asyncClipboard.clipboardItem", true);
  opts.setPreference("dom.events.testing.asyncClipboard", true);
```

- [ ] **Step 3: Typecheck + commit.**

Run: `cd e2e && pnpm typecheck`

```bash
git add e2e/src/chromeDriver.ts e2e/src/firefoxDriver.ts
git commit -m "feat(e2e): pre-grant clipboard permissions for both drivers"
```

---

### Task 3: The clipboard-gate extension + corpus entry

**Files:**
- Create: `e2e/testdata/clipboard-extension/manifest.json`, `content.js`
- Modify: `e2e/corpus.json` (new non-quarantined entry)

**Interfaces:**
- Consumes: Task 1 (shim clipboard tracing) + Task 2 (permissions). A content script on the fixture origin writes a fixed string and reads it back; both browsers must produce matching `clipboard.writeText ["c2m-clipboard-gate"]` and `clipboard.readText:resolve ["c2m-clipboard-gate"]` events in the `content` context.

- [ ] **Step 1: Create the extension.** `e2e/testdata/clipboard-extension/manifest.json`:

```json
{
  "manifest_version": 3,
  "name": "C2M Clipboard Gate",
  "version": "1.0",
  "permissions": ["clipboardRead", "clipboardWrite"],
  "content_scripts": [
    { "matches": ["http://127.0.0.1/*"], "js": ["content.js"], "run_at": "document_idle" }
  ]
}
```

`e2e/testdata/clipboard-extension/content.js`:

```javascript
// Deterministic clipboard round-trip: write a fixed string, read it back. The spy shim
// records clipboard.writeText [text] and clipboard.readText:resolve [value] on both browsers;
// with permissions pre-granted (harness drivers) both resolve, so the two traces MATCH.
(async () => {
  const MARK = "c2m-clipboard-gate";
  try {
    await navigator.clipboard.writeText(MARK);
    await navigator.clipboard.readText();
  } catch (e) {
    // Recorded as clipboard.*:reject on both sides symmetrically if permissions somehow fail.
  }
})();
```

- [ ] **Step 2: Add the corpus entry.** Append to `e2e/corpus.json` `extensions`:

```json
{
  "id": "clipboard-gate",
  "name": "C2M Clipboard Gate",
  "source": "local:testdata/clipboard-extension",
  "allowed_diffs": [],
  "quarantined": false,
  "_notes": {
    "_purpose": "Exercises the clipboard readback observable (issue #4.3): a content script writes a fixed string and reads it back. The spy shim records clipboard.writeText and clipboard.readText:resolve on both browsers; with clipboard permissions pre-granted by the drivers, both resolve identically, so the content-context traces MATCH with no allowance. A regression that changed the clipboard value (or dropped the read-back) would diverge and fail here."
  }
}
```

- [ ] **Step 3: Build + confirm headless + run the gate.**

Run: `cargo build --release`
Run: `grep -n headless e2e/src/chromeDriver.ts e2e/src/firefoxDriver.ts`
Run: `cd e2e && pnpm e2e --only clipboard-gate`
Read `e2e/results/clipboard-gate/trace-chrome.json` / `trace-firefox.json`. Confirm both contain `clipboard.writeText ["c2m-clipboard-gate"]` and `clipboard.readText:resolve ["c2m-clipboard-gate"]` in the `content` context, MATCHED (no unallowed divergence).
Expected console: `PASS  C2M Clipboard Gate`.

- [ ] **Step 4: If one side rejects (asymmetric resolve/reject).** Do NOT paper over it with an allowance blindly. First confirm Task 2 landed (Chromium `grantPermissions`, Firefox prefs) and that the converted content script actually runs (check `e2e/results/clipboard-gate/converted/content.js`). If, after that, a specific browser genuinely cannot resolve clipboard in headless on this platform, record that in the entry `_notes` with the exact error, and allow only the precise `clipboard.writeText:reject#<reason>` / `clipboard.readText:reject#<reason>` on that side, keeping the `clipboard.writeText`/`clipboard.readText` CALL events asserted (those must still match). State clearly in the note which browser and why.

- [ ] **Step 5: Update the spec.** In `docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md` § Observables, change the "Clipboard readback ... not implemented" line to implemented, describing the shim-level `clipboard.writeText`/`readText` trace observable (read-back value via `readText:resolve`), the driver permission pre-grant, and the `clipboard-gate` corpus entry.

- [ ] **Step 6: Full verification.**

Run: `cd e2e && pnpm typecheck && pnpm test && pnpm e2e`
Expected: harness typecheck/unit green; full corpus `PASS LatexToCalc`, `PASS C2M Offscreen Gate`, `PASS C2M Clipboard Gate`, OneNote reported.

- [ ] **Step 7: Commit.**

```bash
git add e2e/testdata/clipboard-extension e2e/corpus.json docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md
git commit -m "test(e2e): clipboard readback observable + gate extension (#4.3)"
```

---

### Task 4: Push and open the PR

- [ ] **Step 1: Push.**

```bash
git push -u origin feature/e2e-clipboard-readback
```

- [ ] **Step 2: Open the PR (do NOT merge).**

```bash
gh pr create --base main --head feature/e2e-clipboard-readback \
  --title "feat(e2e): clipboard readback observable" \
  --body "Implements the clipboard readback observable (issue #4.3, spec Observables). The spy shim now traces navigator.clipboard.writeText/readText, with readText:resolve carrying the read-back value; drivers pre-grant clipboard permissions (Chromium grantPermissions, Firefox test prefs) so headless ops resolve symmetrically. New clipboard-gate corpus entry proves a content-script write/read round-trip matches on both browsers with no allowance. Both browsers headless."
```

Report the PR URL and which browser(s) resolved clipboard vs. needed a documented reject allowance.

## Self-Review

- Spec coverage: clipboard readback observable (writeText + readText:resolve read-back value) -> Task 1; permissions pre-granted -> Task 2; deterministic gate + spec update -> Task 3. Covered.
- Placeholder scan: the reject-allowance branch (Task 3 Step 4) is a genuine conditional with a concrete `api#reason` pattern, not a placeholder; PR body specifics filled at Step 2.
- Type consistency: `record`/`norm` are the shim's existing helpers; `ctx.grantPermissions` is Playwright `BrowserContext`; `opts.setPreference` is `selenium-webdriver/firefox` `Options`. `loadShim`'s new `opts.navigator` is used consistently in the Task 1 test.
- Note: LatexToCalc's own clipboard write is gated behind the `translate-clipboard` command (dispatch-unsupported per spike C) and a translation server, so it is deliberately NOT the gate; the observable exists for it, but the deterministic proof uses the dedicated clipboard-gate. Stated in the corpus note.
