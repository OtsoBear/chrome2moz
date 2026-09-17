// Boot counter in storage.local (persists across background restarts on BOTH browsers,
// unlike storage.session, so the wake behavior is what is under test, not the session shim).
// Only deterministic values are persisted: a random per-boot marker (as used in the spike to
// prove a genuine restart) must NOT be stored, because it differs across browsers and would
// poison the trace diff (every storage.local.set would diverge). The restart evidence here is
// `boots` advancing after the kill, which the content script's storage.local.get carries
// into the trace identically on both sides.
async function recordBoot(reason) {
  const cur = await chrome.storage.local.get(["boots"]);
  const boots = (cur.boots || 0) + 1;
  await chrome.storage.local.set({ boots, lastWake: reason });
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
