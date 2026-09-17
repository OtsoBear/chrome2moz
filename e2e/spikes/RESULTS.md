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

> **Update (30.07.2026):** local Firefox auto-updated to **153.0.1**, which Selenium Manager now pairs with **geckodriver 0.37.1**. That geckodriver version refuses WebDriver navigation (`driver.get(...)`) to internal URL schemes (`moz-extension:`, `about:`, `chrome:`) unless the geckodriver server itself is launched with `--allow-system-access` (a geckodriver process flag, not a `moz:firefoxOptions` capability — geckodriver rejects it if set there). `src/firefoxDriver.ts` now passes this via `firefox.ServiceBuilder().addArguments("--allow-system-access")` on `Builder().setFirefoxService(...)`; no marionette chrome-context switch needed (chrome context rejects `Get` outright with "Only supported in content context"). Re-verified: full `pnpm e2e` green.

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

## Kill/Wake

Ran `E2E_INTEGRATION=1 pnpm exec tsx spikes/spike-killwake.ts` from `e2e/` with `playwright@1.62.0` (bundled Chromium 151.0.7922.34, `--headless=new`) and `selenium-webdriver@4.46.0` against **Firefox** at `/Applications/Firefox.app` (`-headless`, Selenium Manager `geckodriver`), on macOS. Ran three consecutive times for reproducibility; identical qualitative result every time.

**Verdict: both browsers confirmed killed + rebooted under headless automation.** Chromium via CDP `ServiceWorker.stopAllWorkers` (as given in the plan). Firefox via the `extensions.background.idle.timeout` pref set to `1000` at launch plus an idle wait past that window — no explicit "kill" call exists for Firefox event pages, but the idle-timeout-driven suspend is real, observable, and reboots correctly. **No `kill-unsupported` fallback is needed for Firefox.**

### Instrumentation (why the result is trustworthy, not noise)

A shared throwaway `background.js` (prototyping the Task 4 gate design) does two things on every (re)boot:

```javascript
const bootMark = Date.now() + ":" + Math.random(); // computed ONCE per top-level script run
async function recordBoot(reason) {
  const cur = await chrome.storage.local.get(["boots"]);
  const boots = (cur.boots || 0) + 1;
  await chrome.storage.local.set({ boots, bootMark, lastWake: reason });
}
recordBoot("startup");
chrome.tabs.onUpdated.addListener((_tabId, info) => {
  if (info.status === "complete") recordBoot("tabs.onUpdated");
});
```

`bootMark` is computed once, synchronously, at top level — it can only change if the whole script re-executes in a new global scope (a genuine restart), unlike `boots`, which increments on every call regardless of restart. **First attempt at this instrumentation had a bug**: `bootMark` was regenerated inside `recordBoot()` on every call, so it could not distinguish "a live worker handled one more event" from "the worker restarted." That produced a misleading first raw run (committed history of this file does not include it; caught before recording a verdict). Fixed by hoisting the `Date.now() + ":" + Math.random()` computation above `recordBoot`, matching the design already specified for the Task 4 gate's `background.js`.

### Chromium

- `ctx.serviceWorkers()[0]` gives the initial worker; state read via `sw.evaluate(() => chrome.storage.local.get(...))` — `boots: 1` after the 3s settle (the implicit initial `about:blank` tab does not itself trigger `tabs.onUpdated` for this extension; confirmed empirically, `boots` stayed at 1 through the settle wait).
- Kill: `ServiceWorker.stopAllWorkers` was sent over a CDP session attached to the existing `about:blank` page (`ctx.newCDPSession(target)`), not a browser-level session — `ServiceWorker.*` is a page/target-scoped CDP domain; `Browser.newBrowserCDPSession()` (tried first, to avoid any page-touching confound) throws `'ServiceWorker.enable' wasn't found` because that session type doesn't expose the domain. The already-open `about:blank` tab was confirmed not to introduce its own `tabs.onUpdated`, so using it as the CDP target is confound-free.
- `ctx.serviceWorkers()` **never dropped the old worker handle** in the 3s poll after `stopAllWorkers` (`old worker handle cleared: no`), and the object returned after the wake trigger was reference-equal to the original (`new Playwright worker object identity: false`). Playwright appears to key its `ServiceWorker` wrapper by scope/URL and rebind it across a respawn rather than surfacing a new object — this is a Playwright bookkeeping quirk, not evidence the kill failed, and is not treated as the deciding signal.
- The deciding signal is `bootMark`, read from inside the extension's own runtime: after the wake trigger (one `ctx.newPage()`), `bootMark` changed on every one of three runs, and `boots` advanced by exactly 2 (`recordBoot("startup")` from the fresh top-level run, immediately followed by `recordBoot("tabs.onUpdated")` delivering the event that woke it — the textbook sequence for a genuinely dead worker receiving a pending event, versus +1 for a still-alive worker just handling one more event). `lastWake` read back as `"tabs.onUpdated"` every time.
- **Confirmed**: `ServiceWorker.stopAllWorkers` genuinely terminates the extension service worker under `--headless=new`, and it reboots (fresh top-level execution) on the next qualifying event, three runs in a row, no flakiness observed.

