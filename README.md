# Chrome to Firefox Extension Converter

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![WebAssembly](https://img.shields.io/badge/WebAssembly-supported-654FF0.svg)](https://webassembly.org/)
[![Build Status](https://img.shields.io/github/actions/workflow/status/OtsoBear/chrome2moz/deploy.yml?branch=main)](https://github.com/OtsoBear/chrome2moz/actions)
[![GitHub Stars](https://img.shields.io/github/stars/OtsoBear/chrome2moz?style=social)](https://github.com/OtsoBear/chrome2moz/stargazers)
[![GitHub Issues](https://img.shields.io/github/issues/OtsoBear/chrome2moz)](https://github.com/OtsoBear/chrome2moz/issues)
[![GitHub Last Commit](https://img.shields.io/github/last-commit/OtsoBear/chrome2moz)](https://github.com/OtsoBear/chrome2moz/commits/main)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macos%20%7C%20windows-lightgrey.svg)](https://github.com/OtsoBear/chrome2moz)

A Rust-based tool that automatically converts Chrome Manifest V3 extensions to Firefox-compatible format. Features AST-based parsing, automatic API conversion, manifest transformation, and compatibility shim generation.

**Live Demo**: [https://otsobear.github.io/chrome2moz/](https://otsobear.github.io/chrome2moz/)

## Features

- **AST-Based Parsing**: Accurate code transformations using SWC
- **Full TypeScript Support**: Handles `.ts`, `.tsx`, and `.d.ts` files
- **Automatic API Conversion**: `chrome.*` → `browser.*` with scope awareness
- **Smart Polyfills**: Context-aware injection based on module type
- **Manifest Transformation**: MV3 manifests adapted for Firefox
- **Compatibility Shims**: 10+ auto-generated shims for API differences
- **WebAssembly UI**: Browser-based interface (no installation required)
- **XPI Packaging**: Ready-to-install Firefox extension packages

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

# Analyze
./target/release/chrome2moz analyze -i ./chrome-extension

# Convert
./target/release/chrome2moz convert -i ./chrome-extension -o ./output --report
```

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
- Session storage polyfill
- Action API bridging
- declarativeNetRequest fallbacks
- sidePanel → sidebarAction mapping
- Legacy API wrappers

## Testing in Firefox

1. Open `about:debugging#/runtime/this-firefox`
2. Click "Load Temporary Add-on"
3. Select the converted `manifest.json` or `.xpi` file

For errors, check Browser Console (Ctrl+Shift+J).

## Known Limitations

**Chrome-Only APIs (No Firefox Equivalent):**
- `chrome.offscreen.*`
- `chrome.declarativeContent.*`
- `chrome.tabGroups.*`

**Partial Support (Shims Provided):**
- `chrome.sidePanel.*` → `sidebarAction` (different UI)
- `chrome.declarativeNetRequest.*` → `webRequest` fallback
- `chrome.storage.session` → In-memory polyfill
- `chrome.userScripts.*` → Alternative API structure

**Other Differences:**
- Service workers converted to event pages
- `importScripts()` not supported in Firefox
- Host permissions are optional in Firefox (user can deny)

## Building for Development

```bash
# Build and test
cargo build --release
cargo test

# Build WebAssembly
./build-wasm.sh

# Generate documentation
cargo doc --open
```

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for technical details.

## Contributing

Contributions welcome! Areas for improvement:
- Additional API mappings
- Test cases
- Documentation
- Bug fixes

## License

MIT License - See LICENSE file for details.

## Support

- [Report Issues](https://github.com/OtsoBear/chrome2moz/issues)
- [View Architecture](./ARCHITECTURE.md)
- [Live Demo](https://otsobear.github.io/chrome2moz/)
