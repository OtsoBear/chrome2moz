# Chrome ↔ Firefox WebExtension API Mappings

This document details the differences between Chrome and Firefox WebExtension APIs, focusing on MV3 compatibility.

## API Namespace

| Chrome | Firefox | Notes |
|--------|---------|-------|
| `chrome.*` | `browser.*` | Firefox supports both, Chrome only supports `chrome.*` |
| Callbacks | Promises | Firefox uses promises by default, Chrome MV3 adds promise support |

## Background Context

| Chrome MV3 | Firefox MV3 | Conversion |
|------------|-------------|------------|
| `background.service_worker` | `background.scripts` | Keep both keys; Firefox uses scripts, Chrome uses service_worker |
| Service Worker context | Event page context | Refactor long-running operations to use alarms |
| `importScripts()` | Scripts array | Convert `importScripts('a.js', 'b.js')` to `"scripts": ["a.js", "b.js"]` |

### Background Conversion Examples

**Chrome (Service Worker):**
```javascript
// background.js
self.addEventListener('install', () => {
  console.log('Service worker installed');
});

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  // Handle message
});
```

**Firefox (Event Page):**
```javascript
// background.js
// Top-level listeners
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  // Handle message
});

// No install event in event pages
```

## Manifest Keys

### background

**Chrome MV3:**
```json
{
  "background": {
    "service_worker": "background.js"
  }
}
```

**Firefox MV3:**
```json
{
  "background": {
    "scripts": ["background.js"]
  }
}
```

**Cross-browser:**
```json
{
  "background": {
    "service_worker": "background.js",
    "scripts": ["background.js"]
  }
}
```

### browser_specific_settings

**Required for Firefox:**
```json
{
  "browser_specific_settings": {
    "gecko": {
      "id": "extension@domain.com",
      "strict_min_version": "121.0"
    }
  }
}
```

### content_security_policy

**Chrome MV3:**
```json
{
  "content_security_policy": {
    "extension_pages": "script-src 'self'; object-src 'self'"
  }
}
```

**Firefox MV3 with WASM:**
```json
{
  "content_security_policy": {
    "extension_pages": "script-src 'self' 'wasm-unsafe-eval'; object-src 'self'"
  }
}
```

### host_permissions

**MV2 (old):**
```json
{
  "permissions": ["https://example.com/*"]
}
```

**MV3 (new):**
```json
{
  "permissions": ["storage"],
  "host_permissions": ["https://example.com/*"]
}
```

**Note:** Firefox treats `host_permissions` as optional (user can grant/deny), Chrome grants at install.

### action / browser_action

**Chrome MV3:**
```json
{
  "action": {
    "default_popup": "popup.html",
    "default_icon": "icon.png"
  }
}
```

**Firefox MV3:**
```json
{
  "action": {
    "default_popup": "popup.html",
    "default_icon": "icon.png"
  }
}
```

**Note:** Remove `browser_style` property in MV3 (no longer supported).

### web_accessible_resources

**Chrome MV3:**
```json
{
  "web_accessible_resources": [{
    "resources": ["images/*"],
    "matches": ["https://example.com/*"],
    "use_dynamic_url": true
  }]
}
```

**Firefox MV3:**
```json
{
  "web_accessible_resources": [{
    "resources": ["images/*"],
    "matches": ["https://example.com/*"]
  }]
}
```

**Note:** Firefox does not support `use_dynamic_url`. Must use `matches` or `extension_ids`.

## JavaScript APIs

### Storage API

**Chrome (Callback):**
```javascript
chrome.storage.local.get('key', (result) => {
  if (chrome.runtime.lastError) {
    console.error(chrome.runtime.lastError);
  } else {
    console.log(result);
  }
});
```

**Firefox (Promise):**
```javascript
browser.storage.local.get('key')
  .then(result => console.log(result))
  .catch(error => console.error(error));
```

**Chrome MV3 (Promise support added):**
```javascript
chrome.storage.local.get('key')
  .then(result => console.log(result))
  .catch(error => console.error(error));
```

### Scripting API

**Chrome MV2 (deprecated):**
```javascript
chrome.tabs.executeScript(tabId, {
  code: 'document.body.style.background = "red"'
});
```

**Chrome/Firefox MV3:**
```javascript
chrome.scripting.executeScript({
  target: { tabId: tabId },
  func: () => { document.body.style.background = "red"; }
});
```

**Note:** Cannot use code strings, must use `func` or `files`.

### WebRequest API

| Feature | Chrome MV3 | Firefox MV3 | Notes |
|---------|------------|-------------|-------|
| Blocking webRequest | ❌ Removed | ✅ Supported | Firefox still allows blocking |
| declarativeNetRequest | ✅ Required | ✅ Supported | Use for cross-browser |

**Chrome MV3 (DNR only):**
```javascript
chrome.declarativeNetRequest.updateDynamicRules({
  addRules: [{
    id: 1,
    priority: 1,
    action: { type: 'block' },
    condition: { urlFilter: 'example.com' }
  }]
});
```

