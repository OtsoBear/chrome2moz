# chrome.offscreen Polyfill + runtime.getContexts Hardening

**Date:** 31.07.2026
**Status:** Draft — pending review
**Fixes:** OneNote-class breakage (issue #2): unguarded `chrome.offscreen` / `getContexts(OFFSCREEN_DOCUMENT)` calls throw on Firefox and cascade into total extension failure.

## Principle

Fix the API surface at runtime, not the source text. Static rewriting (the existing `OffscreenConverter`) cannot win against minified/aliased code; a polyfill catches every syntactic shape. Keep it minimal: one shim file, one state variable, no capability matrices.

## Why it works

Chrome extensions need offscreen documents because MV3 service workers have no DOM. chrome2moz converts backgrounds to **event pages, which have a DOM** — so an offscreen document can simply be a hidden iframe in the background page. The iframe is a real extension page: `runtime.sendMessage`/`onMessage` work natively, no bridging needed.

## The shim (`shims/offscreen-polyfill.js`, plain JS)

Injected first in `background.scripts`, before extension code. All parts guarded — define only what's missing, wrap only what exists.

1. **Enum backfill:** `runtime.ContextType.OFFSCREEN_DOCUMENT = "OFFSCREEN_DOCUMENT"` if missing. (OneNote's `[t.ContextType.OFFSCREEN_DOCUMENT]` is `[undefined]` today — this alone removes the throw trigger.)
2. **`runtime.getContexts` wrapper** (only if the API exists): strip `"OFFSCREEN_DOCUMENT"` and non-string entries from `filter.contextTypes`; if entries remain, call native with the sanitized filter, else skip the native call (empty `contextTypes` would match everything). If the polyfill iframe exists and the original filter asked for offscreen documents (or had no `contextTypes`), append one synthetic entry `{contextType: "OFFSCREEN_DOCUMENT", documentUrl, documentOrigin}` (respect a `documentUrls` filter when present).
3. **`chrome.offscreen`** (only if absent): state = one nullable iframe reference.
   - `createDocument({url})` → reject if iframe exists (mirrors Chrome's single-document rule); else append hidden iframe with `src = runtime.getURL(url)`, resolve on `load`, reject on `error`
   - `closeDocument()` → remove iframe, null the ref; reject if none (Chrome behavior)
   - `hasDocument()` → resolves `ref !== null`
   - `Reason` enum constants (string values matching Chrome's). `reasons` are accepted, not validated — an iframe either serves the use case or the extension fails visibly

## Converter wiring (Rust)

- New generator fn in `src/transformer/shims.rs` emitting the shim file (follows the existing 10 generators' pattern)
- Trigger: manifest `permissions` contains `"offscreen"` OR any packaged JS matches `chrome.offscreen`, `.ContextType.OFFSCREEN_DOCUMENT`, or `OFFSCREEN_DOCUMENT` (broad on purpose — injection is cheap and guarded; false positives are harmless no-ops)
- Injection uses the existing shim mechanism (file written to `shims/`, prepended to `background.scripts` ahead of extension code)
- Conversion report notes the injection; existing permission handling already covers the `offscreen` permission for Firefox
- `OffscreenConverter` (static path) untouched
- **Injection precondition:** the polyfill is only injected when the manifest declares a `background` section at all (`will_inject_offscreen_polyfill` = trigger AND `manifest.background.is_some()`). There is no `background.scripts` array to prepend a shim into otherwise, and no DOM-bearing event page for the polyfill's iframe to live in.

## Verification

1. **Rust unit tests:** trigger fires on permission, on source match, not on unrelated extensions; shim lands first in background scripts.
2. **New e2e corpus gate:** tiny testdata extension (`e2e/testdata/offscreen-extension/`) reproducing the OneNote pattern — aliased `getContexts(ContextType.OFFSCREEN_DOCUMENT)`, `createDocument({reasons:[DOM_PARSER]})`, message round-trip through the offscreen page, result written to storage. Added to `corpus.json` **non-quarantined**: the differential diff (Chrome real offscreen vs Firefox polyfill) is the permanent proof of behavioral equivalence.
3. **OneNote measurement (shipped reality):** re-running the corpus after the polyfill landed did not grow the Firefox trace toward Chrome's count — it went from 8 traced background events to 7, while Firefox objectively executed *strictly further* into the extension's code than before (Chrome traces 36 events this session). Raw event count is not a valid progress metric here: `getContexts`/`createDocument` now succeed but run through the polyfill's untraced surface (issue #6 — the e2e spy shim walks the API before the polyfill patches it), so several genuine steps of forward progress produce zero trace events, more than offsetting the one new event and one new terminal error this path adds. The original getContexts-throw crash (root cause #1) is gone; Firefox now reaches and loads the offscreen document, sends the first relay message, and crashes one layer deeper — on the async-listener/`sendResponse` incompatibility (issue #8: an `async function(msg, sender, sendResponse)` listener that calls `sendResponse(...)` synchronously without `return true`-ing; Chrome honors the manual `sendResponse` value, Firefox's `sendMessage` promise instead resolves to the listener's own implicit `undefined`, and OneNote's `JSON.parse(undefined)` on the receiving end throws). The offscreen-gate corpus entry's third probe (see below) proves this exact incompatibility deterministically and doubles as the gate's regression control — the stale `runtime.error#for runtime.getContexts` allowlist pattern (the pre-polyfill getContexts TypeError) is deleted along with `offscreen.*`, `runtime.getContexts*`, and `runtime.sendMessage#offscreen`, replaced by a re-triaged pattern set: `runtime.getContexts*` and `offscreen.*` stay (issue #6 tracing blind spot, functionally verified equivalent), `runtime.getURL#<ext>/offscreen.html` is added (the polyfill's own `getURL` call to resolve the iframe src), and the issue #8 cascade reintroduces the downstream chrome-only patterns (`runtime.error#JSON.parse: unexpected character`, `runtime.getManifest`, `net.fetch#onenote.com/strings`, `tabs.query#lastFocusedWindow`, `tabs.create#getting-started`, `tabs.create:resolve`, `tabs.onCreated:fired`, `tabs.onUpdated:fired#getting-started`, `contextMenus.*`) under a new root-cause note rather than the old one. Entry **stays quarantined** (fixture host = issue #5).
4. **Offscreen-gate probe 3 (callback form + close/recreate):** after the promise-form and async-listener-incompatibility probes, a third probe exercises `hasDocument(cb)` → `closeDocument(cb)` → recreate → parse round-trip again, all via the callback form rather than the returned Promise. Expected and observed: identical behavior on both sides (Chrome native callbacks vs polyfill callbacks) — MATCH, no new allowance required.

## Out of scope

- HTTPS fixture host for OneNote's content scripts (issue #5 / Plan 2)
- Service-worker-shaped Firefox backgrounds (converter always emits event pages)
- `USER_MEDIA`-style capability emulation, multi-document support, static-converter changes
- Offscreen document does not survive event-page suspension: the polyfill's iframe lives in the event page's DOM, so when Firefox suspends/discards the event page, the iframe (and its state) goes with it. The polyfill self-heals on the next call — `hasDocument()` reports `false` and a subsequent `createDocument()` re-creates it — which is correct for the request/response usage pattern in this extension, but wrong for a continuous-use case (e.g. `AUDIO_PLAYBACK`-style long-lived documents) where the extension itself must periodically call `hasDocument()`/re-`createDocument()`, or use an explicit `stop`/keepalive mechanism, to notice and recover from suspension.
- `window.close()` self-teardown from inside the iframe is a no-op: the polyfill has no listener on the iframe's own `window.close()` (extension pages generally can't close themselves this way), so an extension that calls `window.close()` from within its offscreen document expecting `hasDocument()` to flip to `false` will observe a stale `true` until the background calls `closeDocument()` itself. Mitigated in practice by the `isConnected` checks added to `createDocument`/`hasDocument`/`getContexts` synthesis (treats a detached frame as absent), but that only recovers once something else removes the iframe from the DOM — it does not make `window.close()` itself trigger teardown.
- Remaining known polyfill gaps not addressed in this fix wave — constant `contextId` across recreate cycles, the iframe `error` listener outliving its promise (a late error can silently destroy a live document), and `createDocument` resolving instead of rejecting for a missing/404'd page — are tracked in [issue #10](https://github.com/OtsoBear/chrome2moz/issues/10).
