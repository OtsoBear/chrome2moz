# Chrome to Firefox Extension Converter

A powerful Rust-based CLI tool that automatically converts Chrome Manifest V3 extensions to Firefox-compatible format. Handles API conversions, manifest transformations, and generates compatibility shims with support for complex patterns like `executeScript` to message-passing conversion.

## Features

- **Automatic API Conversion**: Converts `chrome.*` namespace to `browser.*`
- **Expanded API Coverage**: 80+ Chrome API mappings including MV3 features
- **Manifest Transformation**: Adapts Chrome MV3 manifests for Firefox compatibility
- **Service Worker Handling**: Converts service workers to Firefox event pages
- **Advanced Transformations**: Automatically converts `executeScript` patterns to message-passing
- **Smart Analysis**: Detects 90+ types of incompatibilities
- **Intelligent Shims**: Auto-generates 10+ compatibility shims based on API usage
- **MV3 API Support**: Handles `storage.session`, `sidePanel`, `userScripts`, and more
- **Legacy API Support**: Maps deprecated Chrome APIs to modern equivalents
- **XPI Packaging**: Creates ready-to-install Firefox extension packages
- **Detailed Reports**: Comprehensive conversion reports with statistics
- **Batch Processing**: Handles multiple files efficiently

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/OtsoBear/chrome-to-firefox.git
cd chrome-to-firefox

# Build the project
cargo build --release
```

### Interactive Mode (Recommended for Beginners)

Simply run without arguments for a user-friendly interactive menu:

```bash
cargo run
```

Or with the release build:

```bash
./target/release/chrome-to-firefox
```

The interactive CLI guides you through all operations with menus and prompts.

### Command-Line Mode

For automation or if you prefer CLI arguments:

```bash
# Analyze your extension first
cargo run -- analyze -i ./path/to/chrome-extension

# Convert to Firefox format
cargo run -- convert -i ./path/to/chrome-extension -o ./output --report

# Check the results
cat output.md
```

### Try the Example

```bash
# Convert the included test extension
cargo run -- convert -i ./LatexToCalc -o ./converted-output --report

# Expected output:
# Conversion completed successfully!
# Files modified: 5
# Files added: 3 (compatibility shims)
# Total changes: 73
```

## What Gets Converted

### JavaScript Transformations

**API Namespace Conversion:**
```javascript
// Before
chrome.storage.sync.get("key", callback);
chrome.tabs.query({active: true}, callback);

// After
browser.storage.sync.get("key", callback);
browser.tabs.query({active: true}, callback);
```

**Browser Polyfill Injection:**
```javascript
// Added automatically to all JavaScript files
if (typeof browser === 'undefined') {
  var browser = chrome;
}
```

**executeScript to Message Passing** (Advanced):
```javascript
// Before (Chrome pattern)
chrome.scripting.executeScript({
    target: { tabId: activeTab.id },
    function: (reqId) => {
        const result = myFunction(reqId);  // Function from content script
        chrome.runtime.sendMessage({type: "RESULT", result});
    },
    args: [requestId]
});

// After (Firefox-compatible)
// In background.js:
browser.tabs.sendMessage(activeTab.id, {
    type: 'EXECUTE_SCRIPT_REQUEST_265',
    args: [requestId]
});

// In content.js (auto-generated listener):
browser.runtime.onMessage.addListener((request, sender, sendResponse) => {
    if (request.type === 'EXECUTE_SCRIPT_REQUEST_265') {
        const [reqId] = request.args;
        const result = myFunction(reqId);
        browser.runtime.sendMessage({type: "RESULT", result});
        return true;
    }
});
```

### Manifest Transformations

**Firefox-Specific Settings:**
```json
{
  "browser_specific_settings": {
    "gecko": {
      "id": "extension@converted.extension",
      "strict_min_version": "121.0"
    }
  }
}
```

**Background Scripts:**
```json
{
  "background": {
    "service_worker": "background.js",
    "scripts": ["background.js"],
    "persistent": false
  }
}
```

**Permission Restructuring:**
```json
{
  "permissions": ["storage", "tabs"],
  "host_permissions": ["https://example.com/*"]
}
```

## Usage

### Command Reference

#### Analyze Command
Inspect an extension without converting:

```bash
cargo run -- analyze -i ./extension-directory
```

Output shows:
- All detected incompatibilities
- Severity levels (Blocker, Major, Minor, Info)
- Auto-fixable issues
- Manual action items

#### Convert Command

Convert an extension to Firefox format:

```bash
cargo run -- convert -i ./chrome-extension -o ./firefox-version --report

Options:
  -i, --input <PATH>     Input Chrome extension directory
  -o, --output <PATH>    Output directory for Firefox version
  -r, --report           Generate detailed conversion report
