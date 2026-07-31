async function viaOffscreen(payload, kind) {
  const t = chrome.runtime, r = chrome.offscreen;
  const url = t.getURL("offscreen.html");
  const existing = await t.getContexts({ contextTypes: [t.ContextType.OFFSCREEN_DOCUMENT], documentUrls: [url] });
  if (existing.length === 0) {
    await r.createDocument({ url: "offscreen.html", reasons: [r.Reason.DOM_PARSER], justification: "parse" });
  }
  return chrome.runtime.sendMessage({ kind, payload });
}

chrome.runtime.onInstalled.addListener(async () => {
  try {
    const result = await viaOffscreen("<p id='x'>hello offscreen</p>", "parse");
    await chrome.storage.local.set({ offscreenResult: result });
  } catch (e) {
    await chrome.storage.local.set({ offscreenError: String(e) });
  }

  // Regression control for issue #8 -- see offscreen.js's second listener.
  // Sequenced strictly after the "parse" round-trip above (the offscreen
  // document is a singleton -- a second createDocument() call while one
  // already exists rejects, which would otherwise race both round-trips
  // together instead of isolating this one). If the async-listener/
  // sendResponse discard theory is right, Chrome stores the real parsed
  // text and Firefox stores the UNDEFINED_RESPONSE marker (the sendMessage
  // promise resolves to undefined instead of rejecting, so this must be
  // distinguished explicitly rather than relying on catch()).
  try {
    const result = await viaOffscreen("<p id='x'>hello async offscreen</p>", "parse-async");
    await chrome.storage.local.set({
      offscreenAsyncResult: result === undefined ? { UNDEFINED_RESPONSE: true } : result,
    });
  } catch (e) {
    await chrome.storage.local.set({ offscreenAsyncError: String(e) });
  }

  // Probe 3: exercises the polyfill's callback-form support on
  // offscreen.hasDocument/closeDocument (callback-to-Promise helper below),
  // then recreates the document and repeats the parse round-trip via the
  // same Promise-form path probe 1 already uses -- proving a close +
  // recreate cycle driven through the callback API still works.
  try {
    const hadBefore = await callbackToPromise(chrome.offscreen.hasDocument.bind(chrome.offscreen));
    await callbackToPromise(chrome.offscreen.closeDocument.bind(chrome.offscreen));
    const hadAfterClose = await callbackToPromise(chrome.offscreen.hasDocument.bind(chrome.offscreen));
    const result = await viaOffscreen("<p id='x'>hello callback offscreen</p>", "parse");
    await chrome.storage.local.set({ offscreenCallbackResult: { hadBefore, hadAfterClose, result } });
  } catch (e) {
    await chrome.storage.local.set({ offscreenCallbackError: String(e) });
  }
});

function callbackToPromise(fn, ...args) {
  return new Promise((resolve) => fn(...args, (result) => resolve(result)));
}
