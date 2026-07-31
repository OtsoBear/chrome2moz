chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg && msg.kind === "parse") {
    const doc = new DOMParser().parseFromString(msg.payload, "text/html");
    sendResponse({ text: doc.getElementById("x").textContent });
    return true;
  }
});

// Regression control for chrome2moz issue #8: a separate listener, same
// shape as OneNote Web Clipper's offscreen-relay listener -- an `async`
// function that calls sendResponse synchronously and never `return true`s
// (an async function always returns a Promise, so the explicit `true` is
// skipped, unlike the "parse" listener above). Isolated to its own message
// kind and storage key so it can't affect the already-passing "parse" case.
chrome.runtime.onMessage.addListener(async (msg, _sender, sendResponse) => {
  if (msg && msg.kind === "parse-async") {
    const doc = new DOMParser().parseFromString(msg.payload, "text/html");
    sendResponse({ text: doc.getElementById("x").textContent });
  }
});
