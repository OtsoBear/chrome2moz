# Chrome2Moz Architecture Documentation

> **Focused Chrome to Firefox extension converter**
> Built with Rust, designed for real incompatibilities

---

## Executive Summary

Chrome2Moz converts Chrome extensions to Firefox by handling **actual incompatibilities**, not artificial namespace differences.

**Key Understanding**: Firefox natively supports `chrome.*` namespace! This tool focuses on:
1. Chrome-only APIs that don't exist in Firefox
2. Manifest format differences
3. Behavior differences (URL resolution, etc.)

**Key Features:**
- **Smart Detection**: Identifies Chrome-only APIs requiring conversion
- **Pass-Through Approach**: JavaScript unchanged, runtime shims handle compatibility
- **Runtime Shims**: 10 compatibility layers for Chrome-only APIs
- **Multi-Target**: CLI tool, WASM library, and web interface
- **importScripts() Handling**: Automatic detection and manifest integration

---

## Table of Contents

1. [Project Structure](#project-structure)
2. [Architecture Overview](#architecture-overview)
3. [Core Components](#core-components)
4. [Shim System](#shim-system)
5. [Manifest Transformation](#manifest-transformation)
6. [importScripts() Handling](#importscripts-handling)
7. [CLI & WASM](#cli--wasm)
8. [Design Decisions](#design-decisions)
9. [API Compatibility Matrix](#api-compatibility-matrix)

---

## Project Structure

```
chrome2moz/
├── src/
│   ├── main.rs                    # CLI entry point
│   ├── lib.rs                     # Library root
│   ├── wasm.rs                    # WebAssembly interface
│   │
│   ├── models/                    # Data structures
│   │   ├── manifest.rs            # Manifest V3 types (700+ lines)
│   │   ├── extension.rs           # Extension representation
│   │   ├── incompatibility.rs     # Issue tracking
│   │   ├── conversion.rs          # Conversion results
│   │   └── chrome_only.rs         # Chrome-only API models
│   │
│   ├── parser/                    # Input parsing
│   │   └── manifest.rs            # manifest.json parser
│   │
│   ├── analyzer/                  # Compatibility analysis
│   │   ├── api.rs                 # Chrome-only API detection
│   │   ├── manifest.rs            # Manifest compatibility
│   │   ├── offscreen.rs           # Offscreen document analyzer (regex)
│   │   └── declarative_content.rs # DeclarativeContent analyzer
│   │
│   ├── transformer/               # Code transformation
│   │   ├── manifest.rs            # Manifest transformations + importScripts()
│   │   ├── javascript.rs          # Pass-through (comments out importScripts)
│   │   ├── shims.rs               # 10 runtime compatibility shims
│   │   ├── chrome_only_converter.rs # Chrome-only API coordinator
│   │   ├── offscreen_converter.rs   # Offscreen conversions
│   │   ├── declarative_content_converter.rs
│   │   └── tab_groups.rs          # TabGroups stub
│   │
│   ├── packager/                  # Output generation
│   │   ├── extractor.rs           # CRX/ZIP extraction
│   │   └── builder.rs             # XPI building
│   │
│   ├── report/                    # Report generation
│   │   └── generator.rs
│   │
│   ├── validator/                 # Output validation
│   │   └── structure.rs
│   │
│   └── cli/                       # CLI interaction
│       └── interactive.rs
│
├── tests/                         # Integration tests
└── web/                          # Web interface
```

---

## Architecture Overview

### Pass-Through Pipeline

```mermaid
flowchart TD
    A[Input: CRX/ZIP/Directory] --> B[Extract & Parse]
    B --> C[Analyze]
    C --> D[Transform Manifest]
    
    subgraph Transform["Minimal Changes"]
        D1[Manifest Adjustments]
        D2[importScripts Detection]
        D3[Comment importScripts Lines]
    end
    
    D --> Transform
    Transform --> E[Include 10 Runtime Shims]
    E --> F[Package & Validate]
    F --> G[Output: Firefox Extension]
    
    style A fill:#e3f2fd
    style G fill:#c8e6c9
```

### Data Flow

1. **Extract**: Parse CRX/ZIP → [`Extension`](src/models/extension.rs) struct
2. **Analyze**: Detect Chrome-only APIs → [`Vec<Incompatibility>`](src/models/incompatibility.rs)
3. **Transform**:
   - Manifest adjustments (service worker, permissions, importScripts)
   - JavaScript pass-through (comments out importScripts() only)
4. **Generate**: Include all 10 runtime shims (always)
5. **Package**: Build output → XPI + report

**Note**: JavaScript passes through unchanged! Runtime shims handle all compatibility.

---

## Core Components

### 1. Models ([`src/models/`](src/models/))

**[`Manifest`](src/models/manifest.rs)**: Comprehensive Chrome MV3 + Firefox representation
- 700+ lines covering all manifest fields
- Handles both MV2 and MV3
- Smart defaults for missing fields

**[`Incompatibility`](src/models/incompatibility.rs)**: Issue tracking with severity levels
```rust
pub enum Severity {
    Blocker,  // Extension won't work
    Major,    // Core functionality affected
    Minor,    // Edge cases
    Info,     // Informational
}
```

### 2. Parser ([`src/parser/`](src/parser/))

**[`manifest.rs`](src/parser/manifest.rs)**: Robust JSON parsing with validation
- Detailed error messages
- Auto-fills optional fields
- Validates required fields

### 3. Analyzer ([`src/analyzer/`](src/analyzer/))

**[`api.rs`](src/analyzer/api.rs)**: Detects Chrome-only APIs
- Pattern matching for API calls
- Provides specific shim recommendations

**[`offscreen.rs`](src/analyzer/offscreen.rs)**: Analyzes offscreen documents (regex-based)
- Determines primary purpose (Canvas/Audio/DOM/Network)
- No AST parsing needed

**[`declarative_content.rs`](src/analyzer/declarative_content.rs)**: Analyzes declarativeContent rules (regex-based)
- Extracts conditions and actions

### 4. Transformer ([`src/transformer/`](src/transformer/))

**[`javascript.rs`](src/transformer/javascript.rs)**: Pass-through with importScripts() handling
- Detects importScripts() calls in background scripts
- Comments them out with explanation
- No other JavaScript transformations

**[`manifest.rs`](src/transformer/manifest.rs)**: Manifest + importScripts() integration
- Extracts script names from importScripts() calls (regex)
- Adds scripts to manifest.background.scripts in correct order
- Service worker → event page conversion

**[`shims.rs`](src/transformer/shims.rs)**: 10 runtime compatibility shims
- Always included for maximum compatibility
- Handle Chrome-only APIs at runtime
- Cross-browser compatible (work in both Chrome and Firefox)

### 5. Packager ([`src/packager/`](src/packager/))

**[`extractor.rs`](src/packager/extractor.rs)**: Handles `.crx`, `.zip`, and directories

**[`builder.rs`](src/packager/builder.rs)**: Creates Firefox output with XPI support

---

## Shim System

Dynamic compatibility layer included in every conversion.

### Runtime Shims (Always Included)

**[`shims.rs`](src/transformer/shims.rs)** generates 10 compatibility files:

1. **storage-session-compat.js**: In-memory polyfill for `chrome.storage.session`
2. **execute-script-compat.js**: Parameter name compatibility
3. **sidepanel-compat.js**: Maps `chrome.sidePanel` → `sidebarAction`
4. **declarative-net-request-stub.js**: Stub with guidance
5. **user-scripts-compat.js**: Maps to contentScripts
6. **tabs-windows-compat.js**: Compatibility fixes
7. **runtime-compat.js**: Runtime API compatibility
8. **downloads-compat.js**: Downloads API fixes
9. **privacy-stub.js**: No-op stub
10. **notifications-compat.js**: Notification compatibility

**Key Design**: All shims are cross-browser compatible and include runtime checks.

## importScripts() Handling

**Challenge**: Chrome service workers support `importScripts()`, Firefox event pages don't.

**Safe Solution** (no eval, no security risk):

### Detection Phase

**[`extract_imported_scripts()`](src/transformer/manifest.rs)** uses regex:
1. Reads background.js content
2. Finds `importScripts('config.js', 'timing.js')` calls
3. Extracts script names: `['config.js', 'timing.js']`
4. Works on both commented and uncommented lines

### Manifest Integration

Adds scripts to `manifest.background.scripts` BEFORE background.js:
```json
{
  "background": {
    "scripts": [
      "shims/storage-session-compat.js",
      "config.js",      // ← From importScripts()
      "timing.js",      // ← From importScripts()
      "background.js"
    ]
  }
}
```

### Code Cleanup

**[`javascript.rs`](src/transformer/javascript.rs)** comments out importScripts():
```javascript
// importScripts('config.js', 'timing.js'); // Moved to manifest.background.scripts for Firefox compatibility
```

**Result**: Scripts load in correct order, no `importScripts()` errors, completely safe!

---

## Manifest Transformation

### Key Changes

1. **Add Firefox-Specific Settings**
```json
{
  "browser_specific_settings": {
    "gecko": {
      "id": "{extension-name}@converted.extension",
      "strict_min_version": "121.0"
    }
  }
}
```

2. **Background Configuration** (with importScripts() handling)
```json
{
  "background": {
    "scripts": [
      "shims/storage-session-compat.js",  // Shims first
      "config.js",                        // Extracted from importScripts()
      "background.js"                      // Original script
    ]
  }
}
```

3. **Permission Separation**
```json
// Chrome allows mixing:
{ "permissions": ["storage", "https://*/*"] }

// Firefox requires separation:
{
  "permissions": ["storage"],
  "host_permissions": ["https://*/*"]
}
```

---

## CLI & WASM

### CLI Commands ([`main.rs`](src/main.rs))

```bash
# Full conversion
chrome2moz convert -i ./chrome-ext -o ./output --report

# Analysis only
chrome2moz analyze -i ./chrome-ext

# List Chrome-only APIs
chrome2moz chrome-only-apis

# Check keyboard shortcuts
chrome2moz check-shortcuts
```

### WASM Interface ([`wasm.rs`](src/wasm.rs))

Exposes conversion pipeline to web browsers:
```rust
#[wasm_bindgen]
pub fn convert_extension(
    manifest_json: &str,
    files: JsValue,
) -> Result<JsValue, JsValue>
```

**Web Demo**: [https://otsobear.github.io/chrome2moz](https://otsobear.github.io/chrome2moz)

---

## Design Decisions

### Pass-Through Architecture

**Design**: JavaScript unchanged, runtime shims provide compatibility

**Rationale**:
- Firefox natively supports `chrome.*` namespace
- No AST parsing needed (1.6GB build vs 3.6GB with SWC)
- Runtime compatibility more reliable than static transformation
- Simpler codebase, easier maintenance
- Faster conversion with fewer edge cases

### Always-Include Shims

**Design**: Include all 10 shims in every conversion

**Rationale**:
- Guarantees compatibility (~50KB overhead)
- No conditional detection needed
- Cross-browser compatible (work in Chrome too)
- Users don't need to debug missing shims

### importScripts() via Manifest

**Design**: Extract scripts and add to manifest instead of polyfilling

**Rationale**:
- Completely safe (no eval, no unsafe-eval CSP)
- Firefox loads scripts in order automatically
- Passes AMO security review
- Simple regex detection

---

## API Compatibility Matrix

| Chrome API | Firefox | Conversion |
|-----------|---------|------------|
| [`storage`](https://developer.chrome.com/docs/extensions/reference/api/storage) | Full | Direct mapping |
| [`storage.session`](https://developer.chrome.com/docs/extensions/reference/api/storage#property-session) | No | In-memory polyfill |
| [`tabs`](https://developer.chrome.com/docs/extensions/reference/api/tabs) | Full | Direct mapping |
| [`action`](https://developer.chrome.com/docs/extensions/reference/api/action) | v109+ | Direct mapping |
| [`offscreen`](https://developer.chrome.com/docs/extensions/reference/api/offscreen) | No | Worker/ContentScript |
| [`sidePanel`](https://developer.chrome.com/docs/extensions/reference/api/sidePanel) | No | Map to [`sidebarAction`](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/sidebarAction) |
| [`declarativeContent`](https://developer.chrome.com/docs/extensions/reference/api/declarativeContent) | No | ContentScript + messaging |
| [`tabGroups`](https://developer.chrome.com/docs/extensions/reference/api/tabGroups) | No | No-op stub |
| [`declarativeNetRequest`](https://developer.chrome.com/docs/extensions/reference/api/declarativeNetRequest) | Limited | Stub + guide to [`webRequest`](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/webRequest) |

---

## Contributing

### Architecture Principles

1. **Modularity**: Single responsibility per module
2. **Type Safety**: Leverage Rust's type system
3. **Error Handling**: Use `Result` types, never panic
4. **Testing**: Every feature needs tests
5. **Documentation**: Keep this file updated

### Code Style

```bash
cargo fmt && cargo clippy -- -D warnings && cargo test
```

---

## Resources

- [Chrome Extensions API](https://developer.chrome.com/docs/extensions/reference/)
- [Firefox WebExtensions API](https://developer.mozilla.org/docs/Mozilla/Add-ons/WebExtensions/API)
- [SWC Documentation](https://swc.rs/)
- [WebExtension Polyfill](https://github.com/mozilla/webextension-polyfill)

---

**Version**: 0.2.0  
**Status**: Production-ready  
**Maintainer**: [@OtsoBear](https://github.com/OtsoBear)

**For user documentation, see [`README.md`](README.md)**