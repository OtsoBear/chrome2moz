async function viaOffscreen(payload) {
  const t = chrome.runtime, r = chrome.offscreen;
  const url = t.getURL("offscreen.html");
  const existing = await t.getContexts({ contextTypes: [t.ContextType.OFFSCREEN_DOCUMENT], documentUrls: [url] });
  if (existing.length === 0) {
    await r.createDocument({ url: "offscreen.html", reasons: [r.Reason.DOM_PARSER], justification: "parse" });
  }
  return chrome.runtime.sendMessage({ kind: "parse", payload });
}

chrome.runtime.onInstalled.addListener(() => {
  viaOffscreen("<p id='x'>hello offscreen</p>")
    .then((result) => chrome.storage.local.set({ offscreenResult: result }))
    .catch((e) => chrome.storage.local.set({ offscreenError: String(e) }));
});
