# Spike Results

## Chromium

Ran `pnpm exec tsx spikes/spike-chromium.ts` from `e2e/` with `playwright@1.54.0` (Chromium browser bundled by Playwright), on macOS.

**Verdict: all three things worked, first try, no deviations from the brief's script.**

Output (two consecutive runs, for reproducibility):

```
extension id: fmigiiinnecobgblbggpecffcfakbndh
telemetry hits: [ '/from-bg', '/from-content', '/favicon.ico' ]
```

Observed behaviors:

- `chromium.launchPersistentContext(...)` with `--disable-extensions-except=<dir>` and `--load-extension=<dir>` loads the unpacked MV3 extension without any headless-flag workaround. `headless: false` was used as given in the brief (MV3 extensions generally require a "headed" or `--headless=new` context; not tested here since the brief specifies `headless: false` and a visible window is expected on macOS per the task instructions).
- `ctx.serviceWorkers()[0] ?? await ctx.waitForEvent("serviceworker")` reliably yields the background service worker; in both runs it was already present in `ctx.serviceWorkers()` by the time the script reached that line (no need to fall through to `waitForEvent`).
- `new URL(sw.url()).host` gives a stable 32-char lowercase extension id (`fmigiiinnecobgblbggpecffcfakbndh`). The id was identical across both runs — Chrome derives unpacked-extension ids deterministically from the absolute path of the extension directory, so the id will stay constant for a given checkout path but will differ across machines/checkouts. Anything that hardcodes this id in a test would be fragile; always read it at runtime.
- The background service worker's top-level `fetch("http://127.0.0.1:41800/from-bg")` succeeded with no CORS issue — extension background contexts are not subject to page-level CORS/CSP the way content scripts nominally are, and the server also sends `access-control-allow-origin: *` regardless.
- The content script (`matches: ["http://127.0.0.1/*"]`) injected into the `http://127.0.0.1:41800/page` fixture page and its `fetch("http://127.0.0.1:41800/from-content")` succeeded too — no preflight/CORS failure observed, likely because the request is same-origin (page is served from `127.0.0.1:41800`, fetch target is also `127.0.0.1:41800`) plus the server's permissive CORS header as a backstop.
- One extra, unrequested telemetry hit showed up: `/favicon.ico`. Chromium's page navigation automatically requests `favicon.ico`, and the test HTTP server's catch-all handler logged it as a "hit" since it isn't `/page`. This is harmless noise, not a bug in the extension or spike — any real telemetry-hit assertions in later tasks should either filter by exact path/prefix or ignore `/favicon.ico` explicitly rather than doing an exact-array-equality check.
- No CORS preflight (`OPTIONS`) requests were observed for either `/from-bg` or `/from-content` — both are simple GET fetches with no custom headers, so no preflight was triggered.
- `ctx.close()` and `server.close()` shut down cleanly with no dangling processes or errors.

No changes were needed to the spike script vs. the brief. No `--headless=new` or other flag variants were required.

**Files touched during the spike (temporary):**
- `testdata/hello-extension/background.js` — added `fetch("http://127.0.0.1:41800/from-bg").catch(() => {});` as the first line. Reverted after the run.
- `testdata/hello-extension/content.js` — added `fetch("http://127.0.0.1:41800/from-content").catch(() => {});` as the first line. Reverted after the run.

Both files are back to their Task 1 scaffold state in the committed tree; only `spikes/spike-chromium.ts` and this `RESULTS.md` are new.
