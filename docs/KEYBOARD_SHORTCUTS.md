# Keyboard Shortcut Conflict Checker

## Overview

The keyboard shortcut checker automatically detects conflicts between your Chrome extension's keyboard shortcuts and Firefox's built-in shortcuts, helping you choose alternative keybindings that won't interfere with Firefox functionality.

## Features

- **Automatic Detection**: Scans your extension's `manifest.json` for keyboard shortcut definitions
- **Comprehensive Database**: Checks against 60+ Firefox built-in shortcuts (navigation, tabs, DevTools, etc.)
- **Interactive Selection**: User-friendly dropdown interface to select alternative shortcuts
- **Custom Shortcuts**: Option to enter custom shortcut combinations
- **Smart Suggestions**: Automatically suggests available Ctrl+Shift+[Letter] and Alt+Shift+[Letter] combinations
- **Cross-Platform**: Supports both Windows/Linux (Ctrl) and macOS (Cmd) modifiers

## How It Works

### 1. Analysis Phase

When you upload your Chrome extension ZIP file, the analyzer:
1. Extracts keyboard shortcuts from the `commands` section in `manifest.json`
2. Normalizes shortcut formats (e.g., "Ctrl+Shift+I" → "ctrl+shift+i")
3. Compares against Firefox's built-in shortcuts database
4. Identifies conflicts and generates alternative suggestions

### 2. User Interface

If conflicts are detected, the web UI displays:
- A dedicated "⌨️ Keyboard Shortcut Conflicts" section
- Each conflicting shortcut with:
  - Original Chrome shortcut
  - Conflicting Firefox shortcut and its purpose
  - Dropdown list of available alternatives
  - Option to enter custom shortcuts

### 3. Resolution

When you convert the extension:
- Selected alternative shortcuts are automatically applied to the converted `manifest.json`
- Original shortcuts are replaced with your chosen alternatives
- No manual file editing required

## Firefox Shortcuts Database

The checker includes shortcuts for:

### Navigation & Tabs
- New tab, close tab, reopen closed tab
- Tab switching (Ctrl+Tab, Ctrl+1-9)
- Window management

### Browser UI
- Address bar, search bar
- History, bookmarks, downloads
- Extensions/add-ons manager

### Developer Tools
- Inspector (Ctrl+Shift+C)
- Console (Ctrl+Shift+K)
- Debugger, Network Monitor
- Responsive Design Mode

### Page Operations
- Find in page, reload, print
- Zoom controls
- Save page

## Example Conflicts

Common conflicts you might encounter:

| Chrome Shortcut | Firefox Function | Suggested Alternatives |
|----------------|------------------|----------------------|
| Ctrl+Shift+I | Open Developer Tools | Ctrl+Shift+X, Alt+Shift+I |
| Ctrl+Shift+K | Delete browsing data (Chrome) | Web Console (Firefox) | Ctrl+Shift+L, Alt+Shift+K |
| Ctrl+T | New tab | Ctrl+Shift+N, Alt+Shift+T |
| Ctrl+B | Toggle bookmarks bar (Chrome) | Show bookmarks (Firefox) | Ctrl+Shift+B is also taken |

## Recommended Shortcuts

Best practices for choosing alternative shortcuts:

### Safe Combinations
These are typically available across Firefox:
- `Ctrl+Shift+[Letter]` where letter is: X, Y, Z, Q, V, etc.
- `Alt+Shift+[Letter]` for most letters
- `Ctrl+Alt+[Letter]` (less common but usually available)

### Avoid
- Single keys (F1-F12 are often used)
- Ctrl+[Letter] alone (most are taken)
- Cmd+[Letter] on macOS (system shortcuts)

## Updating the Shortcuts Database

To fetch the latest Firefox shortcuts from Mozilla's documentation:

```bash
# Run the generator script (requires network access)
cargo run --features cli --bin generate-shortcuts

# This creates: src/analyzer/firefox_shortcuts_data.rs

# Rebuild WASM
bash build-wasm.sh
```

## Technical Details

### Architecture

```
src/analyzer/keyboard_shortcuts.rs
├── get_firefox_shortcuts()     # Precompiled Firefox shortcuts
├── extract_shortcuts()          # Parse manifest.json commands
├── normalize_shortcut()         # Standardize format
├── analyze_shortcuts()          # Detect conflicts
└── generate_alternatives()      # Suggest replacements
```

### Data Format

**Input (manifest.json)**:
```json
{
  "commands": {
    "toggle-feature": {
      "suggested_key": {
        "default": "Ctrl+Shift+I",
        "mac": "Command+Shift+I"
      },
      "description": "Toggle feature"
    }
  }
}
```

**Output (Analysis)**:
```json
{
  "conflicts": [
    {
      "chrome_shortcut": "Ctrl+Shift+I",
      "firefox_shortcut": "ctrl+shift+i",
      "firefox_description": "Toggle Developer Tools",
      "suggested_alternatives": ["Ctrl+Shift+X", "Ctrl+Shift+Y", "..."]
    }
  ],
  "safe_shortcuts": [],
  "available_alternatives": ["Ctrl+Shift+X", "Alt+Shift+A", "..."]
}
```

## WASM API

The keyboard shortcut checker is exposed through WASM:

```javascript
import { analyze_keyboard_shortcuts } from './pkg/chrome2moz.js';

// Analyze a Chrome extension ZIP
const zipData = new Uint8Array(arrayBuffer);
const analysisJson = analyze_keyboard_shortcuts(zipData);
const analysis = JSON.parse(analysisJson);

// Check for conflicts
if (analysis.conflicts.length > 0) {
  console.log('Found conflicts:', analysis.conflicts);
  console.log('Suggested alternatives:', analysis.available_alternatives);
}
```

## Future Enhancements

Potential improvements:
- [ ] Support for Firefox-specific keyboard APIs
- [ ] Conflict severity levels (critical vs warning)
- [ ] Automatic shortcut assignment (AI-based)
- [ ] Export shortcut documentation
- [ ] Platform-specific recommendations
- [ ] Custom shortcuts validation

## Resources

- [Firefox Keyboard Shortcuts](https://support.mozilla.org/en-US/kb/keyboard-shortcuts-perform-firefox-tasks-quickly)
- [Firefox DevTools Shortcuts](https://firefox-source-docs.mozilla.org/devtools-user/keyboard_shortcuts/index.html)
- [WebExtensions commands API](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/commands)