# E2E Differential Testing for Public Extensions

**Date:** 29.07.2026
**Status:** Implemented (v1) — updated post-implementation to match the shipped harness; see "Implementation deviations" callouts inline

## Goal

Automatically verify that any public Chrome extension, after conversion by chrome2moz, behaves **equivalently** in Firefox to the original in Chrome. Target: catch ~99% of conversion-induced behavioral differences with zero per-extension test code.

## Core principle: equivalence, not correctness

We never assert what an extension *should* do. We run the original in Chromium and the converted build in Firefox under identical, hermetic conditions, and diff what happened. A broken fixture page that fails identically in both browsers is a valid pass — divergence is the only failure signal. This is what makes the system fully automatic.

## Non-goals

- Extension *correctness* (if both browsers store the same wrong value, we pass)
- Anything behind auth/accounts/paywalls — no credentials, no coverage, permanently out of scope
- Native messaging hosts — extensions using them get `coverage_flags: ["partial"]` in the corpus
- Cross-OS matrices (Linux CI only for v1)

## Architecture

```
e2e/
  corpus.json           # pinned extension list
  snapshots/            # NOT YET CREATED — Plan 2. Target: manifest of snapshot hashes (archives in GH Release assets)
  fixtures/             # standard fixture pages (basic.html, form.html) — shipped, static only, no snapshot corpus yet
  src/                  # TypeScript harness (pnpm) — shipped
  shim/                 # API spy shim injected into both builds — shipped
  spikes/               # spike scripts + RESULTS.md — shipped
  testdata/             # local fixture extensions used by spikes/unit tests — shipped
  tests/                # unit tests — shipped
  results/              # per-run output (screenshots, notes), uploaded as CI artifact — shipped
```

Per-extension pipeline:

1. **Fetch** — download pinned `.crx` from Google's public CRX endpoint, cached by version
2. **Instrument** — a single TS injector inserts the spy shim into both the original (unpacked) and the converted output post-conversion. One implementation for both sides guarantees symmetric instrumentation; no converter changes needed
3. **Launch** — Playwright drives Chromium (original), selenium-webdriver + geckodriver drives Firefox (converted, temporary add-on install). Headless is mandatory (user directive), not optional, and both browsers need an explicit flag rather than the driver's own headless switch: Chromium is launched with `headless: false` plus an explicit `--headless=new` arg (`chromeDriver.ts`) — Playwright's own `headless: true` alone never surfaced the extension's service worker; Firefox is launched with the `-headless` arg (`firefoxDriver.ts`). CI additionally wraps the whole differential-test step in `xvfb-run --auto-servernum` as defense-in-depth, not as what makes headless extension loading work (see CI). *Design target: both routed through the replay proxy — not shipped in v1, static fixture pages are served directly instead (see Web snapshots)*
4. **Probe** — identical stimulus sequence in both browsers (see Probes)
5. **Collect** — API traces + external observables from both sides
6. **Diff** — normalized structural diff; unallowed divergence = failure

### Three-way baseline run

**Not implemented in v1 — deferred to Plan 3 (follow-up issue filed).** `run.ts` currently runs the two-way comparison (original-in-Chromium vs converted-in-Firefox) only; there is no third "original-in-Firefox" leg, no per-extension `worked-anyway`/`fixed-by-conversion`/`broken-either-way` classification, and no generated README table/badge. The design below is the target for Plan 3.

Each extension also runs a third config: the **unconverted original loaded directly in Firefox**, same probes. Per-extension verdict:

- `worked-anyway` — original passes in Firefox unmodified (conversion not needed for this one)
- `fixed-by-conversion` — original diverges/breaks in Firefox, converted build passes
- `broken-either-way` — converted build still diverges (converter gap → issue to fix)

Results feed a generated table in the README plus a badge ("N extensions verified equivalent, M fixed by conversion"). This quantifies the converter's value per extension and shows which corpus entries actually exercise it.

### Components

