// Read the boot state on every fixture page load; the spy shim records
// storage.local.get:resolve [{boots, lastWake, ...}] on both browsers, so the diff compares
// how the background rebooted after the kill.
chrome.storage.local.get(["boots", "lastWake"]).then(() => {});
