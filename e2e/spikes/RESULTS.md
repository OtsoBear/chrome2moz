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

## Firefox

Ran `pnpm exec tsx spikes/spike-firefox.ts` from `e2e/` with `selenium-webdriver@4.34.0` (Selenium Manager auto-downloaded `geckodriver`) against **Firefox 152.0.6** at `/Applications/Firefox.app`, on macOS.

**Verdict: all three things worked, first try, no deviations from the brief's script.** No MV2 fallback was needed — MV3 `background.scripts` (event page, not `type: "module"` service worker) installed and ran without error.

Output:

```
uuids pref line found: true
telemetry hits: [ '/from-bg', '/favicon.ico', '/from-content' ]
```

Observed behaviors:

- `driver.installAddon(xpi, true)` (temporary install, second arg `true`) accepted the zipped MV3 extension with `background: { scripts: ["background.js"] }` — no `background.service_worker` support needed/attempted; Firefox 152 still uses the MV2-style event-page background shape for MV3 extensions (`browser_specific_settings.gecko.id` is required for `installAddon`'s temporary-install path to assign a deterministic UUID-free id internally, but the *external* extension id doesn't matter — only the `gecko.id` used to look up the UUID in `prefs.js`).
- `browser_specific_settings.gecko.id: "c2m-hello@test"` was accepted as the addon identity. Without a `gecko.id`, Firefox would assign an auto-generated id, making it harder to know which key in the `uuids` pref map corresponds to this extension — always set an explicit `gecko.id` for deterministic lookup.
- `caps.get("moz:profile")` reliably returns the temporary profile directory path used by the geckodriver-launched Firefox instance; `prefs.js` is written into that directory.
- Timing: a `driver.sleep(3000)` after `driver.get(...)` was sufficient for Firefox to have flushed the `extensions.webextensions.uuids` pref to `prefs.js` on disk. This pref appears to be written at addon-install time (not on shutdown), since the file was readable mid-session without calling `driver.quit()` first. A shorter sleep was not tested; 3000ms as given in the brief worked without flakiness across the run.
- **prefs.js escaping format**: the `uuids` pref is stored as a single `user_pref(...)` call whose value is a JSON-stringified object, itself embedded as a double-quoted JS string literal — so every inner `"` is backslash-escaped (`\"`). Sanitized sample (one real entry plus the test extension's, others elided):

  ```
  user_pref("extensions.webextensions.uuids", "{\"newtab@mozilla.org\":\"9b3db26f-c173-4333-9dc4-b07efa6f4c80\",\"c2m-hello@test\":\"18b899fe-ad88-4400-99a5-cecf8b4c9cbe\"}");
  ```

  The brief's regex (`/extensions\.webextensions\.uuids.*?"({.*?})\\?"/`) matches and captures group 1 as the *raw escaped* JSON text (e.g. `{\"c2m-hello@test\":\"...\"...}` with literal backslash-quote sequences still in it, since the capture group boundary sits just inside the outer quotes). **The real parser (Task 8) must unescape `\"` → `"` before `JSON.parse`-ing** — i.e. `JSON.parse(captured.replace(/\\"/g, '"'))` — then look up the value by the `gecko.id` key (here `c2m-hello@test`) to get the `moz-extension://<uuid>/` UUID. Confirmed manually: the captured/unescaped map contained `"c2m-hello@test": "18b899fe-ad88-4400-99a5-cecf8b4c9cbe"`, matching the extension's assigned UUID.
- Both the background script's `fetch("http://127.0.0.1:41801/from-bg")` and the content script's `fetch("http://127.0.0.1:41801/from-content")` (injected via `content_scripts.matches: ["http://127.0.0.1/*"]` into the `/page` fixture) reached the localhost server successfully — no CORS or extension-permission blocking observed, consistent with `host_permissions: ["http://127.0.0.1/*"]` being granted on temporary install.
- Same `/favicon.ico` noise hit seen in the Chromium spike also appeared here (Firefox auto-requests it on navigation); same guidance applies — later telemetry assertions should filter/ignore it rather than doing exact-array equality.
- A visible Firefox window appeared during the run, as expected for a spike (no headless flag was set). `driver.quit()` and `server.close()` shut down cleanly with no dangling processes.

No changes were needed to the spike script vs. the brief; the MV3 event-page shape worked as-is, so the MV2-fallback branch in Step 2 was not exercised.

## Commands

Ran `pnpm exec tsx spikes/spike-commands.ts` from `e2e/` with `playwright@1.54.0` (Chromium) and `selenium-webdriver@4.46.0` + Firefox 152 (`geckodriver` via Selenium Manager), on macOS.

**Verdict: neither browser fires `chrome.commands.onCommand` from synthetic input. Commands probe ships as `skipped: dispatch-unsupported` per the brief's fallback.**

Output:

```
chromium onCommand fired after Control+Shift+9: {}
chromium onCommand fired after Meta+Shift+9: {}
=== chromium verdict === { fired: false, chord: null }
firefox cmd-fired hits after Control+Shift+9: []
firefox cmd-fired hits after Meta+Shift+9: []
=== firefox verdict === { fired: false, chord: null }
```

### Chromium (Playwright `keyboard.press`)

- **Did not fire** for either `Control+Shift+9` or `Meta+Shift+9`, checked via `sw.evaluate(() => chrome.storage.local.get("lastCommand"))` after each chord.
- The macOS caveat in the brief is real and was confirmed directly: a diagnostic call to `chrome.commands.getAll()` from the service worker showed the registered shortcut as `"⇧⌘9"` (Shift+Cmd+9) — i.e. Chrome's MacCtrl→Command mapping applies to the manifest's `Ctrl+Shift+9` default binding on macOS, exactly as the brief warned. So `Meta+Shift+9` (not `Control+Shift+9`) is the chord that *should* match the registered accelerator on this platform — but even that correct chord form did not trigger `onCommand`.
- Root-cause check: a throwaway diagnostic page (not committed) added a page-level `window.addEventListener("keydown", ...)` before calling `page.keyboard.press("Meta+Shift+9")`. The DOM **did** receive all three keydown events with the correct modifiers (`Meta/meta=true`, `Shift/meta=true/shift=true`, `9/meta=true/shift=true`), proving Playwright's CDP-based key dispatch reaches the page's content process correctly. Yet `chrome.commands.onCommand` still never fired.
- Conclusion: `chrome.commands` shortcuts are matched by the browser's native UI-level accelerator table, which sits *above* the content process and is normally the thing that intercepts the keystroke before it would even reach a page's DOM. Playwright's `Input.dispatchKeyEvent` (CDP) injects events into the renderer/content-process input pipeline directly and does not go through that native accelerator interception path, so it can deliver keys to a web page but cannot trigger a registered extension command. This matches widely-reported Playwright/Puppeteer limitations around testing extension keyboard shortcuts.

### Firefox (WebDriver Actions)

- **Did not fire** for either `Control+Shift+9` or `Meta+Shift+9`, checked via the content-script relay described below.
- Readback mechanism used (simpler than the brief's message-passing sketch, chosen because it worked on the first try): the FF test extension's `background.js` sets `chrome.storage.local.lastCommand` on `commands.onCommand` (same as the real hello-extension); its `content.js` adds a `chrome.storage.onChanged` listener that, on a `lastCommand` change, does `fetch("http://127.0.0.1:41802/cmd-fired?cmd=...")` against the spike's local HTTP server. The spike just checks the server's hit log — no need for a second relay hop through `runtime.sendMessage`.
- Root-cause check (mirrors the Chromium one): a throwaway diagnostic page (not committed) with a page-level `keydown` listener confirmed `driver.actions().keyDown(Key.CONTROL/META).keyDown(Key.SHIFT).sendKeys("9")...perform()` reaches the DOM correctly for both modifiers (`Control/ctrl=true/shift=true` and `Meta/meta=true/shift=true` observed on keydown; the final key event reported `key: "("` — Shift+9's produced character on a US layout — rather than `"9"`, which is expected and irrelevant here since `chrome.commands` matching is based on the physical key code, not the DOM `KeyboardEvent.key` value). As with Chromium, the DOM saw the keys but the extension's `onCommand` listener never fired.
- Conclusion: same root cause as Chromium — WebDriver's synthetic Actions-API key dispatch lands in the content process's input pipeline but does not pass through Firefox's native keyboard-shortcut (accelerator) handling that `browser.commands` shortcuts are registered against.
- Per the brief, `Ctrl+Shift+9` (no Mac-specific override) was tried first since Firefox does not apply Chrome's MacCtrl→Command auto-translation; `Meta+Shift+9` was also tried for parity with the Chromium half. Neither fired, so the platform-specific chord question is moot here — the dispatch path itself doesn't reach the accelerator table regardless of chord form.

### Decision

Both browsers: **did not fire**, for the same underlying reason (synthetic input from both Playwright/CDP and Selenium/WebDriver bypasses the browser's native global-accelerator table that `chrome.commands`/`browser.commands` shortcuts are matched against). This is a clean, well-understood negative result, not a chord-mismatch or timing issue — confirmed via DOM-level keydown diagnostics on both browsers showing correct key delivery. Per the brief's fallback: the commands probe ships as `skipped: dispatch-unsupported` in later tasks' reports; later tasks should treat it as optional and not block on it.
