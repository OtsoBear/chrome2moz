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
  snapshots/            # shipped (Plan 2, minimal cut): serve_addon.py, index.json (host -> file, sha256), per-entry HTML captures
  fixtures/             # standard fixture pages (basic.html, form.html), shipped, still used alongside web snapshots
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
3. **Launch** — Playwright drives Chromium (original), selenium-webdriver + geckodriver drives Firefox (converted, temporary add-on install). Headless is mandatory (user directive), not optional, and both browsers need an explicit flag rather than the driver's own headless switch: Chromium is launched with `headless: false` plus an explicit `--headless=new` arg (`chromeDriver.ts`) — Playwright's own `headless: true` alone never surfaced the extension's service worker; Firefox is launched with the `-headless` arg (`firefoxDriver.ts`). CI additionally wraps the whole differential-test step in `xvfb-run --auto-servernum` as defense-in-depth, not as what makes headless extension loading work (see CI). Both browsers are additionally routed through the mitmproxy serve-addon proxy for corpus entries that have a web snapshot, on top of the plain-fixture-server path (see Web snapshots)
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

**Implemented (Plan 2, minimal cut).** v1's two static fixture pages (`e2e/fixtures/basic.html`, `form.html`, served by `fixtureServer.ts`) are still used as the standard fixture set; web snapshots add real-hostname coverage on top for extensions whose `content_scripts.matches` target a specific domain, so their content scripts actually inject rather than being skipped for want of a matching origin.

- **Domain discovery** (`e2e/src/domains.ts`, `discoverDomains`): union of `content_scripts.matches` + `host_permissions` + optional `extra_domains` on the corpus entry. Capped at 20 domains per extension, `<all_urls>` and wildcard-only host patterns (`*://*/*`) dropped, `*.example.com` de-wildcarded to `example.com`. Static scan of extension JS for URL/domain literals is **not implemented**, out of scope for the minimal cut
- **Serve-addon** (`e2e/snapshots/serve_addon.py`): a mitmproxy addon that returns a stored HTML capture for the entry's snapshotted hosts, selected via `C2M_SNAPSHOT_ID`; any other document request to a snapshotted host gets an empty 204 (no live network at run time). Loopback hosts (`127.0.0.1`, `localhost`, `::1`) are always passed through untouched so the harness's own fixture/telemetry servers keep working while the browser is proxied
- **Snapshot storage** (`e2e/snapshots/index.json` + `e2e/snapshots/<entry-id>/<host>.html`): a stored HTML capture per host, sha256-recorded in the index. The minimal deliverable is a placeholder capture (valid `<head>`/`<body>`, a title) written by `e2e/src/snapshotBuild.ts` (`pnpm snapshot --only <id>`, manual/local, never run in CI), not a full recorded flow archive
- **Both drivers proxied:** `launchChrome`/`launchFirefox` take an optional `{ proxyServer }`; Chromium via Playwright `proxy: { server }` + `ignoreHTTPSErrors: true`; Firefox via `network.proxy.*` prefs + `acceptInsecureCerts`. TLS is accept-insecure only on both sides; **mitmproxy's CA is never imported into either browser's trust store** (see Spike 3)
- **Run wiring** (`run.ts`): for a corpus entry with a snapshot, starts `startSnapshotServer` (spawns `mitmdump`), launches both browsers through it, and passes the served URLs into `ProbeContext.snapshotUrls`; `contentProbe`/`pingProbe` navigate/prefer a snapshot URL that `content_scripts.matches` actually covers, alongside the standard fixtures
- **CI:** replay/serve-only. The addon reads stored files, no live network at corpus-run time (`.github/workflows/e2e.yml` installs `mitmproxy` via `uv tool install`)
- **Deferred to a follow-up:** live flow record/replay archives (full HTML + subresources captured from a real browsing session), GH-Release upload + hash-pinned distribution of snapshot archives, and the static-JS domain-literal scan. What's shipped is a stored single-page HTML capture served offline under the real hostname, sufficient for content-script injection coverage, not full-page/subresource fidelity
- Wildcard/`<all_urls>` extensions: standard fixture set (forms, media, iframes, SPA) + their discovered literal domains
- Logged-out/bot-walled snapshots are fine — equivalence over whatever bytes we have

## Probes (auto-derived from manifest)

