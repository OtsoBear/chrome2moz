# E2E Differential Testing for Public Extensions

**Date:** 29.07.2026
**Status:** Draft — pending review

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
  snapshots/            # manifest of snapshot hashes (archives in GH Release assets)
  fixtures/             # standard fixture pages for <all_urls> extensions
  src/                  # TypeScript harness (pnpm)
  shim/                 # API spy shim injected into both builds
```

Per-extension pipeline:

1. **Fetch** — download pinned `.crx` from Google's public CRX endpoint, cached by version
2. **Instrument** — a single TS injector inserts the spy shim into both the original (unpacked) and the converted output post-conversion. One implementation for both sides guarantees symmetric instrumentation; no converter changes needed
3. **Launch** — Playwright drives Chromium (original), selenium-webdriver + geckodriver drives Firefox (converted, temporary add-on install). Both under `xvfb-run`, both routed through the replay proxy
4. **Probe** — identical stimulus sequence in both browsers (see Probes)
5. **Collect** — API traces + external observables from both sides
6. **Diff** — normalized structural diff; unallowed divergence = failure

### Three-way baseline run

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
| Spy shim | Plain JS, injected | Wraps `chrome.*`/`browser.*` (+ `fetch`, `WebSocket`), streams calls to telemetry server |
| Telemetry server | Node, localhost | Receives trace events from both browsers, tags by side |
| Record/replay proxy | mitmproxy (uv-managed) | Record mode for snapshot builds; replay mode in CI — identical bytes to both browsers |
| LLM visual judge | Claude API (optional) | Advisory screenshot-pair comparison |

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
- `allowed_diffs`: glob patterns over trace events for *expected* divergence (e.g. Firefox `tabGroups` no-op stub)
- `quarantined`: runs and reports but does not block CI
- LatexToCalc is the flagship first entry (local source, not CWS-fetched)
- OneNote Web Clipper (`gojbdfnpnhogfdgjbigejoaolejmgdhk`) is a permanent regression entry: it calls `management.uninstallSelf()` on Firefox detection, the exact failure class trace-diffing exists to catch (fixed in 255ca35)

## Instrumentation shim

- Transparent `Proxy` wrappers over every `chrome.*`/`browser.*` namespace the extension's permissions grant, plus `fetch`/`XMLHttpRequest`/`WebSocket`
- Records: API path, normalized args, result/error, context (background/content/popup)
- **Transparency requirement:** must not alter feature detection — wrap existing properties only, never add missing namespaces. Verified by a dedicated shim test suite
- Both sides: shim injected by the harness's TS injector after unpack/conversion (same code path → symmetric by construction)
- `alarms` wrapper supports fast-forward: harness command fires scheduled alarms immediately

## Web snapshots (differential fixtures)

- **Domain discovery:** union of `content_scripts.matches`, `host_permissions`, and a static scan of extension JS for URL/domain literals. Capped at 20 domains per extension (override via `extra_domains`)
- **Snapshot build** (manual script, on corpus change): Chromium visits each domain through mitmproxy in record mode; full flow archive (HTML + subresources) saved, compressed, uploaded as a GH Release asset; hash pinned in `e2e/snapshots/index.json`
- **CI:** replay-only. Proxy serves identical recorded bytes to both browsers. No live network in CI runs
- Wildcard/`<all_urls>` extensions: standard fixture set (forms, media, iframes, SPA) + their discovered literal domains
- Logged-out/bot-walled snapshots are fine — equivalence over whatever bytes we have

## Probes (auto-derived from manifest)

1. **Install/lifecycle** — first install, `onInstalled`, background boot
2. **Content scripts** — navigate to each snapshot/fixture page matching the match patterns
3. **Commands** — dispatch each keybinding chord (Chromium: CDP `Input.dispatchKeyEvent`; Firefox: WebDriver Actions). *Spike risk: synthetic events reaching browser-level command handlers — validate first*
4. **Popup / options** — open, wait for settle, screenshot, error check
5. **Kill/wake** — force-terminate background (Chrome: SW stop via CDP; Firefox: event page idle/termination), fire an event, diff wake behavior and restored state. Targets the #1 real conversion failure class
6. **Message round-trip** — content script ↔ background ping via `runtime.sendMessage`

v1.5: **monkey crawler** — generically click every button/input in popup and options pages in both browsers, diff resulting traces.

## Observables

- **Primary: API trace diff** (the rigor core) — every wrapped call, both sides
- Tab set + URLs after each probe
- DOM mutation summaries on fixture pages
- Clipboard readback (`navigator.clipboard.readText()` from a test page; permissions pre-granted; xvfb provides a real clipboard)
- Watched download directory
- Notifications via D-Bus mock daemon (deferred until a corpus extension needs it; shim-level `notifications.*` trace covers the API side meanwhile)
- Console errors, page errors, background errors
- Chrome-side V8 code coverage → per-extension "% code exercised" report

## Diff engine & pass criteria

- Trace normalization: strip timestamps, generated IDs (tab ids, request ids → stable placeholders), collapse benign reorderings of concurrent events
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

- Every PR + main, full corpus, single ubuntu job under `xvfb-run`
- Caches: CRX files (by id+version), snapshot archives (by hash), cargo build, pnpm store
- Job outputs: pass/fail per extension, trace-diff summaries, coverage %, vacuous-probe warnings, LLM judge report (if enabled)
- Corpus growth path: matrix-shard by extension when wall time exceeds ~15 min

## Known limits (accepted)

- Probe reach is the coverage ceiling; coverage % makes the ceiling visible per extension so fixtures grow where it's low
- Auth-gated behavior: out of scope
- Observer effect: shim could mask exotic feature-detection paths; transparency test suite shrinks this to near-zero
- Semantic correctness: equivalence only, by design

## Spikes (do first, in order)

1. Synthetic key chords triggering extension `commands` in both browsers
2. Firefox temporary add-on install + background error visibility via geckodriver
3. mitmproxy replay determinism with both browsers proxied (incl. TLS CA trust in both profiles)

## Phasing

- **v1 (PR-blocking):** fetch, instrument, launch, probes 1–4 + 6, trace diff, three-way baseline run + results table/badge, snapshots, clipboard readback (flagship LatexToCalc's core output is clipboard), coverage, CI
- **v1.1:** kill/wake probe (5), downloads observable
- **v1.5 (advisory first):** monkey crawler, LLM visual judge, structural-visual checks (zero-size/overflow/a11y-tree), perf ratio flags, D-Bus notifications
