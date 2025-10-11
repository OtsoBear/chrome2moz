# Chrome to Firefox Extension Converter

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![WebAssembly](https://img.shields.io/badge/WebAssembly-supported-purple.svg)](https://webassembly.org/)

A powerful Rust-based tool that automatically converts Chrome Manifest V3 extensions to Firefox-compatible format using AST-based parsing for maximum accuracy. Handles API conversions, manifest transformations, and generates compatibility shims with full TypeScript support.

**Live Demo**: [https://otsobear.github.io/chrome2moz/](https://otsobear.github.io/chrome2moz/)

## Features

- **AST-Based Parsing**: Uses SWC for accurate, semantic code transformations
- **Full TypeScript Support**: Native `.ts`, `.tsx`, and `.d.ts` file handling with automatic type stripping
- **Automatic API Conversion**: Converts `chrome.*` namespace to `browser.*` with scope awareness
- **Module System Detection**: Auto-detects ES modules, CommonJS, and browser globals
- **Smart Polyfill Injection**: Context-aware polyfill injection based on module type
- **Expanded API Coverage**: 80+ Chrome API mappings including MV3 features
- **Manifest Transformation**: Adapts Chrome MV3 manifests for Firefox compatibility
- **Service Worker Handling**: Converts service workers to Firefox event pages
- **Scope-Aware Transformations**: Distinguishes local variables from global Chrome APIs
- **Smart Analysis**: Detects 90+ types of incompatibilities
- **Intelligent Shims**: Auto-generates 10+ compatibility shims based on API usage
- **WebAssembly UI**: Browser-based interface requiring no installation
- **XPI Packaging**: Creates ready-to-install Firefox extension packages
- **Detailed Reports**: Comprehensive conversion reports with statistics

## Quick Start

### WebAssembly UI (Recommended)

The easiest way to use the converter is through our WebAssembly-powered web interface:

```bash
# Build the WebAssembly module
./build-wasm.sh

# Serve the web UI (choose one)
cd web && python3 -m http.server 8080
# OR
npx http-server web -p 8080

# Open http://localhost:8080 in your browser
```

**Features:**
- Drag & drop Chrome extension ZIP files
- Instant compatibility analysis
- One-click conversion
- Direct download of Firefox-compatible extensions
- Runs entirely in your browser (no server needed)
- Choose output format (.xpi or .zip)

### Command-Line Installation

For automation or CLI usage:

```bash
# Clone the repository
git clone https://github.com/OtsoBear/chrome2moz.git
cd chrome2moz

# Build the project
cargo build --release
```

### Interactive CLI Mode

Run without arguments for a user-friendly interactive menu:

```bash
cargo run
# Or with release build
./target/release/chrome2moz
```

### Command-Line Mode

```bash
# Analyze your extension
cargo run -- analyze -i ./path/to/chrome-extension

# Convert to Firefox format
cargo run -- convert -i ./path/to/chrome-extension -o ./output --report

# Check the results
cat output.md
```

## Installation

### Prerequisites

**For CLI:**
- Rust 1.70 or later
- Cargo (included with Rust)

**For WebAssembly UI:**
- wasm-pack: `curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh`

### Building from Source

```bash
# Clone repository
git clone https://github.com/OtsoBear/chrome2moz.git
cd chrome2moz

# Build CLI
cargo build --release

# Build WebAssembly UI
./build-wasm.sh
```

## WebAssembly UI

### Building

```bash
# Install wasm-pack (one-time)
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build WASM module
./build-wasm.sh
```

### Serving Locally

Choose one method:

```bash
# Python
cd web && python3 -m http.server 8080

# Node.js
npx http-server web -p 8080 -c-1

# Rust miniserve
cargo install miniserve
miniserve web -p 8080
```

Then open `http://localhost:8080`

### Architecture

The WebAssembly UI consists of:

1. **Rust Backend** (`src/wasm.rs`) - Compiled to WebAssembly
2. **JavaScript Frontend** (`web/app.js`) - Handles UI and file operations
3. **HTML/CSS** (`web/index.html`, `web/styles.css`) - Modern glassmorphic interface

### Browser Compatibility

- Chrome/Edge 57+
- Firefox 52+
- Safari 11+
- Opera 44+

Requires WebAssembly support and ES6+.

## Deployment

### GitHub Pages

The repository includes automated deployment via GitHub Actions.

#### Setup Steps

1. **Enable GitHub Pages**:
   - Go to Settings → Pages
   - Set Source to "GitHub Actions"

2. **Configure Permissions**:
   - Go to Settings → Actions → General
   - Set Workflow permissions to "Read and write permissions"

3. **Deploy**:
   - Push to `main` branch (automatic)
   - Or manually trigger via Actions tab

Your site will be available at: `https://<username>.github.io/<repository>/`

#### Manual Deployment

```bash
# Build WASM
./build-wasm.sh

# Create deployment directory
mkdir -p deploy
cp -r web/* deploy/
touch deploy/.nojekyll

# Deploy using your preferred method
```

## Usage

### Command Reference

#### Analyze Command

```bash
cargo run -- analyze -i ./extension-directory
```

Shows:
- Detected incompatibilities
- Severity levels (Blocker, Major, Minor, Info)
- Auto-fixable issues
- Manual action items

#### Convert Command

```bash
cargo run -- convert -i ./chrome-extension -o ./firefox-version --report

Options:
  -i, --input <PATH>     Input Chrome extension directory
  -o, --output <PATH>    Output directory for Firefox version
  -r, --report           Generate detailed conversion report
```

#### Chrome-Only APIs Command

```bash
cargo run -- chrome-only-apis
```

Lists WebExtension APIs that exist only in Chrome using MDN's browser-compat-data.

### Output Structure

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

## What Gets Converted

### JavaScript Transformations

**API Namespace:**
```javascript
// Before
chrome.storage.sync.get("key", callback);

// After
browser.storage.sync.get("key", callback);
```

**Browser Polyfill:**
```javascript
if (typeof browser === 'undefined') {
  var browser = chrome;
}
```

**executeScript Fix:**
```javascript
// Chrome uses 'function'
chrome.scripting.executeScript({
    target: { tabId: id },
    function: () => { /* code */ }
});

// Firefox uses 'func'
browser.scripting.executeScript({
    target: { tabId: id },
    func: () => { /* code */ }
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

### Compatibility Shims

Generated shims include:

**Core Shims:**
- `browser-polyfill.js` - Namespace compatibility
- `action-compat.js` - Action API bridging
- `promise-wrapper.js` - Callback-to-promise helpers

**MV3 API Shims:**
- `storage-session-compat.js` - Session storage polyfill
- `sidepanel-compat.js` - Maps sidePanel to sidebarAction
- `declarative-net-request-stub.js` - DNR to webRequest conversion
- `user-scripts-compat.js` - userScripts API translation

**Legacy API Shims:**
- `tabs-windows-compat.js` - Deprecated tabs API mapping
- `runtime-compat.js` - runtime.getPackageDirectoryEntry stub

## Testing in Firefox

### Method 1: Temporary Add-on

1. Open Firefox
2. Navigate to `about:debugging#/runtime/this-firefox`
3. Click "Load Temporary Add-on"
4. Select `output/manifest.json` or the converted `.xpi/.zip` file

### Method 2: Install XPI

1. Open Firefox
2. Navigate to `about:addons`
3. Click gear icon → "Install Add-on From File"
4. Select `output.xpi`

### Debugging

Check Browser Console (Ctrl+Shift+J) for errors.

## Known Limitations

### Chrome-Only APIs

Some Chrome features have no Firefox equivalent:

**Not Available:**
- `chrome.offscreen.*` - No equivalent
- `chrome.declarativeContent.*` - No equivalent
- `chrome.tabGroups.*` - No equivalent

**Partial Support (Shims Provided):**
- `chrome.sidePanel.*` - Maps to sidebarAction (different UI)
- `chrome.declarativeNetRequest.*` - Converted to webRequest
- `chrome.userScripts.*` - Maps to different API structure
- `chrome.storage.session` - In-memory polyfill
- `chrome.privacy.*` - Stubbed as read-only

**Legacy APIs (Shims Provided):**
- `tabs.getSelected` / `tabs.getAllInWindow` - Mapped to tabs.query
- `runtime.getPackageDirectoryEntry` - Stubbed with guidance

### Service Workers vs Event Pages

- Chrome uses service workers, Firefox uses event pages
- Different lifecycle management
- No `importScripts()` in Firefox
- Converted to `background.scripts` array

### Host Permissions

Firefox treats `host_permissions` as optional (user can deny), while Chrome grants them at install time.

## Development

### Project Structure

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library root
├── wasm.rs              # WebAssembly bindings
├── models/              # Data structures
├── parser/              # Manifest & JS parsing
├── analyzer/            # Incompatibility detection
├── transformer/         # Code transformation
│   ├── manifest.rs      # Manifest transformer
│   ├── javascript.rs    # JS transformer
│   ├── shims.rs        # Shim generation
│   └── ast/            # AST transformation modules
├── packager/            # XPI packaging
├── validator/           # Output validation
└── report/             # Report generation

web/
├── index.html          # Web UI
├── app.js             # Frontend logic
├── styles.css         # Glassmorphic styling
└── pkg/               # Generated WASM (after build)
```

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

### Running with Cargo

```bash
# Analyze
cargo run -- analyze -i ./LatexToCalc

# Convert
cargo run -- convert -i ./LatexToCalc -o ./output --report

# Release build (faster)
./target/release/chrome2moz convert -i ./extension -o ./output
```

## Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Technical architecture and implementation
- **In-code Documentation** - Run `cargo doc --open`
- **Conversion Reports** - Generated with `--report` flag

## Contributing

Contributions welcome! Areas for contribution:

- Additional API mappings
- More test cases
- Documentation improvements
- Bug fixes
- New features

## Troubleshooting

### Build Errors

```bash
cargo clean
cargo build --release
```

### Conversion Issues

1. Check error message
2. Review conversion report
3. Ensure input is valid Chrome MV3
4. Check file permissions

### Extension Doesn't Work

1. Open Browser Console (Ctrl+Shift+J)
2. Check for JavaScript errors
3. Review manifest in `about:debugging`
4. Verify permissions granted

### WASM Build Issues

```bash
# Update wasm-pack
cargo install wasm-pack --force

# Clean and rebuild
cargo clean
./build-wasm.sh
```

### Deployment Issues

**"Permission denied":**
- Check Settings → Actions → General → Workflow permissions
- Select "Read and write permissions"

**Page shows 404:**
- Verify GitHub Pages enabled (Settings → Pages)
- Ensure Source is "GitHub Actions"
- Wait for DNS propagation
- Access with trailing slash: `https://username.github.io/repo/`

**WASM files not loading:**
- Check browser console for file paths
- Verify `.nojekyll` file exists
- Confirm `web/pkg/` in deployment

## Performance

- **Analysis**: ~100-500ms for typical extensions
- **Conversion**: ~200ms-2s depending on size
- **Memory**: Handles files up to 100-200MB
- **WebAssembly**: Near-native performance in browser

## Security

- All processing client-side in WebAssembly UI
- No files uploaded to servers
- No data collection or tracking
- Open source - audit the code yourself

## License

This project is licensed under the MIT License.

## Acknowledgments

- Built with Rust and WebAssembly
- Test extension: [LatexToCalc](LatexToCalc/)
- Inspired by the need for cross-browser compatibility

## Support

- [Report a bug](https://github.com/OtsoBear/chrome2moz/issues)
- [Request a feature](https://github.com/OtsoBear/chrome2moz/issues)
- [Read architecture docs](./ARCHITECTURE.md)

---

**Status**: Production-ready  
**Version**: 0.1.0  
**Last Updated**: October 2025
