chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg && msg.kind === "parse") {
    const doc = new DOMParser().parseFromString(msg.payload, "text/html");
    sendResponse({ text: doc.getElementById("x").textContent });
    return true;
  }
});
