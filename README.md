# Chrome to Firefox Extension Converter

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![WebAssembly](https://img.shields.io/badge/WebAssembly-supported-654FF0.svg)](https://webassembly.org/)
[![Build Status](https://img.shields.io/github/actions/workflow/status/OtsoBear/chrome2moz/deploy.yml?branch=main)](https://github.com/OtsoBear/chrome2moz/actions)
[![GitHub Stars](https://img.shields.io/github/stars/OtsoBear/chrome2moz?style=social)](https://github.com/OtsoBear/chrome2moz/stargazers)
[![GitHub Issues](https://img.shields.io/github/issues/OtsoBear/chrome2moz)](https://github.com/OtsoBear/chrome2moz/issues)
[![GitHub Last Commit](https://img.shields.io/github/last-commit/OtsoBear/chrome2moz)](https://github.com/OtsoBear/chrome2moz/commits/main)

A Rust-based CLI tool and WebAssembly library (`chrome2moz`) that automatically converts Chrome Manifest V3 extensions to Firefox-compatible format. Features AST-based parsing, automatic API conversion, manifest transformation, and compatibility shim generation.

**Live Demo**: [https://otsobear.github.io/chrome2moz/](https://otsobear.github.io/chrome2moz/)

## Features

- **AST-Based Parsing**: Accurate code transformations using SWC
- **Full TypeScript Support**: Handles `.ts`, `.tsx`, and `.d.ts` files
- **Automatic API Conversion**: `chrome.*` → `browser.*` with scope awareness
- **Smart Polyfills**: Context-aware injection based on module type
- **Manifest Transformation**: MV3 manifests adapted for Firefox
- **Compatibility Shims**: 12 auto-generated shims for API differences
- **Multiple Input Formats**: Supports `.crx`, `.zip`, or unpacked directories
- **WebAssembly UI**: Browser-based interface (no installation required)
- **XPI Packaging**: Ready-to-install Firefox extension packages

## Chrome API Coverage

###  Implementation Progress

![API Implementation Progress](https://progress-bar.xyz/34/?scale=100&title=API%20Coverage&width=500&color=00d1b2&suffix=%25)

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

## What It Does

**JavaScript Transformations:**
- Converts `chrome.*` API calls to `browser.*`
- Injects browser polyfills and compatibility shims
- Fixes `executeScript` parameter differences (`function` → `func`)
- Handles TypeScript files with automatic type stripping

**Manifest Transformations:**
- Adds Firefox-specific `browser_specific_settings`
- Converts service workers to event pages
- Adjusts permission declarations
- Updates background script configuration

**Compatibility Shims:**
- `browser` polyfill (chrome → browser namespace)
- Session storage polyfill (`chrome.storage.session`)
- Action API compatibility (`chrome.action` ↔ `browser.action`)
- declarativeNetRequest stub with webRequest migration guidance
- sidePanel → sidebarAction mapping (different UI placement)
- Legacy API wrappers (deprecated tabs/windows methods)
- Downloads API compatibility (removes unsupported options)
- Notifications compatibility (removes Chrome-only features)
- Runtime compatibility (Chrome-specific methods)
- Privacy API stubs
- User scripts compatibility
- Promise wrapper utilities

## Testing in Firefox

1. Open `about:debugging#/runtime/this-firefox`
2. Click "Load Temporary Add-on"
3. Select the converted `manifest.json` or `.xpi` file

For errors, check Browser Console (Ctrl+Shift+J).

## Known Limitations

**Chrome-Only APIs:**

See **[ Chrome API Implementation Status](./CHROME_ONLY_API_IMPLEMENTATION_STATUS.md)** for the complete list of 176 Chrome-only APIs and their implementation status.

**Fully Implemented (Automatic Conversion):**
-  `chrome.offscreen.*` - Converted to Web Workers, Content Scripts, or Background Script integrations
-  `chrome.declarativeContent.*` - Converted to content script + messaging patterns
-  `chrome.declarativeNetRequest.*` - Full converter to Firefox `webRequest` API (46 APIs)
-  `chrome.sidePanel.*` - Maps to Firefox `sidebarAction` with compatibility layer (10 APIs)
-  `chrome.storage.session` - In-memory polyfill using JavaScript Map
-  `chrome.userScripts.*` - Falls back to `contentScripts.register()`
-  Legacy APIs - `tabs.getSelected`, `tabs.getAllInWindow`, etc.

**Stub/No-Op (No Firefox Equivalent):**
-  `chrome.tabGroups.*` - Stub provided (Firefox doesn't support tab grouping)
-  `chrome.action.openPopup` - Not available in Firefox

**Not Yet Implemented (118 APIs):**
- Most `devtools.*` extended features (19 APIs)
- Extended `notifications.*` options (11 APIs)
- `privacy.*` settings (12 APIs)
- Various extended features in `tabs`, `downloads`, `runtime`, etc.

**Important Differences:**
- **Service Workers**: Converted to event pages (Firefox background scripts)
- **Host Permissions**: Optional by default in Firefox (users can deny)
- **Manifest Keys**: Some Chrome-specific keys preserved for cross-browser compatibility

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