```

#### Chrome-Only APIs Command

List WebExtension APIs that currently exist only in Chrome:

```bash
cargo run -- chrome-only-apis
```

This command reaches out to MDN's `browser-compat-data` repository using the GitHub
API (no clone required) and reports every feature where Chrome has support but Firefox
does not. Use it to quickly identify compatibility gaps before starting a port. The
report now highlights how many of the detected APIs already have shims or detection
logic in `src/parser/javascript.rs`, so you can spot remaining work at a glance.

Example summary section:

```text
Summary:
  Total Chrome-only APIs found: 42
  Implemented (matches parser/javascript.rs): 6
  Not yet implemented: 36
  Known chrome-only prefixes tracked: 14
  Known prefixes missing from MDN dataset: 2
    Missing prefixes:
      - chrome.downloads.acceptDanger
      - chrome.downloads.setShelfEnabled
```

### Output Structure

After conversion:

```
output/
├── manifest.json              # Transformed for Firefox
├── background.js              # chrome.* → browser.*
├── content.js                 # With auto-generated listeners
├── popup.js                   # Converted
└── [other extension files]

output.xpi                     # Ready-to-install Firefox package
output.md                      # Detailed conversion report
```

### Conversion Report

The report includes:

```markdown
## Summary
- Extension: LatexToCalc v2.0.1
- Conversion Status: ✅ Success
- Files Modified: 5
- Total Changes: 73
- Chrome API Calls Converted: 57
- Callback→Promise Conversions: 10

## Transformations
- background.js: 36 changes
  - ✓ Converted chrome → browser (8)
  - ✓ Converted executeScript to message passing (3)
  - ✓ Added browser polyfill
```

## Testing in Firefox

### Method 1: Temporary Add-on
1. Open Firefox
2. Go to `about:debugging#/runtime/this-firefox`
3. Click "Load Temporary Add-on"
4. Select `output/manifest.json`

### Method 2: Install XPI
1. Open Firefox
2. Go to `about:addons`
3. Click gear icon → "Install Add-on From File"
4. Select `output.xpi`

### Debugging
Check the Browser Console (Ctrl+Shift+J) for any errors.

## Key Transformations

### 1. Chrome API → Browser API
- All `chrome.*` calls converted to `browser.*`
- Maintains backward compatibility with Chrome

### 2. Manifest V3 → Firefox MV3
- Adds `browser_specific_settings.gecko.id`
- Converts service workers to event pages
- Restructures permissions

### 3. executeScript Isolation Handling
- Detects `scripting.executeScript` with function references
- Extracts function code and variables
- Generates message passing architecture
- Creates listeners in content scripts

### 4. declarativeNetRequest → webRequest Conversion
Chrome's declarative network request API is automatically converted to Firefox's imperative webRequest:

```javascript
// Chrome DNR rule (declarative)
chrome.declarativeNetRequest.updateDynamicRules({
    addRules: [{
        id: 1,
        priority: 1,
        action: { type: 'block' },
        condition: {
            urlFilter: '||ads.example.com/*',
            resourceTypes: ['script', 'image']
        }
    }]
});

// Converted to Firefox webRequest (imperative)
browser.webRequest.onBeforeRequest.addListener(
    (details) => {
        if (matchesCondition(details)) {
            return { cancel: true };
        }
    },
    { urls: ['*://ads.example.com/*'], types: ['script', 'image'] },
    ['blocking']
);
```

**Supported DNR Actions:**
- **Block**: Converts to `onBeforeRequest` with `{cancel: true}`
- **Redirect**: Converts to `onBeforeRequest` with `{redirectUrl: newUrl}`
  - Supports URL rewrites, regex substitution, URL transformations
- **ModifyHeaders**: Converts to `onBeforeSendHeaders`/`onHeadersReceived`
  - Supports request and response header modifications
- **UpgradeScheme**: HTTP→HTTPS via redirect

**Rule Features:**
- Domain and initiator filtering
- Resource type filtering
- URL pattern conversion
- Dynamic and session rule support
- Debug events emulation

### 5. Compatibility Shims
Generated shims provide extensive cross-browser support:

**Core Shims:**
- `browser-polyfill.js`: Namespace compatibility
- `action-compat.js`: Action API bridging
- `promise-wrapper.js`: Callback-to-promise helpers

**MV3 API Shims:**
- `storage-session-compat.js`: Native in Firefox 115+, in-memory polyfill for older versions
- `sidepanel-compat.js`: Maps `sidePanel` to Firefox's `sidebarAction`
- `declarative-net-request-stub.js`: Converts DNR rules to `webRequest` listeners automatically
- `user-scripts-compat.js`: Translates `userScripts` API

**Legacy API Shims:**
- `tabs-windows-compat.js`: Maps deprecated `tabs.getSelected`, `tabs.getAllInWindow`
- `runtime-compat.js`: Stubs `runtime.getPackageDirectoryEntry`

**Optional Shims:**
- `downloads-compat.js`: Handles Chrome-specific download features
- `privacy-stub.js`: Stubs `chrome.privacy` API
- `notifications-compat.js`: Adapts notification options for Firefox

## ⚠️ Known Limitations

