# onMessage async-listener + synchronous sendResponse Compat Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a chrome2moz compat shim that restores Chrome's precedence for `runtime.onMessage` listeners that are `async` functions AND call `sendResponse(...)` synchronously without `return true`, so extensions of the OneNote-Web-Clipper shape (issue #8) work on Firefox instead of receiving `undefined` and crashing.

**Architecture:** One guarded plain-JS shim, `shims/runtime-onmessage-compat.js`, that wraps `runtime.onMessage`/`onMessageExternal` `addListener` in whatever context it loads. It is (a) added to `background.scripts` like the other compat shims (fixes background-context listeners) and (b) injected as the first `<script>` into every packaged extension HTML page via a NEW converter step (fixes offscreen-document / popup / options listeners, which is where OneNote's buggy listener actually lives). Verified by Rust unit tests (shim content + HTML injection) and the existing `offscreen-gate` differential corpus entry, whose four issue-#8 control allowances are removed once the fix lands.

**Tech Stack:** Rust converter (`src/transformer/`), plain-JS shim, the existing e2e harness (`e2e/`, pnpm + tsx + vitest, Playwright + Selenium).

**Spec:** GitHub issue #8; `docs/superpowers/specs/2026-07-31-offscreen-polyfill-design.md` (Verification item 4 describes this exact incompatibility and the offscreen-gate probe that gates it).

## Global Constraints

- **Headless always.** Chromium `--headless=new` (`e2e/src/chromeDriver.ts`), Firefox `-headless` (`e2e/src/firefoxDriver.ts`). Run `grep -n headless e2e/src/chromeDriver.ts e2e/src/firefoxDriver.ts` before any corpus run and confirm both. Never headed. (Prior session flooded the user with ~200 popups.)
- **pnpm only** in `e2e/`; **uv only** for Python. **No em dashes** anywhere.
- Build the converter with `cargo build --release`; keep `cargo clippy` clean and `cargo test` green.
- Firefox/geckodriver via Selenium Manager (do not install manually); Playwright Chromium is cached.
- Work only in worktree `/Users/otsov/.superset/worktrees/1b400713-93e8-4131-93b2-695e1e5e7562/onmessage-compat` on branch `feature/e2e-onmessage-compat`. Never cd into sibling worktrees or the main checkout.
- The shim must be guarded and idempotent: wrap only what exists, mark the event object so a second load (background + HTML both include it) is a no-op. In Firefox `chrome` and `browser` are DISTINCT objects; patch both roots.

---

### Task 1: The compat shim generator + background wiring

**Files:**
- Modify: `src/transformer/shims.rs` (add `create_runtime_onmessage_compat()`, push it in `generate_shims`)
- Modify: `src/transformer/manifest.rs` (`transform_background`, add the shim to the `scripts` list alongside the other compat shims)
- Test: `#[cfg(test)]` in `src/transformer/shims.rs`

**Interfaces:**
- Produces: converted output contains `shims/runtime-onmessage-compat.js`, listed in `background.scripts` among the other `shims/*.js` entries (before the extension's own scripts).
- Shim contract (the JS behavior later tasks and the corpus rely on): for a listener registered on `runtime.onMessage`/`onMessageExternal`, if the listener returns a thenable AND `sendResponse` was invoked during the listener's synchronous execution, the returned Promise is suppressed (wrapper returns `undefined`) so Firefox delivers the already-called `sendResponse` value. All other shapes are left native. `removeListener`/`hasListener` keep working via a `WeakMap` from the caller's callback to the wrapped one.

- [ ] **Step 1: Read the existing pattern.** Read `create_runtime_compat()` in `src/transformer/shims.rs` (the generator/`NewFile` shape) and `transform_background` in `src/transformer/manifest.rs` (lines ~90-136, where the `shims/*.js` entries are pushed). Note that shims are pushed unconditionally except the offscreen polyfill.

- [ ] **Step 2: Write failing Rust tests.** Add to the `#[cfg(test)] mod tests` in `src/transformer/shims.rs`:

```rust
#[test]
fn test_runtime_onmessage_compat_generation() {
    let shim = create_runtime_onmessage_compat();
    assert_eq!(shim.path, PathBuf::from("shims/runtime-onmessage-compat.js"));
    // Core behavior markers.
    assert!(shim.content.contains("onMessage"));
    assert!(shim.content.contains("onMessageExternal"));
    assert!(shim.content.contains("respondedSync"));
    assert!(shim.content.contains("typeof ret.then === \"function\""));
    // Idempotency guard + both-roots patching.
    assert!(shim.content.contains("__c2m_onmessage_compat__"));
    assert!(shim.content.contains("typeof browser"));
    // Preserves removeListener/hasListener mapping.
    assert!(shim.content.contains("removeListener"));
    assert!(shim.content.contains("WeakMap"));
}

#[test]
fn test_runtime_onmessage_compat_included_in_shims() {
    // generate_shims must always include the onMessage compat shim (guarded + no-op safe).
    // Build a minimal ConversionContext the same way the other generate_shims tests do
    // (mirror an existing generate_shims-level test in this repo; if none exists, assert via
    // the generator being referenced in generate_shims by checking the file list a conversion
    // produces in Task 3's e2e run instead, and delete this unit test).
}
```

If there is no existing `generate_shims`-level unit test to mirror for the second test, delete it and rely on the Task 3 e2e run to prove inclusion; keep the first test.

- [ ] **Step 3: Run tests, confirm the first fails.**

Run: `cargo test runtime_onmessage_compat`
Expected: FAIL (`create_runtime_onmessage_compat` not defined).

- [ ] **Step 4: Implement the generator.** Add to `src/transformer/shims.rs` (after `create_runtime_compat`), embedding this JS verbatim:

```rust
/// runtime.onMessage async-listener + synchronous sendResponse compat.
/// Chrome honors a sendResponse() called synchronously inside a listener even when the
/// listener is an async function (which implicitly returns a Promise). Firefox instead uses
/// the async listener's returned Promise (resolves to undefined without an explicit return),
/// discarding the sendResponse value. This restores Chrome's precedence. See issue #8.
fn create_runtime_onmessage_compat() -> NewFile {
    let content = r#"// chrome2moz: runtime.onMessage async-listener + synchronous sendResponse compat.
// Chrome honors a sendResponse() invoked synchronously inside a listener, even when the
// listener is an async function (which implicitly returns a Promise). Firefox instead uses
// the async listener's own returned Promise (resolves to undefined without an explicit
// return), discarding the synchronous sendResponse value. This wrapper restores Chrome's
// precedence: when a listener answers synchronously via sendResponse AND returns a thenable,
// the returned Promise is suppressed so Firefox delivers the sendResponse value. All other
// shapes (return true then a late sendResponse; a listener that returns a real Promise value;
// a plain synchronous listener) are left native.
(() => {
  const roots = [];
  if (typeof browser !== "undefined") roots.push(browser);
  if (typeof chrome !== "undefined" && (typeof browser === "undefined" || chrome !== browser)) roots.push(chrome);
  for (const api of roots) {
    const rt = api && api.runtime;
    if (!rt) continue;
    for (const evName of ["onMessage", "onMessageExternal"]) {
      const ev = rt[evName];
      if (!ev || typeof ev.addListener !== "function" || ev.__c2m_onmessage_compat__) continue;
      const origAdd = ev.addListener.bind(ev);
      const origRemove = typeof ev.removeListener === "function" ? ev.removeListener.bind(ev) : null;
      const origHas = typeof ev.hasListener === "function" ? ev.hasListener.bind(ev) : null;
      const map = new WeakMap();
      ev.addListener = function (cb, ...rest) {
        if (typeof cb !== "function") return origAdd(cb, ...rest);
        const wrapped = function (message, sender, sendResponse) {
          let respondedSync = false;
          const wrappedSR = (v) => { respondedSync = true; try { return sendResponse(v); } catch (e) {} };
          const ret = cb.call(this, message, sender, wrappedSR);
          // Async listener that already answered synchronously via sendResponse: suppress its
          // Promise so Firefox uses the sendResponse value we just delivered (Chrome order).
          if (respondedSync && ret && typeof ret.then === "function") return undefined;
          return ret;
        };
        try { map.set(cb, wrapped); } catch (e) {}
        return origAdd(wrapped, ...rest);
      };
      if (origRemove) ev.removeListener = function (cb, ...rest) { let w; try { w = map.get(cb); } catch (e) {} return origRemove(w || cb, ...rest); };
      if (origHas) ev.hasListener = function (cb, ...rest) { let w; try { w = map.get(cb); } catch (e) {} return origHas(w || cb, ...rest); };
      try { Object.defineProperty(ev, "__c2m_onmessage_compat__", { value: true }); } catch (e) { ev.__c2m_onmessage_compat__ = true; }
    }
  }
})();
"#;
    NewFile {
        path: PathBuf::from("shims/runtime-onmessage-compat.js"),
        content: content.to_string(),
        purpose: "Restores Chrome's sendResponse precedence for async onMessage listeners on Firefox (issue #8)".to_string(),
    }
}
```

Then in `generate_shims`, add `shims.push(create_runtime_onmessage_compat());` alongside the other unconditional pushes (e.g. right after `create_runtime_compat()`).

- [ ] **Step 5: Wire into background.scripts.** In `src/transformer/manifest.rs::transform_background`, add `scripts.push("shims/runtime-onmessage-compat.js".to_string());` in the compat-shim block (e.g. right after the `runtime-compat.js` push at line ~102), so background-context listeners are also fixed.

- [ ] **Step 6: Run tests + clippy.**

Run: `cargo test runtime_onmessage_compat` (expect PASS), then `cargo test` (expect all green), then `cargo clippy` (expect clean).

- [ ] **Step 7: Commit.**

```bash
cd /Users/otsov/.superset/worktrees/1b400713-93e8-4131-93b2-695e1e5e7562/onmessage-compat
git add src/transformer/shims.rs src/transformer/manifest.rs
git commit -m "feat(shims): runtime.onMessage async+sync sendResponse compat shim (#8)"
```

---

### Task 2: Inject the compat shim into extension HTML pages

**Files:**
- Create: `src/transformer/html_inject.rs` (new module)
- Modify: `src/transformer/mod.rs` (call the new step in `transform_extension`, register the module)
- Test: `#[cfg(test)]` in `src/transformer/html_inject.rs`

**Interfaces:**
- Consumes: `context.source` (the `Extension`), its file enumeration and `get_file_content`, and the `ModifiedFile { path, original_content, new_content, changes: Vec<FileChange> }` shape (see `src/transformer/offscreen_converter.rs` for a construction example) plus `FileChange`/`ChangeType`.
- Produces: for each packaged `.html`/`.htm` source file that contains a `<script`, a `ModifiedFile` whose `new_content` has `<script src="{rel}shims/runtime-onmessage-compat.js"></script>` inserted immediately before the first `<script`, where `{rel}` is `../` repeated once per directory level of the HTML file (empty for a root-level file like `offscreen.html`). These `ModifiedFile`s are appended to the conversion result's modified files so the packager writes them.

**Why HTML injection is required:** OneNote's buggy listener is in `offscreen.js`, loaded by `offscreen.html` (an extension page), not the background. Background-only injection (Task 1) cannot fix it. The shim is guarded/idempotent, so loading it in both the background and every HTML page is safe.

- [ ] **Step 1: Read the modified-files plumbing.** Read `transform_extension` in `src/transformer/mod.rs` (how `modified_files` from the JS transformer are collected and passed into `ConversionResult`) and `src/packager/builder.rs::build_directory` (how modified files are written vs. source files copied), and the `ModifiedFile`/`FileChange`/`ChangeType` definitions (`src/models/`). Confirm how to enumerate source HTML files (mirror `context.source.get_javascript_files()`; if there is no HTML enumerator, filter the source file list by extension `html`/`htm`).

- [ ] **Step 2: Write failing tests.** Create `src/transformer/html_inject.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_before_first_script_at_root() {
        let html = "<!DOCTYPE html><html><body><script src=\"offscreen.js\"></script></body></html>";
        let out = inject_onmessage_shim(html, std::path::Path::new("offscreen.html"));
        assert_eq!(
            out.as_deref(),
            Some("<!DOCTYPE html><html><body><script src=\"shims/runtime-onmessage-compat.js\"></script><script src=\"offscreen.js\"></script></body></html>")
        );
    }

    #[test]
    fn computes_relative_prefix_for_nested_html() {
        let html = "<html><head><script src=\"x.js\"></script></head></html>";
        let out = inject_onmessage_shim(html, std::path::Path::new("pages/popup.html")).unwrap();
        assert!(out.contains("<script src=\"../shims/runtime-onmessage-compat.js\"></script><script src=\"x.js\">"));
    }

    #[test]
    fn skips_html_without_scripts() {
        let html = "<html><body><p>no scripts here</p></body></html>";
        assert_eq!(inject_onmessage_shim(html, std::path::Path::new("about.html")), None);
    }

    #[test]
    fn is_idempotent() {
        let html = "<html><body><script src=\"shims/runtime-onmessage-compat.js\"></script><script src=\"a.js\"></script></body></html>";
        assert_eq!(inject_onmessage_shim(html, std::path::Path::new("a.html")), None);
    }
}
```

- [ ] **Step 3: Run tests, confirm they fail.**

Run: `cargo test html_inject`
Expected: FAIL (`inject_onmessage_shim` not defined).

- [ ] **Step 4: Implement `inject_onmessage_shim`.** In `src/transformer/html_inject.rs`:

```rust
//! Injects the runtime.onMessage compat shim (issue #8) as the first <script> of each
//! packaged extension HTML page, so listeners in offscreen documents / popups / options
//! pages are fixed too (background.scripts injection alone cannot reach them).

use std::path::Path;

const SHIM_REL: &str = "shims/runtime-onmessage-compat.js";

/// Returns the modified HTML with the compat shim inserted before the first `<script`, or
/// None when the page has no script tag (nothing to fix) or the shim is already present.
pub fn inject_onmessage_shim(html: &str, html_path: &Path) -> Option<String> {
    if html.contains(SHIM_REL) {
        return None; // already injected (idempotent)
    }
    let idx = find_first_script(html)?;
    let depth = html_path.parent().map(|p| p.components().count()).unwrap_or(0);
    let rel: String = "../".repeat(depth);
    let tag = format!("<script src=\"{}{}\"></script>", rel, SHIM_REL);
    let mut out = String::with_capacity(html.len() + tag.len());
    out.push_str(&html[..idx]);
    out.push_str(&tag);
    out.push_str(&html[idx..]);
    Some(out)
}

/// Case-insensitive search for the first `<script` opening tag.
fn find_first_script(html: &str) -> Option<usize> {
    let lower = html.to_ascii_lowercase();
    lower.find("<script")
}
```

- [ ] **Step 5: Run the module tests.**

Run: `cargo test html_inject`
Expected: PASS.

- [ ] **Step 6: Wire into the conversion.** In `src/transformer/mod.rs`: register the module (`mod html_inject;` / `pub mod html_inject;` per repo convention), and in `transform_extension`, after the JS transformer loop and before/with the shim generation, enumerate source HTML files, run `inject_onmessage_shim` on each, and for every `Some(new_content)` push a `ModifiedFile` into the same `modified_files` vec the JS transformer feeds (construct it like `offscreen_converter.rs` does: set `path` to the HTML's relative path, `original_content` to the source HTML, `new_content` to the injected HTML, and `changes` to a single `FileChange` describing "Injected onMessage compat shim"). Add a `manifest_changes.push("Injected runtime.onMessage compat shim into N HTML page(s) (issue #8)")`-style note.

- [ ] **Step 7: Full Rust verification.**

Run: `cargo test` (all green), `cargo clippy` (clean).

- [ ] **Step 8: Commit.**

```bash
git add src/transformer/html_inject.rs src/transformer/mod.rs
git commit -m "feat(convert): inject onMessage compat shim into extension HTML pages (#8)"
```

---

### Task 3: Prove it with the offscreen-gate corpus entry, re-measure OneNote

**Files:**
- Modify: `e2e/corpus.json` (`offscreen-gate` entry: remove the four issue-#8 control allowances; OneNote entry: re-triage the relay cascade)
- Modify: `docs/superpowers/specs/2026-07-31-offscreen-polyfill-design.md` (Verification item 4: note issue #8 is now fixed and the gate passes with no issue-#8 allowance)

**Interfaces:**
- Consumes: Tasks 1-2 (converter now injects the compat shim into background + HTML). The existing `offscreen-gate` extension already contains the buggy async listener in `e2e/testdata/offscreen-extension/offscreen.js` (the `parse-async` kind) and its background sender in `background.js` (the `parse-async` round-trip writing `offscreenAsyncResult`).

**Expected effect:** before the fix, Firefox's `sendMessage` for `parse-async` resolved to `undefined` (stored as `{UNDEFINED_RESPONSE:true}`), diverging from Chrome's real parsed text, gated by four allowances (`runtime.sendMessage:resolve#hello async offscreen`, `runtime.sendMessage:resolve#null`, `storage.local.set#hello async offscreen`, `storage.local.set#UNDEFINED_RESPONSE`). After the fix, Firefox delivers the real text on both sides, so those four divergences disappear and the allowances must be deleted (the `_issue_8_regression_control` note says exactly this).

- [ ] **Step 1: Build + confirm headless.**

Run: `cargo build --release`
Run: `grep -n headless e2e/src/chromeDriver.ts e2e/src/firefoxDriver.ts` (expect `--headless=new`, `-headless`).

- [ ] **Step 2: Run the gate WITH the four allowances still present (baseline).**

Run: `cd e2e && pnpm e2e --only offscreen-gate`
Read `e2e/results/offscreen-gate/trace-chrome.json` / `trace-firefox.json` and `report.json`. Confirm the `parse-async` round-trip now MATCHES on both sides (background `runtime.sendMessage:resolve` carries the real parsed text on Firefox, and `storage.local.set` stores the real `offscreenAsyncResult` text, not `{UNDEFINED_RESPONSE:true}`). If it does not match, the fix is incomplete: inspect whether the compat shim was injected into the converted `offscreen.html` (check `e2e/results/offscreen-gate/converted/offscreen.html` for the `<script src="shims/runtime-onmessage-compat.js">` tag before `offscreen.js`) and whether it loaded; fix Task 1/2 rather than touching allowances.

- [ ] **Step 3: Delete the four issue-#8 control allowances** from `offscreen-gate.allowed_diffs`: `runtime.sendMessage:resolve#hello async offscreen`, `runtime.sendMessage:resolve#null`, `storage.local.set#hello async offscreen`, `storage.local.set#UNDEFINED_RESPONSE`. Delete their `_notes` entries and the `_issue_8_regression_control` note (or rewrite that note to say the fix shipped and the gate now passes with no issue-#8 allowance, so future regressions re-fail it). Keep the offscreen polyfill allowances (`offscreen.*`, `runtime.getContexts*`, `runtime.getURL#offscreen.html`) untouched; they are issue #6, not #8.

- [ ] **Step 4: Re-run the gate, expect PASS with no issue-#8 allowance.**

Run: `cd e2e && pnpm e2e --only offscreen-gate`
Expected: `PASS  C2M Offscreen Gate`. If it fails, the four divergences are still present -> the fix did not take; do NOT re-add the allowances, fix the converter.

- [ ] **Step 5: Re-measure OneNote.**

Run: `cd e2e && pnpm e2e --only gojbdfnpnhogfdgjbigejoaolejmgdhk`
Read the Firefox trace. Expect Firefox to get PAST the `JSON.parse: unexpected character` crash (issue #8) now that the offscreen relay returns the real value: the `clipperIdProcessed` chain should resolve further, so downstream chrome-only patterns (`runtime.getManifest`, `net.fetch#onenote.com/strings`, `tabs.query#lastFocusedWindow`, `tabs.create#getting-started`, `contextMenus.*`) should shrink or vanish on the divergence list. Re-triage: delete the `runtime.error#JSON.parse: unexpected character` allowance and any now-matched downstream patterns; keep any that remain chrome-only for an unrelated reason with an updated note. Record before/after Firefox background-event counts and the deleted patterns in the entry `_notes` and via `gh issue comment 8`. OneNote stays `quarantined: true` (fixture host is issue #5, unrelated).

- [ ] **Step 6: Update the spec.** In `docs/superpowers/specs/2026-07-31-offscreen-polyfill-design.md` Verification item 4, note that issue #8 is now fixed by `shims/runtime-onmessage-compat.js` (background + HTML-page injection) and the offscreen-gate `parse-async` probe passes with no allowance.

- [ ] **Step 7: Full verification.**

Run: `cargo test`
Run: `cd e2e && pnpm typecheck && pnpm test && pnpm e2e`
Expected: Rust green; harness typecheck/unit green; full corpus `PASS LatexToCalc`, `PASS C2M Offscreen Gate`, OneNote reported.

- [ ] **Step 8: Commit.**

```bash
git add e2e/corpus.json docs/superpowers/specs/2026-07-31-offscreen-polyfill-design.md
git commit -m "test(e2e): offscreen-gate passes without issue-#8 allowance; re-triage OneNote (#8)"
```

---

### Task 4: Push and open the PR

- [ ] **Step 1: Push.**

```bash
git push -u origin feature/e2e-onmessage-compat
```

- [ ] **Step 2: Open the PR (do NOT merge).**

```bash
gh pr create --base main --head feature/e2e-onmessage-compat \
  --title "feat(shims): onMessage compat for async listeners with sync sendResponse (#8)" \
  --body "Fixes #8. New guarded shim shims/runtime-onmessage-compat.js restores Chrome's precedence (a sendResponse called synchronously inside an async listener wins over the listener's implicit undefined promise). Added to background.scripts AND injected as the first <script> of every extension HTML page (new html_inject transformer step) so offscreen-document listeners are covered. offscreen-gate now passes with the four issue-#8 control allowances removed. OneNote re-measured: Firefox gets past the JSON.parse crash (before/after counts in the corpus notes). Both browsers headless."
```

Report the PR URL and the OneNote before/after Firefox event counts.

## Self-Review

- Spec coverage: issue #8 wrapper (background) -> Task 1; HTML-page injection so offscreen listeners are reached -> Task 2; behavioral proof + allowance removal + OneNote re-measure -> Task 3; spec update -> Task 3 Step 6. Covered.
- Placeholder scan: the `generate_shims`-inclusion unit test is conditionally deleted with an explicit fallback (Task 1 Step 2); PR body counts filled at Step 2. No TODO/TBD.
- Type consistency: `create_runtime_onmessage_compat` used in Task 1 Steps 2/4; `inject_onmessage_shim(html: &str, html_path: &Path) -> Option<String>` used in Task 2 Steps 2/4/6; `ModifiedFile`/`FileChange`/`ChangeType` per `offscreen_converter.rs`. The four issue-#8 allowance strings match `e2e/corpus.json` exactly.
- Risk: the wrapper must not double-respond. `wrappedSR` forwards to native `sendResponse` once; when suppressing the promise it returns `undefined` (no second response). A late (post-return) `sendResponse` leaves `respondedSync` false at the return check, so native `ret` (Promise/true) is returned unchanged. Verified by the offscreen-gate behavioral gate in Task 3.