**Firefox MV3 (Both supported):**
```javascript
// Option 1: Keep blocking webRequest
chrome.webRequest.onBeforeRequest.addListener(
  (details) => { return { cancel: true }; },
  { urls: ["*://example.com/*"] },
  ["blocking"]
);

// Option 2: Use DNR for cross-browser
chrome.declarativeNetRequest.updateDynamicRules({
  addRules: [{
    id: 1,
    priority: 1,
    action: { type: 'block' },
    condition: { urlFilter: 'example.com' }
  }]
});
```

### Notifications API

**Chrome:**
```javascript
chrome.notifications.create({
  type: 'basic',
  iconUrl: 'icon.png', // Required in Chrome
  title: 'Title',
  message: 'Message'
});
```

**Firefox:**
```javascript
browser.notifications.create({
  type: 'basic',
  iconUrl: 'icon.png', // Optional in Firefox
  title: 'Title',
  message: 'Message'
});
```

### Offscreen API (Chrome only)

**Chrome MV3:**
```javascript
chrome.offscreen.createDocument({
  url: 'offscreen.html',
  reasons: ['DOM_SCRAPING'],
  justification: 'Parse DOM data'
});
```

**Firefox MV3:**
❌ Not supported - must use alternative approaches:
- Event page with hidden window
- Visible popup/tab
- Refactor to not need DOM

### Proxy API

**Chrome:**
```javascript
chrome.proxy.settings.set({
  value: {
    mode: 'fixed_servers',
    rules: { /* ... */ }
  }
});
```

**Firefox:**
```javascript
browser.proxy.settings.set({
  value: {
    proxyType: 'manual',
    http: 'proxy.example.com:8080'
  }
});
```

**Note:** Completely different API structure between browsers.

### Sidebar API (Firefox only)

**Firefox:**
```json
{
  "sidebar_action": {
    "default_panel": "sidebar.html",
    "default_icon": "icon.png"
  }
}
```

**Chrome:**
Uses different `chrome.sidePanel` API (Chrome 114+). Not directly compatible.

### Tabs API

#### executeScript differences

**Firefox:**
```javascript
// Relative URLs resolved relative to current page
browser.tabs.executeScript(tabId, {
  file: '/script.js'
});
```

**Chrome:**
```javascript
// Relative URLs resolved relative to extension root
chrome.tabs.executeScript(tabId, {
  file: '/script.js'
});
```

#### remove() behavior

**Firefox:**
```javascript
// Promise fulfills after beforeunload
await browser.tabs.remove(tabId);
```

**Chrome:**
```javascript
// Callback doesn't wait for beforeunload
chrome.tabs.remove(tabId, () => {
  // Tab may still be closing
});
```

### Windows API

**Firefox:**
```javascript
// onFocusChanged triggers multiple times
browser.windows.onFocusChanged.addListener((windowId) => {
  // May fire multiple times for single focus change
});
```

**Chrome:**
```javascript
// onFocusChanged triggers once
chrome.windows.onFocusChanged.addListener((windowId) => {
  // Fires once per focus change
});
```

## Chrome-only APIs (Not in Firefox)

| API | Chrome | Firefox Alternative |
|-----|--------|---------------------|
| `chrome.offscreen` | ✅ | Use event page or visible page |
| `chrome.declarativeContent` | ✅ | Not available (rarely used) |
| `chrome.sidePanel` | ✅ Chrome 114+ | Use `sidebar_action` (different API) |
| `chrome.tabGroups` | ✅ | Not available |

## Firefox-only APIs (Not in Chrome)

| API | Firefox | Chrome Alternative |
|-----|---------|-------------------|
| `browser.proxy` (Firefox implementation) | ✅ | Different API structure |
| `browser.theme` | ✅ | Not available |
| `browser.contextualIdentities` | ✅ (containers) | Not available |
| `browser.pkcs11` | ✅ | Not available |

## Unsupported APIs in Both Browsers

| API | Status |
|-----|--------|
| `chrome.types` | Deprecated |
| `chrome.app.*` | Removed in MV3 |
| `chrome.csi()` | Removed |
| `chrome.loadTimes()` | Removed |

## Conversion Patterns

### Pattern 1: Chrome namespace to browser

**Before:**
```javascript
chrome.storage.local.get('key', callback);
```

**After (with polyfill):**
```javascript
// Add at top of file or in shim
if (typeof browser === 'undefined') {
  var browser = chrome;
}

browser.storage.local.get('key', callback);
```

### Pattern 2: Callback to Promise

**Before:**
```javascript
chrome.tabs.query({active: true}, (tabs) => {
  if (chrome.runtime.lastError) {
    console.error(chrome.runtime.lastError);
    return;
  }
  console.log(tabs[0]);
});
```

**After:**
```javascript
browser.tabs.query({active: true})
  .then(tabs => console.log(tabs[0]))
  .catch(error => console.error(error));
```

### Pattern 3: Service Worker to Event Page

**Before (Service Worker):**
```javascript
// background.js
self.addEventListener('install', () => {
  console.log('Installed');
});

const intervalId = setInterval(() => {
  // Long-running task
}, 60000);

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  // Handler
});
```

