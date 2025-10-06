# Development Scripts

## fetch_chrome_only_apis.py

Fetches Chrome-only WebExtension APIs from MDN's browser-compat-data repository without cloning the entire repo.

### Usage

```bash
python3 scripts/fetch_chrome_only_apis.py
```

### What it does

1. Queries GitHub API to list all files in `webextensions/api/`
2. Downloads each JSON file from raw.githubusercontent.com
3. Analyzes compatibility data for Chrome vs Firefox
4. Lists APIs that are supported in Chrome but not in Firefox

### Output

```
WebExtension APIs supported in Chrome but not Firefox:

- chrome.offscreen.createDocument
    Source: offscreen.json
    Chrome: 109
    Firefox: not supported

- chrome.sidePanel.open
    Source: sidePanel.json
    Chrome: 114
    Firefox: not supported

...
```

### Requirements

- Python 3.7+
- `aiohttp` library for async HTTP requests
  ```bash
  pip install aiohttp
  ```
- Internet connection (no local data needed)

### Performance

The script uses **async/parallel fetching** to download ~100 JSON files concurrently, making it significantly faster than sequential downloads:
- **Sequential**: ~30-60 seconds
- **Concurrent**: ~3-5 seconds

### Use Cases

- **Update API List**: When you need to refresh the hardcoded Chrome-only API list in [`src/parser/javascript.rs`](../src/parser/javascript.rs)
- **Research**: Investigate which Chrome APIs don't have Firefox equivalents
- **Documentation**: Generate lists for project documentation

### Advantages over local clone

- **No disk space**: ~40MB saved by not cloning browser-compat-data
- **Always current**: Fetches latest data from GitHub
- **Fast**: Only downloads what's needed (~100 small JSON files)
- **Simple**: Pure Python, no git dependencies

### Integration (Future)

Could be integrated into the Rust converter for dynamic API detection:

```rust
// Future enhancement
pub fn fetch_chrome_only_apis() -> Result<HashSet<String>> {
    // Call GitHub API from Rust
    // Parse compatibility data
    // Return dynamic list
}
```

For now, the Rust code uses a hardcoded list of the most common Chrome-only APIs.