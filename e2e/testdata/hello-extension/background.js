chrome.runtime.onInstalled.addListener(() => {
  chrome.storage.local.set({ installed: true });
});
chrome.commands.onCommand.addListener((cmd) => {
  chrome.storage.local.set({ lastCommand: cmd });
});