| Component | Tech | Role |
|---|---|---|
| Harness | TypeScript, pnpm | Orchestration, diffing, reporting |
| Chromium driver | Playwright | Load original, dispatch probes, CDP access (coverage, targets) |
| Firefox driver | selenium-webdriver + geckodriver | Load converted (temporary add-on), dispatch probes |
| Spy shim | Plain JS, injected | Wraps `chrome.*`/`browser.*` + `fetch` via direct property reassignment (not `Proxy`), streams calls to telemetry server. `WebSocket`/`XMLHttpRequest` wrapping not implemented (backlog, issue #5) |
| Telemetry server | Node, localhost | Receives trace events from both browsers, tags by side |
| Record/replay proxy | mitmproxy (uv-managed) | **Not implemented (Plan 2).** Design target: record mode for snapshot builds, replay mode in CI. v1 uses only the static fixture pages in `e2e/fixtures/`, served directly, no proxy in the loop |
| LLM visual judge | Claude API (optional) | **Not implemented (v1.5, unstarted).** Advisory screenshot-pair comparison — see Phasing |

## Corpus

`e2e/corpus.json`, one entry per extension:

```json
{
  "id": "cws-extension-id",
  "version": "1.4.2",
  "name": "Example",
  "coverage_flags": [],
  "allowed_diffs": ["tabGroups.*"],
  "extra_domains": [],
  "quarantined": false
}
```

- Pinned versions → deterministic. Growing the corpus = adding an entry.
- `allowed_diffs`: glob patterns over trace events for *expected* divergence (e.g. Firefox `tabGroups` no-op stub). Supports an `api-glob#substring` qualifier form: the pattern only allows a divergence when the glob matches the event's API name **and** the event's normalized args string contains the text after `#` (e.g. `runtime.sendMessage#offscreen`, `net.fetch#onenote.com/strings`). This lets broad-surface APIs (`runtime.error`, `net.fetch`, `runtime.sendMessage`) be pinned to the one specific call site a triage actually investigated, instead of allowlisting every call to that API for the whole corpus entry. Plain entries without `#` keep the old api-only behavior
- `quarantined`: runs and reports but does not block CI
- LatexToCalc is the flagship first entry (local source, not CWS-fetched)
- OneNote Web Clipper (`gojbdfnpnhogfdgjbigejoaolejmgdhk`) is a permanent regression entry for the `management.uninstallSelf()`-on-Firefox-detection failure class (converter fix: commit `255ca35`). **Caveat found in Task 12:** the currently pinned version (3.11.2) does not declare the `management` permission at all and never calls `uninstallSelf` in either trace — the `management.uninstallSelf*` allowlist pattern is currently inert (matches nothing), kept only so a future re-pin that reintroduces the behavior would still be caught. The corpus entry's real, currently-exercised value is different: it caught a genuine chrome2moz-unrelated bug in the extension's own source (an unguarded `chrome.offscreen`/`runtime.getContexts` call with no feature-detection, which cascades into ~10 allowlisted divergence patterns — see `e2e/corpus.json`'s `_offscreen_cascade_root_cause` note and issue #2). Separately, this entry is a thin assertion overall (2/18 of the extension's API surface exercised) until a fixture host matching its actual `content_scripts` patterns (`onenote.officeapps.live.com`) exists — tracked in the harness-backlog follow-up issue

## Instrumentation shim

- Every `chrome.*`/`browser.*` namespace the extension's permissions grant is walked recursively and each function/event property is replaced in place (`ns[key] = wrapped`), not wrapped in a `Proxy` object. `fetch` is wrapped the same way, by reassigning `globalThis.fetch` to a function that records then delegates to the captured original. **Not implemented:** `XMLHttpRequest` and `WebSocket` wrapping — only `fetch` is covered on the network side today (backlog, issue #5)
- Records: API path, normalized args, result/error, context (background/content/popup)
- **Transparency requirement:** must not alter feature detection — wrap existing properties only, never add missing namespaces. Verified by a dedicated shim test suite
- Both sides: shim injected by the harness's TS injector after unpack/conversion (same code path → symmetric by construction)
- Ping/command channel: the shim polls `GET /cmd?side=…` from the injected content script and posts outcomes to `/cmdresult`; the pingProbe uses this today. **Not implemented:** `alarms` fast-forward (harness command to fire scheduled alarms immediately) — deferred to backlog, tracked in the wrap-up harness-backlog issue, not shipped in v1

## Web snapshots (differential fixtures)

**Not implemented in v1 — deferred to Plan 2 (follow-up issue filed).** v1 ships two static fixture pages (`e2e/fixtures/basic.html`, `form.html`) served by a plain local static server (`fixtureServer.ts`), with probes gated on whether an extension's `content_scripts.matches` actually cover the fixture origin (see Probes). No domain discovery, no mitmproxy record/replay, no per-extension snapshot archives. The design below is the target for Plan 2.

- **Domain discovery:** union of `content_scripts.matches`, `host_permissions`, and a static scan of extension JS for URL/domain literals. Capped at 20 domains per extension (override via `extra_domains`)
- **Snapshot build** (manual script, on corpus change): Chromium visits each domain through mitmproxy in record mode; full flow archive (HTML + subresources) saved, compressed, uploaded as a GH Release asset; hash pinned in `e2e/snapshots/index.json`
- **CI:** replay-only. Proxy serves identical recorded bytes to both browsers. No live network in CI runs
- Wildcard/`<all_urls>` extensions: standard fixture set (forms, media, iframes, SPA) + their discovered literal domains
- Logged-out/bot-walled snapshots are fine — equivalence over whatever bytes we have

## Probes (auto-derived from manifest)

1. **Install/lifecycle** — first install, `onInstalled`, background boot
2. **Content scripts** — navigate to each snapshot/fixture page matching the match patterns. Gated on `content_scripts.matches` actually covering the fixture origin (checked via a real WebExtension match-pattern matcher, not just "does `content_scripts` exist") — skips honestly with the unmatched patterns listed, instead of reporting `ran` when nothing could possibly inject
3. **Commands** — **spiked and found unsupported, not just a risk.** Spike C confirmed neither Chromium (Playwright CDP `Input.dispatchKeyEvent`) nor Firefox (Selenium WebDriver Actions) can trigger `chrome.commands.onCommand`/`browser.commands.onCommand` in either browser: DOM-level `keydown` diagnostics proved the correct key events do reach the page/content process, but `chrome.commands` shortcuts are matched against the browser's native global-accelerator table, which sits above the content-process input pipeline that synthetic CDP/WebDriver dispatch injects into — that table is architecturally unreachable from automation in both engines. This is a permanent limitation, not a timing or chord-mismatch bug (see `e2e/spikes/RESULTS.md` § Commands for the full root-cause trail, including the macOS MacCtrl→Command chord caveat that was checked and ruled out separately). The probe ships as an **unconditional** `skipped: dispatch-unsupported (spike C)` whenever an extension declares `commands` — no dispatch is attempted
4. **Popup / options** — open, wait for settle, screenshot, error check
5. **Kill/wake** — force-terminate background (Chrome: SW stop via CDP; Firefox: event page idle/termination), fire an event, diff wake behavior and restored state. Targets the #1 real conversion failure class. **Not yet implemented** (v1.1, see Phasing)
6. **Message round-trip** — content script ↔ background ping via `runtime.sendMessage`. Implemented via a poll-based command channel in the shim (`GET /cmd?side=…` polled from the injected content script; results posted back to `/cmdresult`), not a push channel. Same match-pattern gate as probe 2 — a ping can only round-trip through a content script the extension itself injects, so the probe skips honestly (`no content script on fixture page`) rather than opening pages and waiting on a relay that can never respond. If content_scripts covers the fixture origin but *neither* side's relay answers, that's reported skipped too (different note) — but if the two sides disagree (one answers, one doesn't), that asymmetry is treated as a real regression and the probe **fails**, not skips

v1.5: **monkey crawler** — generically click every button/input in popup and options pages in both browsers, diff resulting traces.

## Observables

- **Primary: API trace diff** (the rigor core, implemented) — every wrapped call, both sides
- Tab set + URLs after each probe (implemented, via the Tab-shape projection in `diff.ts`)
- DOM mutation summaries on fixture pages — not implemented
- Clipboard readback (`navigator.clipboard.readText()` from a test page; permissions pre-granted; xvfb provides a real clipboard) — **not implemented, deferred to Plan 3** (follow-up issue filed)
- Watched download directory — not implemented
- Notifications via D-Bus mock daemon (deferred until a corpus extension needs it; shim-level `notifications.*` trace covers the API side meanwhile) — not implemented, `notifications.*` trace-level coverage also not yet exercised by the corpus
- Page errors, background errors — implemented as `runtime.error` trace events (shim's global `error`/`unhandledrejection` listeners feed into the same trace diff, not a separate observable pipeline). `console.error`/`console.*` calls specifically are not captured
- Chrome-side V8 code coverage → per-extension "% code exercised" report — **not implemented, deferred to Plan 3** (follow-up issue filed)

## Diff engine & pass criteria

- Trace normalization: strip timestamps, generated IDs (tab ids, request ids → stable placeholders), collapse benign reorderings of concurrent events. Implementation specifics worth calling out:
  - Positional ids in `tabs.on*:fired` listener payloads (`onUpdated(tabId, changeInfo, tab)`, `onRemoved(tabId, removeInfo)`) are remapped by array position, not by an object key match, since the id there isn't under a named key
  - WebExtensions spec sentinel ids (`frameId: 0` for the top frame, `tabId: -1` for "no tab") are left as literal `0`/`-1` rather than remapped through the per-trace id map — they're directly comparable across browsers as-is, and remapping them would consume an id-map slot and skew every real id after them (n ≤ 0 is treated as a sentinel, not remapped)
  - Numeric epoch scrubbing only fires on an unambiguous case: an exact 13-digit integer (ms-epoch), or any fractional number with 10–13 integer digits — a bare 10-digit integer could be either a Chrome tab id or a seconds-epoch timestamp, so it's left alone; no legitimate id/counter is ever fractional, so a fractional value in that magnitude range is safely a timestamp
  - Any Tab-shaped object (`id`+`windowId`+`index`+boolean `active` present) is projected down to `{url, title, status, index, active}` before comparison, collapsing the entire class of Chrome-vs-Firefox native `tabs.Tab` shape differences (Chrome-only `frozen`/`groupId`/`selected`; Firefox-only `attention`/`hidden`/`isArticle`/`isInReaderMode`/`sharingState`/`successorTabId`/`cookieStoreId`) instead of allowlisting each one
  - `temporary` is stripped, but only from `runtime.onInstalled:fired`'s first-arg object — it's harness noise (the Firefox driver always installs via `installAddon(path, true)`, since temporary install is the only way to load an unsigned build), not stripped anywhere else a key named `temporary` might appear
- Structural diff of normalized Chrome trace vs Firefox trace, then external observables
- **Fail:** any divergence not matched by `allowed_diffs`; any Firefox-only error; any missing external effect
- **Vacuity guard:** a probe that exercises 0% extension code on both sides is marked `vacuous` and warned, never silently passed
- A failure must reproduce on one automatic retry to fail the build (flake control)

## LLM visual judge (optional, advisory)

- Screenshot pairs (Chrome original vs Firefox converted): popup, options, each fixture page post-probe
- Claude call: "Same UI intent? Anything visually broken in the second image?" → advisory verdict + reasoning
- Posted as a PR comment section; **never blocks CI**
- Enabled by `--llm-judge`; key from AWS Secrets Manager (`otso-personal-anthropic-api-key`); large corpus runs use the Message Batches API

## CI

- `.github/workflows/e2e.yml`, committed (not yet merged/pushed at time of writing). Every PR + main, full corpus, single ubuntu job. Headless flags are set in the drivers themselves (see Launch step for the exact `--headless=new`/`-headless` args); `xvfb-run --auto-servernum` around the differential-test step is defense-in-depth only
- Caches implemented: CRX files (by corpus.json hash), cargo build (`Swatinem/rust-cache`), pnpm store (`actions/setup-node` pnpm cache). **Not implemented:** snapshot archive cache — no snapshot corpus exists yet (Plan 2)
- Job outputs implemented: unit test results, differential e2e pass/fail, `e2e/results/` uploaded as a build artifact (screenshots, notes). **Not implemented:** coverage %, LLM judge report — both deferred (Plan 3 / v1.5)
- Corpus growth path: matrix-shard by extension when wall time exceeds ~15 min — not yet needed (2-extension corpus), design intent unchanged

## Known limits (accepted)

- Probe reach is the coverage ceiling; a coverage % report would make the ceiling visible per extension (design target, not shipped — see V8 coverage in Phasing/Plan 3)
- Auth-gated behavior: out of scope
- Observer effect: shim could mask exotic feature-detection paths; transparency test suite shrinks this to near-zero
- Semantic correctness: equivalence only, by design

## Spikes (do first, in order)

All run and results recorded in `e2e/spikes/RESULTS.md`.

1. **Synthetic key chords triggering extension `commands` in both browsers — ran, negative result.** Neither Playwright CDP (`Input.dispatchKeyEvent`) nor Selenium WebDriver Actions can trigger `commands.onCommand` in Chromium or Firefox; both dispatch paths land in the content-process input pipeline but never pass through the browser's native global-accelerator table that `commands` shortcuts are matched against. Confirmed via DOM-level `keydown` diagnostics showing correct key delivery on both browsers, ruling out a timing/chord-mismatch explanation. See Probes § Commands and `e2e/spikes/RESULTS.md` § Commands for the full trail
2. **Firefox temporary add-on install + background error visibility via geckodriver — ran, worked as designed.** `driver.installAddon(xpi, true)` + `prefs.js` UUID lookup (see `e2e/spikes/RESULTS.md` § Firefox for the exact `prefs.js` escaping format the Task 8 parser has to handle: the `uuids` pref value is JSON-stringified and then re-escaped as a JS string literal, so captured text needs `\"` → `"` unescaping before `JSON.parse`)
3. **mitmproxy replay determinism with both browsers proxied — not run.** Superseded: v1 shipped without the record/replay proxy (static fixture pages only); this spike is deferred along with the rest of Web snapshots to Plan 2

## Phasing

- **v1 (PR-blocking) — shipped:** fetch (CRX download + CRX3→ZIP parsing), instrument (shim injector), launch (headless Chromium + headless Firefox drivers), probes 1 (install), 2 (content, match-pattern gated), 4 (popup), 6 (ping, match-pattern gated); commands probe (3) ships as an honest unconditional skip per the spike finding, not real dispatch; trace normalizer + diff engine; two-way corpus runner (LatexToCalc local-source + OneNote Web Clipper from CWS); CI workflow (`.github/workflows/e2e.yml`, committed, not yet pushed/merged)
- **v1 (PR-blocking) — designed but deferred to follow-up issues, not shipped:** three-way baseline run + results table/badge (Plan 3), web snapshots + domain discovery + mitmproxy record/replay (Plan 2), clipboard readback (Plan 3 — flagship LatexToCalc's core output is clipboard, so this is a real coverage gap, not just a nice-to-have), V8 code coverage % (Plan 3)
- **v1.1 — not implemented:** kill/wake probe (5), downloads observable, `alarms` fast-forward (harness backlog)
- **v1.5 (advisory first) — not implemented:** monkey crawler, LLM visual judge, structural-visual checks (zero-size/overflow/a11y-tree), perf ratio flags, D-Bus notifications
