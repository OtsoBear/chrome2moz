# Chrome to Firefox Extension Converter

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![WebAssembly](https://img.shields.io/badge/WebAssembly-supported-654FF0.svg)](https://webassembly.org/)
[![Build Status](https://img.shields.io/github/actions/workflow/status/OtsoBear/chrome2moz/deploy.yml?branch=main)](https://github.com/OtsoBear/chrome2moz/actions)
[![GitHub Stars](https://img.shields.io/github/stars/OtsoBear/chrome2moz?style=social)](https://github.com/OtsoBear/chrome2moz/stargazers)
[![GitHub Issues](https://img.shields.io/github/issues/OtsoBear/chrome2moz)](https://github.com/OtsoBear/chrome2moz/issues)
[![GitHub Last Commit](https://img.shields.io/github/last-commit/OtsoBear/chrome2moz)](https://github.com/OtsoBear/chrome2moz/commits/main)

A Rust-based CLI tool and WebAssembly library that converts Chrome Manifest V3 extensions to Firefox-compatible format, focusing on **Chrome-only APIs** and **real compatibility differences**.

**Live Demo**: [https://otsobear.github.io/chrome2moz/](https://otsobear.github.io/chrome2moz/)

## Key Understanding

**Firefox natively supports `chrome.*` APIs!** Most Chrome extensions work in Firefox without changes. This tool focuses on:

1. **Chrome-only APIs** that don't exist in Firefox (e.g., `chrome.offscreen`, `chrome.declarativeContent`)
2. **Manifest differences** (service workers → event pages, permission separation)
3. **Behavior differences** (URL resolution, web_accessible_resources)

## Features

- **Smart Detection**: Identifies Chrome-only APIs that need conversion
- **Minimal Transformation**: Pass-through approach with runtime shims
- **Manifest Transformation**: Handles MV3 manifest differences for Firefox
- **Runtime Compatibility**: 10 shims for Chrome-only APIs
- **importScripts() Handling**: Automatic detection and manifest integration
- **Multiple Input Formats**: Supports `.crx`, `.zip`, or unpacked directories
- **WebAssembly UI**: Browser-based interface (no installation required)
- **XPI Packaging**: Ready-to-install Firefox extension packages

## Chrome API Coverage

###  Implementation Progress

![API Implementation Progress](https://progress-bar.xyz/34/?scale=100&title=API%20Coverage&width=500&color=122f&suffix=%25)

**61 of 179 Chrome-only APIs** have automatic conversion support

| Category | Count | Status |
|----------|-------|--------|
|  **Total Tracked** | 179 | APIs supported in Chrome but not Firefox |
|  **Implemented** | 61 (34%) | Automatic conversion with shims/converters |
|  **Not Implemented** | 118 (66%) | Detection only, no conversion yet |

**[ View Full API Status & Breakdown →](./CHROME_ONLY_API_IMPLEMENTATION_STATUS.md)**

>  **Tip**: Run `cargo run chrome-only-apis` to fetch the latest Chrome-only API list from MDN and check current implementation status.

## Quick Start

### Web UI (Recommended)

```bash
# Build and serve
./build-wasm.sh
cd web && python3 -m http.server 8080

# Open http://localhost:8080
```

Drag & drop your Chrome extension ZIP, analyze, and download the converted Firefox version.

### Command Line

```bash
# Install
git clone https://github.com/OtsoBear/chrome2moz.git
cd chrome2moz
cargo build --release

# Analyze compatibility issues
./target/release/chrome2moz analyze -i ./chrome-extension

# Convert (with all options)
./target/release/chrome2moz convert -i ./chrome-extension -o ./output --report --yes

# List Chrome-only APIs
./target/release/chrome2moz chrome-only-apis

# Check for keyboard shortcut conflicts
./target/release/chrome2moz check-shortcuts
```

**Options:**
- `--report` - Generate detailed markdown report
- `--yes` / `-y` - Skip interactive prompts, use defaults
- `--preserve-chrome` - Keep both chrome and browser namespaces for compatibility

For interactive mode, run without arguments: `./target/release/chrome2moz`

## What It Actually Converts

**Note**: Firefox already supports `chrome.*` namespace natively! JavaScript passes through unchanged with runtime shims.

**Runtime Shims** (Chrome-only APIs):
- `chrome.storage.session` → In-memory polyfill
- `chrome.sidePanel` → Firefox `sidebarAction` mapping
- `chrome.declarativeNetRequest` → Stub (guides to webRequest)
- `chrome.tabGroups` → No-op stub
- `chrome.privacy` → Stub
- `chrome.userScripts` → ContentScripts mapping
- `chrome.tabs/windows` → Compatibility layer
- `chrome.runtime` → Compatibility layer
- `chrome.downloads` → Compatibility layer
- `chrome.notifications` → Compatibility layer

**Manifest Transformations**:
- `background.service_worker` → `background.scripts` (event page)
- Add `browser_specific_settings.gecko` for extension ID
- Separate `permissions` from `host_permissions`
- Handle `web_accessible_resources` format differences
- `importScripts()` → Extract scripts, add to manifest, comment out calls

## Testing in Firefox

1. Open `about:debugging#/runtime/this-firefox`
2. Click "Load Temporary Add-on"
3. Select the converted `manifest.json` or `.xpi` file

For errors, check Browser Console (Ctrl+Shift+J).

## Known Limitations & Compatibility

**What Works Automatically:**
- ✅ Standard WebExtension APIs (`chrome.storage`, `chrome.tabs`, `chrome.runtime`, etc.)
- ✅ Most Chrome APIs (Firefox supports them natively)
- ✅ Callback-based APIs (Firefox handles them automatically)
- ✅ TypeScript extensions

**What Needs Conversion** (This Tool Handles):
- ⚙️ `chrome.offscreen` → Web Workers/content scripts
- ⚙️ `chrome.declarativeContent` → Content script patterns
- ⚙️ `chrome.declarativeNetRequest` → webRequest API
- ⚙️ `chrome.storage.session` → In-memory polyfill
- ⚙️ `chrome.sidePanel` → sidebarAction
- ⚙️ Service workers → Event pages
- ⚙️ Manifest permission separation

**Not Supported in Firefox** (Stubbed):
- ❌ `chrome.tabGroups` (Firefox doesn't support tab grouping)
- ❌ `chrome.action.openPopup()` (not available)
- ❌ Some `chrome.privacy` settings (different architecture)
- ❌ Various Chrome-specific extended features

See **[Chrome API Implementation Status](./CHROME_ONLY_API_IMPLEMENTATION_STATUS.md)** for the complete list.

**Important Notes:**
- Firefox **does support** `chrome.*` namespace natively
- No need to rewrite code to use `browser.*` (though you can if you want)
- Promise/callback differences are handled automatically by Firefox
- Focus is on **actual incompatibilities**, not artificial namespace differences

## Building for Development

```bash
# Build CLI tool
cargo build --release

# Run tests
cargo test

# Build WebAssembly for web UI
./build-wasm.sh

# Generate documentation
cargo doc --open

# Format and lint
cargo fmt
cargo clippy -- -D warnings
```

**Note:** The WASM build requires `wasm-pack`:
```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```


## Contributing

Contributions welcome! See [`ARCHITECTURE.md`](ARCHITECTURE.md) for architectural details.

**Areas for improvement:**
- Additional Chrome-only API conversion strategies
- More comprehensive API compatibility shims
- Enhanced test coverage for edge cases
- Documentation improvements
- Bug fixes and performance optimizations

**Before submitting:**
```bash
cargo fmt && cargo clippy && cargo test
```

## License

MIT License - See LICENSE file for details.

## Support

- [Report Issues](https://github.com/OtsoBear/chrome2moz/issues)
- [View Architecture](./ARCHITECTURE.md)
- [Live Demo](https://otsobear.github.io/chrome2moz/)