**After (Event Page):**
```javascript
// background.js
// Remove install listener (not needed)

// Replace setInterval with alarms
chrome.alarms.create('periodicTask', { periodInMinutes: 1 });

chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === 'periodicTask') {
    // Task
  }
});

// Keep listeners at top level
chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  // Handler
});
```

### Pattern 4: importScripts conversion

**Before (Service Worker):**
```javascript
// background.js
importScripts('lib1.js', 'lib2.js', 'config.js');

// Rest of code
```

**After (Manifest + Event Page):**
```json
{
  "background": {
    "scripts": ["lib1.js", "lib2.js", "config.js", "background.js"]
  }
}
```

### Pattern 5: Web Accessible Resources

**Before:**
```json
{
  "web_accessible_resources": [{
    "resources": ["content/*.js"],
    "matches": ["<all_urls>"],
    "use_dynamic_url": true
  }]
}
```

**After:**
```json
{
  "web_accessible_resources": [{
    "resources": ["content/*.js"],
    "matches": ["https://example.com/*", "https://another.com/*"]
  }]
}
```

## Compatibility Shims

### Universal Browser Namespace
```javascript
// browser-polyfill-lite.js
if (typeof browser === 'undefined') {
  window.browser = window.chrome;
}
```

### Promise Wrapper
```javascript
// promise-wrapper.js
function promisifyChrome(fn) {
  return function(...args) {
    return new Promise((resolve, reject) => {
      fn(...args, (...results) => {
        if (chrome.runtime.lastError) {
          reject(chrome.runtime.lastError);
        } else {
          resolve(results.length === 1 ? results[0] : results);
        }
      });
    });
  };
}
```

### Action API Compatibility
```javascript
// action-compat.js
const browserAction = chrome.action || chrome.browserAction;
```

### Storage Session Compatibility
```javascript
// storage-session-compat.js
const storageArea = chrome.storage.session || chrome.storage.local;
```

## Version Requirements

| Feature | Chrome Version | Firefox Version |
|---------|----------------|-----------------|
| Manifest V3 | Chrome 88+ | Firefox 109+ |
| Promises in chrome.* | Chrome 90+ (partial) | Firefox 52+ |
| storage.session | Chrome 102+ | Firefox 115+ |
| Scripting API | Chrome 88+ | Firefox 102+ |
| declarativeNetRequest | Chrome 84+ | Firefox 113+ |
| Side Panel API | Chrome 114+ | N/A |
| Offscreen API | Chrome 109+ | N/A |

## Testing Compatibility

### Feature Detection Pattern
```javascript
// Check if API exists before using
if (chrome.offscreen) {
  // Chrome-specific path
  chrome.offscreen.createDocument({...});
} else {
  // Firefox fallback
  // Use alternative approach
}
```

### Version Detection
```javascript
// Get browser info
browser.runtime.getBrowserInfo()
  .then(info => {
    console.log(info.name); // "Firefox"
    console.log(info.version); // "121.0"
  });
```

### Namespace Detection
```javascript
const isFirefox = typeof browser !== 'undefined' && browser.runtime;
const isChrome = typeof chrome !== 'undefined' && chrome.runtime;
```

## Common Migration Issues

### Issue 1: Content Script URLs
**Problem:** Firefox resolves relative URLs differently than Chrome.
**Solution:** Always use absolute paths from extension root with leading `/`.

### Issue 2: Host Permissions UX
**Problem:** Firefox prompts for host_permissions, Chrome grants at install.
**Solution:** Implement `browser.permissions.request()` flow in extension.

### Issue 3: Service Worker Context
**Problem:** Service workers have different lifecycle than event pages.
**Solution:** Refactor to use alarms, avoid long-running tasks, keep listeners at top level.

### Issue 4: Clipboard Access
**Problem:** Different security contexts between browsers.
**Solution:** Use content scripts for clipboard operations, not background scripts.

### Issue 5: Web Accessible Resources
**Problem:** Firefox doesn't support `use_dynamic_url`.
**Solution:** Explicitly list all `matches` or `extension_ids`.

## Best Practices for Cross-Browser Extensions

1. **Use browser namespace:** Always use `browser.*` with polyfill for Chrome
2. **Prefer promises:** Use promise-based APIs instead of callbacks
3. **Feature detection:** Check for API existence before using
4. **Test both browsers:** Don't assume compatibility without testing
5. **Follow MV3 strictly:** Avoid MV2 patterns even if still supported
6. **Document limitations:** Be clear about browser-specific features
7. **Use webextension-polyfill:** Consider using Mozilla's official polyfill
8. **Version targeting:** Set appropriate minimum versions in manifest

## Resources

- [MDN WebExtensions API](https://developer.mozilla.org/docs/Mozilla/Add-ons/WebExtensions/API)
- [Chrome Extensions API](https://developer.chrome.com/docs/extensions/reference/)
- [Firefox Extension Workshop](https://extensionworkshop.com/)
- [WebExtension Polyfill](https://github.com/mozilla/webextension-polyfill)