### Firefox

- Same shared `background.js`/`content.js` design, converted to `background.scripts` (event page, matching the existing `firefoxDriver.ts`/spike-firefox conversion shape) with `browser_specific_settings.gecko.id` set; `extensions.background.idle.timeout` set to `1000` via `firefox.Options().setPreference(...)` at launch.
- No WebDriver command exists to force-terminate a Firefox event page, and there is no equivalent to Playwright's `serviceWorkers()` list to directly observe live/dead state — so the signal has to come the same way the real probe will observe it: the content script reads `chrome.storage.local` on each fixture page load and reports it out via `fetch()` to the spike's local HTTP server (the same telemetry shape the real shim/gate already uses).
- Timeline per run: install, settle 2s, navigate to the fixture page (`boots: 3` after startup + two implicit `tabs.onUpdated` from the browser's own initial-tab lifecycle — unlike Chromium, Firefox's initial window's `about:blank` load plus the subsequent navigation both counted), settle 2s (report captured), **idle 4s** (past the 1000ms pref, no navigation, no extension activity), then wake: open one new tab (`switchTo().newWindow("tab")`, itself a `tabs.onUpdated`) and navigate it to the fixture page (a second `tabs.onUpdated`), settle 2s, report captured again.
- If the event page had stayed alive through the idle window, the wake trigger's two `tabs.onUpdated` calls would add exactly 2 to `boots` and leave `bootMark` unchanged. Observed on all three runs: `boots` advanced by exactly **3** (one `recordBoot("startup")` from a fresh top-level run plus the two `tabs.onUpdated` deliveries) and `bootMark` **changed** every time.
- **Confirmed**: Firefox's converted event-page background does idle-terminate past `extensions.background.idle.timeout` under headless Selenium/WebDriver, and does reboot (fresh top-level execution) on the next `tabs.onUpdated`-triggering event, three runs in a row, no flakiness observed.

### Decision

Both sides confirmed kill + reboot, observed via `bootMark` (a signal that can only change on a genuine top-level script re-execution, immune to the "listener fired again on a still-live worker" false positive). Task 2's Firefox `killBackground()` should report `{ killed: true, mechanism: "idle-timeout" }` after sleeping past the idle-timeout pref (no fallback `kill-unsupported` needed). Task 4's `killwake-gate` corpus entry should use `allowed_diffs: []` with no `_firefox_kill_caveat` allowance — both sides are expected to show `boots` going `1 -> 2`-equivalent identically once wired through the real `killWakeProbe` (which drives one wake tab per side, matching this spike's confirmed single-trigger behavior, not the two-navigation wake sequence used here to make the Firefox implicit-event accounting airtight).
## Web snapshots / mitmproxy

Ran `E2E_INTEGRATION=1 pnpm exec tsx spikes/spike-snapshot.ts` from `e2e/` with `mitmproxy 12.2.3` (installed via `uv tool install mitmproxy`, Python 3.12.11), `playwright@1.54.0` (Chromium) and `selenium-webdriver@4.34.0` + Firefox (Selenium Manager), on macOS.

**Verdict: PASS on both browsers, first run. Both Chromium and Firefox load `https://example.test/` served entirely by the mitmproxy addon (no live network), and the extension content script (`testdata/hello-extension`, `matches: ["http://127.0.0.1/*", "https://example.test/*"]`) injects on the served page in both. Spike 3 is fully green; the ~60-minute fallback in Global Constraints was not needed.**

Output:

```
=== chromium ===
{ title: 'snapshot spike', injected: true }
=== firefox ===
{ title: 'snapshot spike', injected: true }
```

### Working mitmdump invocation

```
mitmdump -q -p <port> -s snapshots/serve_addon.py --set upstream_cert=false --set connection_strategy=lazy
```

with `C2M_SNAPSHOT_ID=<entry-id>` set in the environment (selects which entry of `snapshots/index.json` the addon serves).

- `-p <port>`: listen port (`--listen-port` long form). `-q`: quiet (suppress the flow log to stdout).
- `-s snapshots/serve_addon.py`: load the serve-addon (`--scripts` long form), relative to the mitmdump working directory (spawned with `cwd: e2e/`).
- `--set upstream_cert=false`: **required**. mitmproxy's default `connection_strategy=eager` plus `upstream_cert=true` makes it eagerly open a real TCP/TLS connection to the upstream host at CONNECT time, to clone the real server's certificate fields for the MITM cert it presents to the client. For a fixture-only hostname like `example.test` that resolves to nothing routable, this eager upstream connection either hangs or fails, and the client-side TLS handshake with mitmproxy never completes (`curl` observed this as `CURLE_RECV_ERROR`, exit 56, with the addon's `request` hook never even being reached). Setting `upstream_cert=false` makes mitmproxy generate its self-signed leaf cert without contacting the upstream host at all.
- `--set connection_strategy=lazy`: belt-and-suspenders with the above so mitmproxy never opens the upstream connection unless a flow is actually let through un-intercepted (which the addon never does for a snapshotted host, since `request()` always sets `flow.response` before mitmproxy would otherwise connect upstream).
- Confirmed directly with `curl --insecure -x http://127.0.0.1:<port> https://example.test/` before writing the TS driver: without the two `--set` flags, the request hung/failed at the TLS layer; with them, it returned the stored HTML (`200`) immediately, and an unlisted host (`https://other.test/`) returned the addon's `204` refusal, proving no live network occurs for either the matched or unmatched case.

### Proxy config per browser

- **Chromium (Playwright):** `chromium.launchPersistentContext(..., { proxy: { server: "http://127.0.0.1:<port>" }, ignoreHTTPSErrors: true, args: ["--headless=new", "--ignore-certificate-errors", ...] })`. `ignoreHTTPSErrors` alone was sufficient for Playwright's own navigation/assertion APIs to treat the mitmproxy leaf cert as trusted; `--ignore-certificate-errors` was added as a Chromium-native belt-and-suspenders flag but was not isolated as strictly required (not worth the extra spike time to bisect, given the run passed).
- **Firefox (Selenium):** proxy set via prefs, not capabilities: `network.proxy.type=1`, `network.proxy.http`/`network.proxy.ssl="127.0.0.1"`, `network.proxy.http_port`/`network.proxy.ssl_port=<port>`, `network.proxy.allow_hijacking_localhost=true` (permits proxying to a loopback destination, which Firefox otherwise special-cases), `network.proxy.no_proxies_on=""` (clears the default localhost bypass list so `127.0.0.1` doesn't shortcut past the proxy). TLS acceptance via `firefox.Options#setAcceptInsecureCerts(true)`, which maps to the WebDriver `acceptInsecureCerts` capability.

### TLS approach

Accept-insecure only, both browsers, **no NSS import of mitmproxy's CA anywhere**: Playwright's `ignoreHTTPSErrors: true` and Selenium/geckodriver's `acceptInsecureCerts: true` (via `setAcceptInsecureCerts`) are sufficient for both to complete the TLS handshake against mitmproxy's self-signed leaf cert and load the page. This matches the plan's TLS approach exactly; mitmproxy's own CA cert (normally at `~/.mitmproxy/mitmproxy-ca-cert.pem`) was never referenced or imported into either browser's trust store.

### Content-script injection confirmation

Used a DOM marker rather than a telemetry fetch, consistent with `testdata/hello-extension/content.js`'s existing pattern (`document.documentElement.dataset.c2mHello = "1"`) and with how `probes.ts`/`contentProbe` already treats fixture pages: a fetch-based telemetry hit would itself be intercepted by the same mitmproxy instance (all browser traffic is proxied, not just the snapshot host), adding complexity with no extra proof value. `content_scripts.matches` on `testdata/hello-extension/manifest.json` was extended (additively, not replacing) to `["http://127.0.0.1/*", "https://example.test/*"]` so existing spikes/tests that rely on the `127.0.0.1` fixture match are unaffected. Confirmed via `page.evaluate(...)` (Chromium) and `driver.executeScript(...)` (Firefox) that `document.documentElement.dataset.c2mHello === "1"` on the mitmproxy-served `https://example.test/` page, in both browsers.

### Notes for later tasks

- Both browsers were launched headless (`--headless=new` for Chromium per `chromeDriver.ts`'s existing pattern; `-headless` for Firefox), consistent with Global Constraints.
- The serve-addon refuses any host not present in the selected snapshot entry with an empty `204`, so subresource requests (images, scripts, etc.) to unknown hosts don't hang the page load; this was exercised implicitly by the browsers' own subresource probing (favicon etc.) during the run without incident.
- `startSnapshotServer` (Task 3) should wait for the mitmdump listen port to be open before returning (a `net.connect` poll loop, as used in the spike) rather than a fixed sleep, since mitmdump's own startup time varies.