1. **Install/lifecycle** — first install, `onInstalled`, background boot
2. **Content scripts** — navigate to each snapshot/fixture page matching the match patterns. Gated on `content_scripts.matches` actually covering the fixture origin (checked via a real WebExtension match-pattern matcher, not just "does `content_scripts` exist") — skips honestly with the unmatched patterns listed, instead of reporting `ran` when nothing could possibly inject
3. **Commands** — **spiked and found unsupported, not just a risk.** Spike C confirmed neither Chromium (Playwright CDP `Input.dispatchKeyEvent`) nor Firefox (Selenium WebDriver Actions) can trigger `chrome.commands.onCommand`/`browser.commands.onCommand` in either browser: DOM-level `keydown` diagnostics proved the correct key events do reach the page/content process, but `chrome.commands` shortcuts are matched against the browser's native global-accelerator table, which sits above the content-process input pipeline that synthetic CDP/WebDriver dispatch injects into — that table is architecturally unreachable from automation in both engines. This is a permanent limitation, not a timing or chord-mismatch bug (see `e2e/spikes/RESULTS.md` § Commands for the full root-cause trail, including the macOS MacCtrl→Command chord caveat that was checked and ruled out separately). The probe ships as an **unconditional** `skipped: dispatch-unsupported (spike C)` whenever an extension declares `commands` — no dispatch is attempted
4. **Popup / options** — open, wait for settle, screenshot, error check
5. **Kill/wake** (implemented, v1.1): force-terminate the background, fire an event, diff wake behavior and restored state. Chromium: CDP `ServiceWorker.stopAllWorkers` over a page-scoped CDP session (`ServiceWorker.*` is not exposed on a browser-level session). Firefox: an idle wait (~32s) past the default `extensions.background.idle.timeout` (30s, deliberately left untouched: lowering it at launch idle-kills the event page between ordinary probes because the spy's fetch polling does not count as extension activity, which silenced the ping relay in CI); no WebDriver kill command exists, but the idle-driven suspend is real and reboots correctly, confirmed 3/3 runs per `e2e/spikes/RESULTS.md` (Kill/Wake) via a per-boot marker that can only change on a genuine restart (used in the spike only; the gate persists only deterministic values so the diff stays clean). Wake: opening a second fixture tab fires a top-level `tabs.onUpdated` listener. State reaches the trace through the gate extension's content script reading `storage.local` (boot counter + last wake reason) on each fixture page, so both sides' reboot is diffed like any other event. Gate: the `killwake-gate` corpus entry (`e2e/testdata/killwake-extension/`), `allowed_diffs: []`. Targets the #1 real conversion failure class.
6. **Message round-trip** — content script ↔ background ping via `runtime.sendMessage`. Implemented via a poll-based command channel in the shim (`GET /cmd?side=…` polled from the injected content script; results posted back to `/cmdresult`), not a push channel. Same match-pattern gate as probe 2 — a ping can only round-trip through a content script the extension itself injects, so the probe skips honestly (`no content script on fixture page`) rather than opening pages and waiting on a relay that can never respond. If content_scripts covers the fixture origin but *neither* side's relay answers, that's reported skipped too (different note) — but if the two sides disagree (one answers, one doesn't), that asymmetry is treated as a real regression and the probe **fails**, not skips

v1.5: **monkey crawler** — generically click every button/input in popup and options pages in both browsers, diff resulting traces.

## Observables

- **Primary: API trace diff** (the rigor core, implemented) — every wrapped call, both sides
- Tab set + URLs after each probe (implemented, via the Tab-shape projection in `diff.ts`)
- DOM mutation summaries on fixture pages — not implemented
- **Clipboard readback (implemented, issue #4.3)** — shim-level trace observable: the spy shim wraps `navigator.clipboard.writeText`/`readText` (`e2e/shim/shim.js`) the same way it wraps `fetch`, recording `clipboard.writeText [text]` / `:resolve` / `:reject` and `clipboard.readText []` / `:resolve [value]` / `:reject`, so `readText:resolve`'s argument carries the actual read-back clipboard value through the normal trace diff. Both drivers pre-grant clipboard access so headless writes/reads resolve symmetrically instead of rejecting on user-activation grounds: Chromium via `ctx.grantPermissions(["clipboard-read", "clipboard-write"])` (`e2e/src/chromeDriver.ts`), Firefox via the `dom.events.asyncClipboard.*`/`dom.events.testing.asyncClipboard` test prefs (`e2e/src/firefoxDriver.ts`). Proven by the `clipboard-gate` corpus entry (`e2e/testdata/clipboard-extension`): a content script writes a fixed string and reads it back; on this platform both browsers resolved every call and produced identical `content`-context traces, so the entry needs no `allowed_diffs`.
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

- `.github/workflows/e2e.yml`, committed (not yet merged/pushed at time of writing). Every PR + main, full corpus, single ubuntu job. Headless flags are set in the drivers themselves (see Launch step for the exact `--headless=new`/`-headless` args); `xvfb-run --auto-servernum` around the differential-test step is defense-in-depth only. `astral-sh/setup-uv` + `uv tool install mitmproxy` runs before the differential e2e step so `mitmdump` is on PATH for the web-snapshot serve-addon (serve-only, no live network in CI)
- Caches implemented: CRX files (by corpus.json hash), cargo build (`Swatinem/rust-cache`), pnpm store (`actions/setup-node` pnpm cache). **Not implemented:** snapshot archive cache. The current snapshot corpus is small placeholder HTML checked into git, not yet large enough to need caching
- Job outputs implemented: unit test results, differential e2e pass/fail, `e2e/results/` uploaded as a build artifact (screenshots, notes). **Not implemented:** coverage %, LLM judge report — both deferred (Plan 3 / v1.5)
- Corpus growth path: matrix-shard by extension when wall time exceeds ~15 min — not yet needed (2-extension corpus), design intent unchanged

## Known limits (accepted)

- Probe reach is the coverage ceiling; a coverage % report would make the ceiling visible per extension (design target, not shipped — see V8 coverage in Phasing/Plan 3)
- Auth-gated behavior: out of scope
- Observer effect: shim could mask exotic feature-detection paths; transparency test suite shrinks this to near-zero
- Semantic correctness: equivalence only, by design
- Shim ordering: fixed. The spy shim is now injected immediately after the last `shims/*.js` entry in `background.scripts` (falling back to position 0 when there are none), so it wraps the fully assembled, compat-shimmed API surface instead of a pre-polyfill stub. Converter-added API surface (`chrome.offscreen.*`, the `runtime.getContexts` override) is now traced on the Firefox side; the corpus was re-triaged against the corrected traces ([chrome2moz#6](https://github.com/OtsoBear/chrome2moz/issues/6)). Residual, real (non-tracing) divergences remain and are allowlisted with trace-referenced notes: `runtime.getContexts:resolve`'s payload shape genuinely differs between the polyfill's emulation and native browser internals; `runtime.getURL` gets one extra firefox-only call because the polyfill resolves the offscreen iframe's URL itself. OneNote's deeper cascade (issue #8, an async `onMessage` listener paired with manual `sendResponse`) is unrelated to shim ordering and remains a separate, permanent divergence.

## Spikes (do first, in order)

All run and results recorded in `e2e/spikes/RESULTS.md`.

1. **Synthetic key chords triggering extension `commands` in both browsers — ran, negative result.** Neither Playwright CDP (`Input.dispatchKeyEvent`) nor Selenium WebDriver Actions can trigger `commands.onCommand` in Chromium or Firefox; both dispatch paths land in the content-process input pipeline but never pass through the browser's native global-accelerator table that `commands` shortcuts are matched against. Confirmed via DOM-level `keydown` diagnostics showing correct key delivery on both browsers, ruling out a timing/chord-mismatch explanation. See Probes § Commands and `e2e/spikes/RESULTS.md` § Commands for the full trail
2. **Firefox temporary add-on install + background error visibility via geckodriver — ran, worked as designed.** `driver.installAddon(xpi, true)` + `prefs.js` UUID lookup (see `e2e/spikes/RESULTS.md` § Firefox for the exact `prefs.js` escaping format the Task 8 parser has to handle: the `uuids` pref value is JSON-stringified and then re-escaped as a JS string literal, so captured text needs `\"` → `"` unescaping before `JSON.parse`)
3. **mitmproxy replay determinism with both browsers proxied, ran: PASS on both browsers, first run.** `mitmproxy 12.2.3` (via `uv tool install mitmproxy`); Chromium (Playwright proxy + `ignoreHTTPSErrors`) and Firefox (proxy prefs + `acceptInsecureCerts`) both load `https://example.test/` served entirely by the serve-addon (no live network), and the extension content script injects on the served page in both browsers. Working mitmdump invocation: `mitmdump -q -p <port> -s snapshots/serve_addon.py --set upstream_cert=false --set connection_strategy=lazy`; the two `--set` flags are required so mitmproxy generates its self-signed leaf cert without eagerly opening a real upstream connection (which hangs/fails for a fixture-only host). See `e2e/spikes/RESULTS.md` § Web snapshots / mitmproxy for the full trail (proxy config per browser, TLS approach, injection confirmation)

## Phasing

- **v1 (PR-blocking) — shipped:** fetch (CRX download + CRX3→ZIP parsing), instrument (shim injector), launch (headless Chromium + headless Firefox drivers), probes 1 (install), 2 (content, match-pattern gated), 4 (popup), 6 (ping, match-pattern gated); commands probe (3) ships as an honest unconditional skip per the spike finding, not real dispatch; trace normalizer + diff engine; two-way corpus runner (LatexToCalc local-source + OneNote Web Clipper from CWS); CI workflow (`.github/workflows/e2e.yml`, committed, not yet pushed/merged); web snapshots + domain discovery + mitmproxy serve proxy (Plan 2, minimal cut: stored single-page HTML capture served offline, not a full record/replay archive; see Web snapshots)
- **v1 (PR-blocking) — designed but deferred to follow-up issues, not shipped:** three-way baseline run + results table/badge (Plan 3), live snapshot record/replay archives + GH-Release hash-pinned distribution (Web snapshots follow-up), clipboard readback (Plan 3, flagship LatexToCalc's core output is clipboard, so this is a real coverage gap, not just a nice-to-have), V8 code coverage % (Plan 3)
- **v1.1:** kill/wake probe (5) implemented (see Probes). Not implemented: downloads observable, `alarms` fast-forward (harness backlog)
- **v1.5 (advisory first) — not implemented:** monkey crawler, LLM visual judge, structural-visual checks (zero-size/overflow/a11y-tree), perf ratio flags, D-Bus notifications
