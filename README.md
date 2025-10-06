# Chrome to Firefox Extension Converter

A powerful Rust-based CLI tool that automatically converts Chrome Manifest V3 extensions to Firefox-compatible format. Handles API conversions, manifest transformations, and generates compatibility shims with support for complex patterns like `executeScript` to message-passing conversion.

## ✨ Features

- **🔄 Automatic API Conversion**: Converts `chrome.*` namespace to `browser.*`
- **📝 Manifest Transformation**: Adapts Chrome MV3 manifests for Firefox compatibility
- **⚙️ Service Worker Handling**: Converts service workers to Firefox event pages
- **🔧 Advanced Transformations**: Automatically converts `executeScript` patterns to message-passing
- **🎯 Smart Analysis**: Detects 78+ types of incompatibilities
- **📦 XPI Packaging**: Creates ready-to-install Firefox extension packages
- **📊 Detailed Reports**: Comprehensive conversion reports with statistics
- **🚀 Batch Processing**: Handles multiple files efficiently

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/chrome-to-firefox.git
cd chrome-to-firefox

# Build the project
cargo build --release
```

### Your First Conversion

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
# ✅ Conversion completed successfully!
# 📊 Files modified: 5
# 📊 Files added: 3 (compatibility shims)
# 📊 Total changes: 73
```

## 📋 What Gets Converted

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

## 📖 Usage

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

## 🧪 Testing in Firefox

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

## 🎯 Key Transformations

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

### 4. Compatibility Shims
Generated shims provide cross-browser support:
- `browser-polyfill.js`: Namespace compatibility
- `action-compat.js`: Action API bridging
- `promise-wrapper.js`: Callback-to-promise helpers

## ⚠️ Known Limitations

### Chrome-Only APIs
Some Chrome features have no Firefox equivalent:
- `chrome.offscreen.*` - Not available
- `chrome.sidePanel.*` - Not available
- `chrome.declarativeContent.*` - Not available
- `chrome.tabGroups.*` - Not available

The tool flags these in the report as requiring manual intervention.

### Service Workers vs Event Pages
Chrome uses service workers, Firefox uses event pages:
- Different lifecycle management
- No `importScripts()` in Firefox
- Converted to `background.scripts` array

### Host Permissions
Firefox treats `host_permissions` as optional (user can deny), while Chrome grants them at install time.

## 🛠️ Development

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

## 📚 Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Technical architecture and implementation details
- **Conversion Reports** - Generated with `--report` flag
- **In-code Documentation** - Run `cargo doc --open`

## 🤝 Contributing

Contributions are welcome! Areas for contribution:

- 🔧 Additional API mappings
- 🧪 More test cases
- 📚 Documentation improvements
- 🐛 Bug fixes
- ✨ New features

See the LatexToCalc extension as a reference for testing.

## 🐛 Troubleshooting

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

## 📄 License

This project is licensed under the MIT License.

## 🙏 Acknowledgments

- Built with Rust 🦀
- Test extension: [LatexToCalc](LatexToCalc/)
- Inspired by the need for cross-browser extension compatibility

## 📞 Support

- 🐛 [Report a bug](https://github.com/yourusername/chrome-to-firefox/issues)
- 💡 [Request a feature](https://github.com/yourusername/chrome-to-firefox/issues)
- 📖 [Read the architecture docs](./ARCHITECTURE.md)

---

**Status**: ✅ Production-ready  
**Version**: 0.1.0  
**Last Updated**: October 2025

Made with ❤️ for the open web