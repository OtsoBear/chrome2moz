// Boot counter in storage.local (persists across background restarts on BOTH browsers,
// unlike storage.session, so the wake behavior is what is under test, not the session shim).
// bootMark is computed ONCE, synchronously, at top level: it can only change when the whole
// script re-executes in a fresh global scope (a genuine restart), unlike `boots`, which
// increments on every recordBoot() call regardless of restart. Both signals reach the trace
// via the content script's storage.local.get.
const bootMark = Date.now() + ":" + Math.random();

async function recordBoot(reason) {
  const cur = await chrome.storage.local.get(["boots"]);
  const boots = (cur.boots || 0) + 1;
  await chrome.storage.local.set({ boots, bootMark, lastWake: reason });
}

recordBoot("startup");

// Filtered to real navigations only: a new tab starts at about:blank before navigating to its
// real target, and Chromium's Playwright-driven ctx.newPage() fires tabs.onUpdated for THAT
// intermediate about:blank load in a way Firefox's WebDriver-driven tab-open does not mirror
// 1:1 -- an asymmetry in browser-native tab-open mechanics that has nothing to do with the
// kill/wake mechanism under test. Excluding about:blank keeps both sides' boot counts driven
// only by real fixture-page navigations, which both browsers' probes issue identically.
chrome.tabs.onUpdated.addListener((_tabId, info) => {
  if (info.status === "complete" && info.url && !info.url.startsWith("about:")) {
    recordBoot("tabs.onUpdated");
  }
});
