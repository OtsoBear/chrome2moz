# Chrome to Firefox Extension Converter

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![WebAssembly](https://img.shields.io/badge/WebAssembly-supported-654FF0.svg)](https://webassembly.org/)
[![Build Status](https://img.shields.io/github/actions/workflow/status/OtsoBear/chrome2moz/deploy.yml?branch=main)](https://github.com/OtsoBear/chrome2moz/actions)

A Rust-based CLI tool and WebAssembly library that converts Chrome Manifest V3 extensions to Firefox-compatible format.

**Live Demo**: [https://otsobear.github.io/chrome2moz/](https://otsobear.github.io/chrome2moz/)

## Why This Tool?

Firefox natively supports `chrome.*` APIs, but some Chrome-only APIs don't exist in Firefox. This tool:

- **Detects Chrome-only APIs** (e.g., `chrome.offscreen`, `chrome.declarativeContent`, `chrome.tabGroups`) - see [current coverage](./CHROME_ONLY_API_IMPLEMENTATION_STATUS.md)
- **Converts manifests** (service workers → event pages, permission separation)
- **Provides runtime shims** for Chrome-only APIs
- **Checks keyboard shortcuts** for conflicts with Firefox built-ins

**Read more**: [ARCHITECTURE.md](./ARCHITECTURE.md) covers Chrome API detection system, detection scope, and keyboard shortcuts in detail.

## Features

- **Smart Detection**: Identifies Chrome-only APIs that need conversion
- **Manifest Transformation**: Handles MV3 manifest differences for Firefox
- **Keyboard Shortcut Checker**: Detects conflicts with 60+ Firefox shortcuts
- **Multiple Formats**: Supports `.crx`, `.zip`, or unpacked directories
- **Web Interface**: Browser-based UI (no installation required)

## Quick Start

### Web UI (Recommended)

```bash
./build-wasm.sh
cd web && python3 -m http.server 8080
# Open http://localhost:8080
```

Drag & drop your Chrome extension ZIP, analyze, and download the converted Firefox version.

### CLI

```bash
# Install
git clone https://github.com/OtsoBear/chrome2moz.git
cd chrome2moz
cargo build --release

# Analyze
./target/release/chrome2moz analyze -i ./chrome-extension

# Convert
./target/release/chrome2moz convert -i ./chrome-extension -o ./output

# List Chrome-only APIs
./target/release/chrome2moz chrome-only-apis
```

**Options**: `--report` (generate report), `--yes` (skip prompts), `--preserve-chrome` (keep both namespaces)

## What Gets Converted

**Chrome-Only APIs** → Runtime shims provided for:
- `chrome.storage.session` → In-memory polyfill
- `chrome.sidePanel` → Firefox `sidebarAction`
- `chrome.offscreen` → Web Workers/content scripts
- `chrome.declarativeContent` → Content script patterns
- `chrome.tabGroups` → No-op stub (Firefox doesn't support)
- And more... (see [ARCHITECTURE.md](./ARCHITECTURE.md))

**Manifest Changes**:
- `background.service_worker` → `background.scripts` (event page)
- Add `browser_specific_settings.gecko` for Firefox ID
- Separate `permissions` from `host_permissions`
- Convert `web_accessible_resources` format
- Handle `importScripts()` → Add to manifest

**Firefox Compatibility Fixes**:
- Disables `browser.management.uninstallSelf()` calls so extensions that detect Firefox don't self-destruct (seen in OneNote Web Clipper)

## API Coverage
**[View Full API Status →](./CHROME_ONLY_API_IMPLEMENTATION_STATUS.md)**

## Testing in Firefox

1. Open `about:debugging#/runtime/this-firefox`
2. Click "Load Temporary Add-on"
3. Select the converted `manifest.json` or `.xpi` file

Check Browser Console (Ctrl+Shift+J) for any errors.

## Important Notes

- **Firefox supports `chrome.*` namespace** natively - no need to rewrite to `browser.*`
- **Static analysis has limits** - runtime behavior differences need manual testing
- **~90% of conversions work** automatically; remaining 10% may need manual adjustments
- See [ARCHITECTURE.md](./ARCHITECTURE.md) for what's detected vs. what requires testing

## Contributing

Contributions welcome! See [ARCHITECTURE.md](./ARCHITECTURE.md) for architectural details.

```bash
cargo fmt && cargo clippy && cargo test
```

### E2E differential tests

`e2e/` runs every corpus extension through the converter, loads the original in
Chromium and the converted build in Firefox, and diffs their API-call traces.
CI fails on unallowed divergence.

```bash
cd e2e && pnpm install && pnpm e2e            # full corpus
pnpm e2e --only latextocalc                    # one extension
```

Corpus lives in `e2e/corpus.json`. See
`docs/superpowers/specs/2026-07-29-e2e-differential-testing-design.md`.

## Resources

- [Report Issues](https://github.com/OtsoBear/chrome2moz/issues)
- [Architecture Documentation](./ARCHITECTURE.md)
- [Live Demo](https://otsobear.github.io/chrome2moz/)
- [API Implementation Status](./CHROME_ONLY_API_IMPLEMENTATION_STATUS.md)

## License

MIT License - See LICENSE file for details.
