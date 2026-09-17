# E2E Spy-Shim Ordering Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Inject the e2e spy shim AFTER the converter's own `shims/*.js` compat layer in the converted (Firefox) background, so converter-added API surface is traced instead of appearing as false Chrome-only divergences, then re-triage the corpus `allowed_diffs` that were built against the buggy ordering.

**Architecture:** One targeted change in `e2e/src/injector.ts`: replace the unconditional `background.scripts.unshift(spy)` with an insert positioned immediately after the last `shims/*.js` entry (falling back to index 0 when there are none). Content scripts and HTML entry points are unchanged because the converter adds compat shims only to `background.scripts` (confirmed in `src/transformer/manifest.rs::transform_background`). After the fix, re-run the corpus and re-triage the `offscreen.*` / `runtime.getContexts*` allowances in `e2e/corpus.json`, which the buggy ordering forced.

**Tech Stack:** TypeScript (Node, tsx, vitest), the existing chrome2moz e2e harness in `e2e/`; Rust converter (`cargo build --release`) only as a dependency of running the corpus.

**Spec:** `docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md` (§ "Known limits" references this as issue #6) and GitHub issue #6.

## Global Constraints

- **Headless always.** Every browser launch must stay headless. Chromium launches via `--headless=new` (see `e2e/src/chromeDriver.ts`), Firefox via `-headless` (see `e2e/src/firefoxDriver.ts`). Before any corpus run, run `grep -n headless e2e/src/chromeDriver.ts e2e/src/firefoxDriver.ts` and confirm both are present. Never flip either to headed. (A prior session flooded the user with ~200 popup windows.)
- **pnpm only** inside `e2e/` (never npm/yarn). **uv only** for any Python (never pip). Not expected here.
- **No em dashes** in any code, comment, commit message, PR body, or corpus `_notes`.
- The converter binary must be built with `cargo build --release` for the harness to convert; the harness calls `buildConverter()` itself (`e2e/src/convert.ts`), but a manual `cargo build --release` up front surfaces Rust errors early.
- Firefox/geckodriver resolve through Selenium Manager (auto-download); do not install them manually. Playwright's Chromium is already cached.
- Work only in the worktree `/Users/otsov/.superset/worktrees/1b400713-93e8-4131-93b2-695e1e5e7562/shim-order` on branch `feature/e2e-shim-order`. Never cd into sibling worktrees or the main checkout.

---

### Task 1: Fix the injector ordering

**Files:**
- Modify: `e2e/src/injector.ts` (the `Array.isArray(m.background?.scripts)` branch of `instrumentExtension`, currently `m.background.scripts.unshift(BG_SHIM_NAME)`)
- Test: `e2e/tests/injector.test.ts`

**Interfaces:**
- Consumes: `instrumentExtension(dir: string, side: Side, port: number): void` (unchanged signature).
- Produces: for a `background.scripts` array, the spy shim `__c2m_shim_bg.js` is inserted at index `lastShimIdx + 1`, where `lastShimIdx` is the highest index whose entry starts with `"shims/"`; when no entry starts with `"shims/"`, it is inserted at index 0 (preserving today's behavior for non-converted inputs and the existing test).

- [ ] **Step 1: Write the failing test.** Add to `e2e/tests/injector.test.ts` inside the `describe("instrumentExtension", ...)` block:

```typescript
it("inserts the bg shim AFTER the converter's shims/*.js compat layer", () => {
  const dir = makeExt(
    {
      manifest_version: 3,
      background: {
        scripts: [
          "shims/storage-session-compat.js",
          "shims/runtime-compat.js",
          "config.js",
          "background.js",
        ],
      },
    },
    { "config.js": "", "background.js": "" },
  );
  instrumentExtension(dir, "firefox-conv", 41999);
  const m = readManifest(dir);
  // Spy shim must sit after the last shims/*.js entry, before the extension's own code,
  // so it wraps the fully assembled (compat-shimmed) API surface.
  expect(m.background.scripts).toEqual([
    "shims/storage-session-compat.js",
    "shims/runtime-compat.js",
    "__c2m_shim_bg.js",
    "config.js",
    "background.js",
  ]);
});

it("still prepends the bg shim when there are no shims/*.js entries", () => {
  const dir = makeExt(
    { manifest_version: 3, background: { scripts: ["bg.js"] } },
    { "bg.js": "" },
  );
  instrumentExtension(dir, "firefox-conv", 41999);
  expect(readManifest(dir).background.scripts).toEqual(["__c2m_shim_bg.js", "bg.js"]);
});
```

- [ ] **Step 2: Run the tests, confirm the first fails.**

Run: `cd e2e && pnpm test -- injector`
Expected: the new "inserts the bg shim AFTER ..." test FAILS (current code unshifts to index 0, producing `["__c2m_shim_bg.js", "shims/storage-session-compat.js", ...]`); the "still prepends" test PASSES (already true today).

- [ ] **Step 3: Implement the fix.** In `e2e/src/injector.ts`, replace the single line `m.background.scripts.unshift(BG_SHIM_NAME);` (inside the `else if (Array.isArray(m.background?.scripts))` branch, right after the `writeFileSync(join(dir, BG_SHIM_NAME), ...)` call) with:

```typescript
    // Insert the spy shim AFTER the converter's own shims/*.js compat layer (issue #6).
    // The compat shims add/patch API surface (stub namespaces, runtime polyfills); the spy
    // must wrap the fully assembled surface, so it goes after the last shims/*.js entry and
    // before the extension's own scripts. When there are no converter shims (unconverted
    // input, e.g. the chrome-orig side never reaches this branch), fall back to index 0.
    const scripts: string[] = m.background.scripts;
    let insertAt = 0;
    for (let i = 0; i < scripts.length; i++) {
      if (typeof scripts[i] === "string" && scripts[i].startsWith("shims/")) insertAt = i + 1;
    }
    scripts.splice(insertAt, 0, BG_SHIM_NAME);
```

- [ ] **Step 4: Run the tests, confirm both pass.**

Run: `cd e2e && pnpm test -- injector`
Expected: PASS (all injector tests).

- [ ] **Step 5: Typecheck.**

Run: `cd e2e && pnpm typecheck`
Expected: no errors.

- [ ] **Step 6: Commit.**

```bash
cd /Users/otsov/.superset/worktrees/1b400713-93e8-4131-93b2-695e1e5e7562/shim-order
git add e2e/src/injector.ts e2e/tests/injector.test.ts
git commit -m "fix(e2e): inject spy shim after converter compat shims (#6)"
```

---

### Task 2: Re-triage the corpus against the corrected ordering

**Files:**
- Modify: `e2e/corpus.json` (the `offscreen-gate` entry's `allowed_diffs` + `_notes`, and the OneNote entry's `allowed_diffs` + `_notes` insofar as the ordering fix changes them)
- Modify: `docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md` ("Known limits" issue-#6 reference)

**Interfaces:**
- Consumes: Task 1's injector fix; the converter (`cargo build --release`); the harness (`pnpm e2e`).
- Produces: an `allowed_diffs` set that reflects the real (post-ordering-fix) traces, with every remaining allowance justified by a trace-event reference in `_notes`.

**Background (why this matters):** before the fix, the spy shim ran before `shims/offscreen-polyfill.js`, so the polyfill's `offscreen.createDocument` / `getContexts` calls were untraced on the Firefox side and appeared Chrome-only. Both the `offscreen-gate` and OneNote entries carry `offscreen.*` and `runtime.getContexts*` allowances documented as "issue #6 tracing blind spot." After the fix, the spy wraps the polyfill surface, so those Firefox-side calls should now be traced. Some allowances become unnecessary (delete them); some divergences may turn out real (keep with a precise pattern and an accurate note); the issue #8 async-listener divergence in `offscreen-gate` is a permanent, intentional gate and must NOT be silently absorbed.

- [ ] **Step 1: Build the converter.**

Run: `cargo build --release`
Expected: builds clean.

- [ ] **Step 2: Confirm headless, then run the offscreen-gate entry.**

Run: `grep -n headless e2e/src/chromeDriver.ts e2e/src/firefoxDriver.ts` (expect `--headless=new` and `-headless`).
Run: `cd e2e && pnpm e2e --only offscreen-gate`
Read `e2e/results/offscreen-gate/trace-chrome.json` and `trace-firefox.json`.

- [ ] **Step 3: Re-triage offscreen-gate.** Compare the two traces. For each currently-allowed pattern (`offscreen.*`, `runtime.getContexts*`, `runtime.getURL#offscreen.html`, and the four issue-#8 patterns `runtime.sendMessage:resolve#hello async offscreen`, `runtime.sendMessage:resolve#null`, `storage.local.set#hello async offscreen`, `storage.local.set#UNDEFINED_RESPONSE`):
  - If the Firefox side now emits the same event and it MATCHES Chrome (so it is no longer a divergence), delete the pattern from `allowed_diffs` and delete its `_notes` entry.
  - If it is now traced on Firefox but genuinely diverges (e.g. the polyfill's `offscreen.createDocument` has different args or resolve shape than Chrome native), keep a tight pattern (prefer the `api#substring` form) and rewrite the `_notes` entry to describe the real, post-fix reason with a trace-event reference, not "tracing blind spot."
  - The four issue-#8 patterns are the permanent regression control for issue #8 (documented under `_issue_8_regression_control`): leave them and their notes intact unless the separate onmessage-compat work has already landed on main (it has not, in this branch), in which case leave them anyway and note that the onmessage-compat PR is responsible for removing them.
  - `runtime.getURL#offscreen.html`: the polyfill calls `rt.getURL(url)` itself; verify from the Firefox trace whether this extra call still appears now that the spy wraps the polyfill. Keep or delete based on the trace, updating the note.

  The rubric: harness artifact (tracing gap) that the fix removed -> delete the allowance; genuine by-design cross-engine difference -> keep a tight pattern with a real note; a converter bug the fix exposes -> do not allowlist, file/annotate and keep the entry failing. Do not add any allowance whose only justification is the pre-fix ordering.

- [ ] **Step 4: Verify offscreen-gate still PASSES** with the re-triaged allowances (the storage round-trip results `offscreenResult` / `offscreenCallbackResult` must be present and MATCHED on both sides; the issue-#8 `offscreenAsyncResult` divergence stays gated by the four control patterns).

Run: `cd e2e && pnpm e2e --only offscreen-gate`
Expected: `PASS  C2M Offscreen Gate`.

- [ ] **Step 5: Re-triage OneNote (quarantined, so it does not gate CI, but keep the record honest).**

Run: `cd e2e && pnpm e2e --only gojbdfnpnhogfdgjbigejoaolejmgdhk`
Read the Firefox trace. Apply the same rubric to OneNote's `offscreen.*` and `runtime.getContexts*` allowances (both documented as issue-#6 blind spots). Delete the ones the fix removes; rewrite the notes for any real remainder. The downstream issue-#8 cascade patterns (`runtime.error#JSON.parse: unexpected character`, `runtime.getManifest`, `net.fetch#onenote.com/strings`, `tabs.*`, `contextMenus.*`) are NOT caused by shim ordering; leave them and their `_offscreen_relay_root_cause` note untouched. Record before/after Firefox background-event counts in the entry `_notes`.

- [ ] **Step 6: Update the spec.** In `docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md`, find the "Known limits" text that references the issue-#6 shim-ordering blind spot and change it to state the spy shim is now injected after the converter compat shims, with the residual (if any) named precisely.

- [ ] **Step 7: Full verification.**

Run: `cargo test`
Run: `cd e2e && pnpm typecheck && pnpm test && pnpm e2e`
Expected: Rust tests pass; harness typecheck/unit pass; full corpus run shows `PASS LatexToCalc`, `PASS C2M Offscreen Gate`, and OneNote reported (quarantined).

- [ ] **Step 8: Commit.**

```bash
git add e2e/corpus.json docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md
git commit -m "e2e: re-triage allowed_diffs after spy-shim ordering fix (#6)"
```

---

### Task 3: Push and open the PR

- [ ] **Step 1: Push the branch.**

```bash
git push -u origin feature/e2e-shim-order
```

- [ ] **Step 2: Open the PR (do NOT merge).**

```bash
gh pr create --base main --head feature/e2e-shim-order \
  --title "fix(e2e): inject spy shim after converter compat shims (#6)" \
  --body "Fixes #6. Spy shim now inserts after the last shims/*.js entry in background.scripts, so converter-added API surface is traced instead of appearing Chrome-only. Re-triaged offscreen-gate and OneNote allowed_diffs against the corrected traces (deleted N ordering-artifact allowances, kept M genuine ones with trace-referenced notes). Full corpus green, both browsers headless."
```

Replace N and M with the actual counts from Task 2. Report the PR URL.

## Self-Review

- Spec coverage: issue #6 (spy after compat shims) -> Task 1; re-triage of allowances built on the bug -> Task 2; spec "Known limits" update -> Task 2 Step 6. Covered.
- Placeholder scan: N/M in the PR body are filled from real counts at Step 8; no other placeholders.
- Type consistency: `instrumentExtension` signature unchanged; `BG_SHIM_NAME` is the existing constant in `injector.ts`; `background.scripts` is `string[]`.
