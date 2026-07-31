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

## Verification

1. **Rust unit tests:** trigger fires on permission, on source match, not on unrelated extensions; shim lands first in background scripts.
2. **New e2e corpus gate:** tiny testdata extension (`e2e/testdata/offscreen-extension/`) reproducing the OneNote pattern — aliased `getContexts(ContextType.OFFSCREEN_DOCUMENT)`, `createDocument({reasons:[DOM_PARSER]})`, message round-trip through the offscreen page, result written to storage. Added to `corpus.json` **non-quarantined**: the differential diff (Chrome real offscreen vs Firefox polyfill) is the permanent proof of behavioral equivalence.
3. **OneNote measurement:** re-run corpus; expect the Firefox trace to grow from 8 events toward Chrome's 42 as the cascade unblocks. Delete now-dead allowlist patterns (`offscreen.*`, `runtime.getContexts*`, `runtime.error#for runtime.getContexts`, `runtime.sendMessage#offscreen`), re-triage the residue, update `_notes`. Entry **stays quarantined** (fixture host = issue #5).

## Out of scope

- HTTPS fixture host for OneNote's content scripts (issue #5 / Plan 2)
- Service-worker-shaped Firefox backgrounds (converter always emits event pages)
- `USER_MEDIA`-style capability emulation, multi-document support, static-converter changes
