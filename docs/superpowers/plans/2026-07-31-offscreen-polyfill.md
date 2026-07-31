# chrome.offscreen Polyfill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Runtime polyfill so converted extensions using `chrome.offscreen` / `getContexts(OFFSCREEN_DOCUMENT)` work on Firefox instead of dying (issue #2, OneNote-class).

**Architecture:** One guarded plain-JS shim (`shims/offscreen-polyfill.js`) injected before extension background code by the existing converter shim mechanism. Offscreen document = hidden iframe in the event-page DOM. One nullable iframe ref is the entire state. Verified by cargo unit tests, a new differential e2e corpus gate, and OneNote before/after measurement.

**Tech Stack:** Rust (existing `src/transformer/shims.rs` pattern), plain JS shim, existing e2e harness.

**Spec:** `docs/superpowers/specs/2026-07-31-offscreen-polyfill-design.md`

## Global Constraints

- Polyfill is guarded: define only what's missing, wrap only what exists; never break an extension where the APIs already work
- In Firefox `chrome` and `browser` are DISTINCT objects — patch both roots
- Shim must be injected BEFORE extension background scripts (existing shim ordering)
- Static `OffscreenConverter` untouched
- pnpm only in `e2e/`; browsers always headless
- Known tracing caveat (issue #6): the e2e spy shim walks APIs before this polyfill runs, so polyfill-created `offscreen.*` calls are untraced on the Firefox side; the corpus gate therefore asserts equivalence via the storage round-trip result, with a documented `offscreen.*` allowance

---

### Task 1: Rust — shim generator, trigger, wiring

**Files:**
- Modify: `src/transformer/shims.rs` (new generator fn, following the existing generators' pattern)
- Modify: the existing trigger/injection site (find where current shims are conditionally added — e.g. how the storage.session or declarativeContent shim is triggered — and add the offscreen polyfill the same way)
- Test: same-file `#[cfg(test)]` or `tests/` following repo convention

**Interfaces:**
- Produces: converted output containing `shims/offscreen-polyfill.js`, listed in `background.scripts` before extension scripts, whenever the trigger fires
- Trigger rule: manifest `permissions` contains `"offscreen"` OR any packaged `.js` file contains `chrome.offscreen` OR `OFFSCREEN_DOCUMENT`

**Shim file content (embed verbatim as the generator's output):**

```js
// chrome2moz: offscreen polyfill. Firefox has no chrome.offscreen and its
// getContexts rejects OFFSCREEN_DOCUMENT. The converted background is an event
// page WITH a DOM, so the offscreen document is emulated as a hidden iframe.
(() => {
  const roots = [];
  if (typeof browser !== "undefined") roots.push(browser);
  if (typeof chrome !== "undefined" && (typeof browser === "undefined" || chrome !== browser)) roots.push(chrome);
  const api = roots[0];
  const rt = api && api.runtime;
  if (!rt) return;

  let frame = null;
  let frameUrl = null;

  // Enum backfill: turns filter entries like [ContextType.OFFSCREEN_DOCUMENT]
  // from [undefined] into a real string, removing the Firefox validation throw.
  for (const r of roots) {
    if (!r.runtime) continue;
    r.runtime.ContextType = r.runtime.ContextType || {};
    if (!r.runtime.ContextType.OFFSCREEN_DOCUMENT) r.runtime.ContextType.OFFSCREEN_DOCUMENT = "OFFSCREEN_DOCUMENT";
  }

  if (typeof rt.getContexts === "function") {
    const origGetContexts = rt.getContexts.bind(rt);
    const wrapped = async function (filter) {
      const f = filter || {};
      const hasTypes = Array.isArray(f.contextTypes);
      const wantsOffscreen = !hasTypes || f.contextTypes.some((t) => t === "OFFSCREEN_DOCUMENT" || t === undefined);
      let results = [];
      if (!hasTypes) {
        results = await origGetContexts(f);
      } else {
        const rest = f.contextTypes.filter((t) => typeof t === "string" && t !== "OFFSCREEN_DOCUMENT");
        // Empty contextTypes matches everything in the native API — never pass it through.
        if (rest.length > 0) results = await origGetContexts({ ...f, contextTypes: rest });
      }
      if (wantsOffscreen && frame && frameUrl) {
        const urlOk = !Array.isArray(f.documentUrls) || f.documentUrls.includes(frameUrl);
        if (urlOk) {
          results = results.concat([{
            contextType: "OFFSCREEN_DOCUMENT",
            documentUrl: frameUrl,
            documentOrigin: new URL(frameUrl).origin,
            contextId: "c2m-offscreen-0",
            frameId: 0, tabId: -1, windowId: -1, incognito: false,
          }]);
        }
      }
      return results;
    };
    for (const r of roots) { if (r.runtime && typeof r.runtime.getContexts === "function") r.runtime.getContexts = wrapped; }
  }

  if (!api.offscreen) {
    const offscreen = {
      Reason: {
        AUDIO_PLAYBACK: "AUDIO_PLAYBACK", BATTERY_STATUS: "BATTERY_STATUS", BLOBS: "BLOBS",
        CLIPBOARD: "CLIPBOARD", DISPLAY_MEDIA: "DISPLAY_MEDIA", DOM_PARSER: "DOM_PARSER",
        DOM_SCRAPING: "DOM_SCRAPING", GEOLOCATION: "GEOLOCATION", IFRAME_SCRIPTING: "IFRAME_SCRIPTING",
        LOCAL_STORAGE: "LOCAL_STORAGE", MATCH_MEDIA: "MATCH_MEDIA", TESTING: "TESTING",
        USER_MEDIA: "USER_MEDIA", WEB_RTC: "WEB_RTC", WORKERS: "WORKERS",
      },
      createDocument(opts) {
        return new Promise((resolve, reject) => {
          if (frame) { reject(new Error("Only a single offscreen document may be created.")); return; }
          const url = opts && opts.url;
          if (typeof url !== "string") { reject(new Error("offscreen.createDocument: url is required.")); return; }
          const el = document.createElement("iframe");
          el.style.display = "none";
          el.src = rt.getURL(url);
          el.addEventListener("load", () => resolve(), { once: true });
          el.addEventListener("error", () => {
            el.remove(); frame = null; frameUrl = null;
            reject(new Error("offscreen.createDocument: failed to load " + url));
          }, { once: true });
          frame = el;
          frameUrl = el.src;
          (document.body || document.documentElement).appendChild(el);
        });
      },
      closeDocument() {
        if (!frame) return Promise.reject(new Error("No current offscreen document."));
        frame.remove(); frame = null; frameUrl = null;
        return Promise.resolve();
      },
      hasDocument() { return Promise.resolve(frame !== null); },
    };
    for (const r of roots) { if (!r.offscreen) r.offscreen = offscreen; }
  }
})();
```

- [ ] **Step 1: Read the existing shim pattern** — how one existing conditional shim (e.g. storage.session) is generated, triggered, written to `shims/`, and prepended to `background.scripts`; note the exact functions/sites to extend
- [ ] **Step 2: Write failing tests** — following the repo's test conventions, three trigger tests (fires on `offscreen` permission; fires on a JS file containing `chrome.offscreen` or `OFFSCREEN_DOCUMENT`; does NOT fire on an unrelated extension) and one ordering test (polyfill path appears in `background.scripts` before the extension's own script)
- [ ] **Step 3: Run tests, confirm they fail**
- [ ] **Step 4: Implement** — generator fn embedding the shim JS above verbatim; trigger + wiring per the existing pattern; conversion-report note ("injected offscreen polyfill")
- [ ] **Step 5: `cargo test` green, `cargo clippy` clean**
- [ ] **Step 6: Commit** — `feat: runtime offscreen polyfill for Firefox conversions`

---

### Task 2: e2e corpus gate — offscreen test extension

**Files:**
- Create: `e2e/testdata/offscreen-extension/manifest.json`, `background.js`, `offscreen.html`, `offscreen.js`
- Modify: `e2e/corpus.json` (new non-quarantined entry)

**Interfaces:**
- Consumes: converter with Task 1's polyfill; existing harness end-to-end
- Produces: a permanently-gating corpus entry proving offscreen behavior equivalence

**The extension (reproduces the OneNote pattern, aliased):**

`manifest.json`:
```json
{
  "manifest_version": 3,
  "name": "C2M Offscreen Gate",
  "version": "1.0",
  "background": { "service_worker": "background.js" },
  "permissions": ["offscreen", "storage"]
}
```

`background.js` (aliased exactly like minified code, no feature detection — the failure shape under test):
```js
async function viaOffscreen(payload) {
  const t = chrome.runtime, r = chrome.offscreen;
  const url = t.getURL("offscreen.html");
  const existing = await t.getContexts({ contextTypes: [t.ContextType.OFFSCREEN_DOCUMENT], documentUrls: [url] });
  if (existing.length === 0) {
    await r.createDocument({ url: "offscreen.html", reasons: [r.Reason.DOM_PARSER], justification: "parse" });
  }
  return chrome.runtime.sendMessage({ kind: "parse", payload });
}

chrome.runtime.onInstalled.addListener(() => {
  viaOffscreen("<p id='x'>hello offscreen</p>")
    .then((result) => chrome.storage.local.set({ offscreenResult: result }))
    .catch((e) => chrome.storage.local.set({ offscreenError: String(e) }));
});
```

`offscreen.html`:
```html
<!DOCTYPE html><html><body><script src="offscreen.js"></script></body></html>
```

`offscreen.js`:
```js
chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg && msg.kind === "parse") {
    const doc = new DOMParser().parseFromString(msg.payload, "text/html");
    sendResponse({ text: doc.getElementById("x").textContent });
    return true;
  }
});
```

Corpus entry (append to `e2e/corpus.json` extensions):
```json
{
  "id": "offscreen-gate",
  "name": "C2M Offscreen Gate",
  "source": "local:testdata/offscreen-extension",
  "allowed_diffs": ["offscreen.*"],
  "quarantined": false,
  "_notes": {
    "offscreen.*": "Firefox-side offscreen.* calls hit the converter's polyfill, which is injected AFTER the spy shim walks the API surface (issue #6) — so they are untraced on that side and appear chrome-only. Behavioral equivalence is asserted by the storage.local.set offscreenResult write, which must match on both sides."
  }
}
```

- [ ] **Step 1: Create the four extension files verbatim**
- [ ] **Step 2: Add the corpus entry verbatim**
- [ ] **Step 3: Run `pnpm e2e --only offscreen-gate` (headless)** — expect PASS with the storage write `{"offscreenResult":{"text":"hello offscreen"}}` present and MATCHED in both traces (check `e2e/results/offscreen-gate/trace-*.json`; a run where both sides store `offscreenError` is a real failure of the polyfill even if traces match — inspect and fix via Task 1, don't allowlist)
- [ ] **Step 4: Full `pnpm e2e` + `pnpm test` still green**
- [ ] **Step 5: Commit** — `test(e2e): offscreen polyfill differential gate`

---

### Task 3: OneNote re-triage, measurement, PR

**Files:**
- Modify: `e2e/corpus.json` (OneNote allowlist), spec's Verification section if numbers warrant a note

**Interfaces:**
- Consumes: everything above

- [ ] **Step 1: Baseline note** — record OneNote's current Firefox event count (8) from the last run artifacts
- [ ] **Step 2: Delete the cascade patterns** from OneNote's allowed_diffs: `offscreen.*`, `runtime.getContexts*`, `runtime.error#for runtime.getContexts`, `runtime.sendMessage#offscreen`; re-run `pnpm e2e --only gojbdfnpnhogfdgjbigejoaolejmgdhk`
- [ ] **Step 3: Triage the new state** — expected: Firefox trace grows substantially (toward Chrome's 42) as post-install setup now runs. Re-derive any genuinely-remaining divergences with tight patterns + accurate `_notes` (the rubric from the harness applies: harness artifact → fix harness; by-design → tight pattern; converter bug → issue). Add an `offscreen.*` allowance ONLY for the issue-#6 tracing blind spot, with the same note as the gate entry. Entry stays `quarantined: true`
- [ ] **Step 4: Record measurement** — before/after Firefox event counts + deleted-pattern list in the corpus `_notes` and as a comment on issue #2 (`gh issue comment 2`)
- [ ] **Step 5: Full verification** — `cargo test`, `cd e2e && pnpm typecheck && pnpm test`, full `pnpm e2e` headless (LatexToCalc PASS, offscreen-gate PASS, OneNote reported)
- [ ] **Step 6: Commit, push branch, open PR** against main titled `feat: runtime offscreen polyfill (fixes OneNote-class breakage)` referencing issues #2 and #6; report the PR URL
