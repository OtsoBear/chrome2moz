// Inert service worker. The clipboard round-trip (Task 1's observable) happens entirely in
// content.js; this file exists only so the harness's Chrome driver can detect the extension's
// id via the service-worker event (see e2e/src/chromeDriver.ts), matching every other local
// testdata extension in this corpus. It makes no chrome.* calls, so it produces no trace events
// and does not affect the clipboard-gate's empty allowed_diffs.