### Module System Detection
The converter currently does not differentiate between ES modules, CommonJS, and browser globals:

**Current Behavior:**
- All JavaScript transformations apply uniformly regardless of module type
- `chrome.*` → `browser.*` conversions work across all module systems
- Browser polyfill injection assumes global scope

**Impact on Different Extension Types:**
- ✅ **Browser Globals (Most Extensions)**: Works perfectly - this is the most common pattern
- ✅ **Traditional Content/Background Scripts**: Full support
- ⚠️ **ES Modules (Manifest V3)**: Transformations work, but polyfill placement may need adjustment
- ⚠️ **CommonJS Modules**: Rare in extensions; may need manual review

**Known Issues:**
- ES module `import`/`export` statements are preserved but not analyzed
- `importScripts()` handling (service workers) works but lacks module-awareness
- Polyfill injection may conflict with ES module imports

**Recommended Workflow:**
1. For extensions using **browser globals** (90%+ of cases): Use as-is
2. For extensions with **ES modules**: Review generated polyfill placement
3. For complex module setups: Use the analyzer first to identify patterns

**Future Enhancement:**
Module type detection is planned for improved handling of:
- ES module import/export transformations
- Context-aware polyfill injection
- Better service worker vs event page conversion

For now, the tool handles typical Chrome extensions (browser globals) very well, which covers the vast majority of use cases.

### Chrome-Only APIs
Some Chrome features have no or limited Firefox equivalent:

**Not Available (Blockers):**
- `chrome.offscreen.*` - No equivalent
- `chrome.declarativeContent.*` - No equivalent
- `chrome.tabGroups.*` - No equivalent

**Partial Support (Shims Provided):**
- `chrome.sidePanel.*` - Maps to Firefox's `sidebarAction` (different UI)
- `chrome.declarativeNetRequest.*` - Automatically converted to `webRequest` listeners at runtime
  - Supports: block, redirect, modifyHeaders, upgradeScheme rules
  - Converts dynamic/session rules to imperative callbacks
  - Note: Static rulesets and allow rules have limited support
- `chrome.userScripts.*` - Maps to Firefox's different API structure
- `chrome.storage.session` - In-memory polyfill (data not persisted)
- `chrome.privacy.*` - Stubbed as read-only (use Firefox preferences)

**Legacy APIs (Shims Provided):**
- `tabs.getSelected` / `tabs.getAllInWindow` - Mapped to modern `tabs.query`
- `runtime.getPackageDirectoryEntry` - Stubbed with guidance

The tool automatically applies shims where possible and flags blockers in the report.

### Service Workers vs Event Pages
Chrome uses service workers, Firefox uses event pages:
- Different lifecycle management
- No `importScripts()` in Firefox
- Converted to `background.scripts` array

### Host Permissions
Firefox treats `host_permissions` as optional (user can deny), while Chrome grants them at install time.

## Development

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Code quality
cargo fmt
cargo clippy
```

### Project Structure

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library root
├── models/              # Data structures
├── parser/              # Manifest & JS parsing
├── analyzer/            # Incompatibility detection
├── transformer/         # Code transformation
│   ├── manifest.rs      # Manifest transformer
│   └── javascript.rs    # JS transformer (executeScript conversion)
├── packager/            # XPI packaging
├── validator/           # Output validation
└── report/             # Report generation
```

### Running with Cargo

```bash
# Analyze
cargo run -- analyze -i ./LatexToCalc

# Convert
cargo run -- convert -i ./LatexToCalc -o ./output --report

# Use release build (faster)
./target/release/chrome-to-firefox convert -i ./extension -o ./output
```

## Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Technical architecture and implementation details
- **Conversion Reports** - Generated with `--report` flag
- **In-code Documentation** - Run `cargo doc --open`

## Contributing

Contributions are welcome! Areas for contribution:

- Additional API mappings
- More test cases
- Documentation improvements
- Bug fixes
- New features

See the LatexToCalc extension as a reference for testing.

## Troubleshooting

### Build Errors
```bash
cargo clean
cargo build --release
```

### Conversion Issues
1. Check the error message
2. Review the conversion report
3. Ensure input is valid Chrome MV3
4. Check file permissions

### Extension Doesn't Work
1. Open Browser Console (Ctrl+Shift+J)
2. Check for JavaScript errors
3. Review manifest in `about:debugging`
4. Check permissions are granted

## License

This project is licensed under the MIT License.

## Acknowledgments

- Built with Rust
- Test extension: [LatexToCalc](LatexToCalc/)
- Inspired by the need for cross-browser extension compatibility

## Support

- [Report a bug](https://github.com/OtsoBear/chrome-to-firefox/issues)
- [Request a feature](https://github.com/OtsoBear/chrome-to-firefox/issues)
- [Read the architecture docs](./ARCHITECTURE.md)

---

**Status**: Production-ready  
**Version**: 0.1.0  
**Last Updated**: October 2025

Made with care for the open web
