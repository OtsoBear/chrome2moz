use std::collections::HashSet;
use std::time::Duration;
use std::path::{Path, PathBuf};
use std::fs;

use anyhow::{Context, Result};
use reqwest::Client;
use regex::Regex;
use lazy_static::lazy_static;
use serde_json::Value;

const FIREFOX_DEVTOOLS_SHORTCUTS_URL: &str =
    "https://firefox-source-docs.mozilla.org/_sources/devtools-user/keyboard_shortcuts/index.rst.txt";

const FIREFOX_SUPPORT_SHORTCUTS_URLS: &[(&str, &str)] = &[
    ("Windows", "https://support.mozilla.org/en-US/kb/keyboard-shortcuts-perform-firefox-tasks-quickly#firefox:win11:fx143"),
    ("macOS", "https://support.mozilla.org/en-US/kb/keyboard-shortcuts-perform-firefox-tasks-quickly#firefox:mac:fx143"),
    ("Linux", "https://support.mozilla.org/en-US/kb/keyboard-shortcuts-perform-firefox-tasks-quickly#firefox:linux:fx143"),
];

lazy_static! {
    // Pattern to match :kbd:`Key` directives in RST format
    static ref KBD_PATTERN: Regex = Regex::new(
        r":kbd:`([^`]+)`"
    ).unwrap();
    
    // Pattern to match table rows that contain keyboard shortcuts
    static ref TABLE_ROW_PATTERN: Regex = Regex::new(
        r"(?m)^\s*-\s+:kbd:`"
    ).unwrap();
    
    // Pattern to match <span class="key">X</span> in HTML
    static ref HTML_KEY_PATTERN: Regex = Regex::new(
        r#"<span class="key">([^<]+)</span>"#
    ).unwrap();
    
    // Pattern to extract doc-content section
    static ref DOC_CONTENT_PATTERN: Regex = Regex::new(
        r#"(?s)<section id="doc-content"[^>]*>(.+?)</section>"#
    ).unwrap();
}

#[derive(Debug, Clone)]
pub struct FirefoxShortcut {
    pub shortcut: String,
    pub normalized: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct ShortcutConflict {
    pub chrome_shortcut: String,
    pub firefox_shortcut: FirefoxShortcut,
    pub severity: ConflictSeverity,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ConflictSeverity {
    Exact,      // Exact match
}

/// Run the keyboard shortcut conflict checker
pub async fn run() -> Result<()> {
    run_with_project_path(None).await
}

/// Run the keyboard shortcut conflict checker with a specific project path
pub async fn run_with_project_path(project_path: Option<&Path>) -> Result<()> {
    let client = Client::builder()
        .user_agent("chrome-to-firefox (https://github.com/OtsoBear/chrome-to-firefox)")
        .timeout(Duration::from_secs(30))
        .build()
        .context("failed to build HTTP client")?;

    eprintln!("Fetching Firefox keyboard shortcuts documentation...");
    
    // Fetch from developer tools documentation (RST)
    eprintln!("  - Fetching DevTools shortcuts...");
    let mut firefox_shortcuts = fetch_firefox_devtools_shortcuts(&client).await?;
    eprintln!("    Found {} DevTools shortcuts", firefox_shortcuts.len());
    
    // Fetch from support pages (HTML)
    for (platform, url) in FIREFOX_SUPPORT_SHORTCUTS_URLS {
        eprintln!("  - Fetching {} shortcuts...", platform);
        let support_shortcuts = fetch_firefox_support_shortcuts(&client, url, platform).await?;
        eprintln!("    Found {} {} shortcuts", support_shortcuts.len(), platform);
        firefox_shortcuts.extend(support_shortcuts);
    }
    
    // Deduplicate shortcuts
    let mut seen = HashSet::new();
    firefox_shortcuts.retain(|s| seen.insert(s.normalized.clone()));
    eprintln!("\nTotal unique shortcuts after deduplication: {}", firefox_shortcuts.len());
    
    println!("\n{}", "=".repeat(80));
    println!("Firefox Keyboard Shortcuts (Found: {})", firefox_shortcuts.len());
    println!("{}\n", "=".repeat(80));
    
    // Group shortcuts by normalized form for easier display
    let mut shortcuts_map: std::collections::HashMap<String, Vec<FirefoxShortcut>> = 
        std::collections::HashMap::new();
    
    for shortcut in &firefox_shortcuts {
        shortcuts_map
            .entry(shortcut.normalized.clone())
            .or_insert_with(Vec::new)
            .push(shortcut.clone());
    }
    
    // Sort by normalized shortcut for consistent output
    let mut sorted_keys: Vec<_> = shortcuts_map.keys().collect();
    sorted_keys.sort();
    
    for key in sorted_keys {
        let shortcuts = &shortcuts_map[key];
        println!("  {}", key);
        for shortcut in shortcuts {
            if !shortcut.description.is_empty() {
                println!("    → {}", shortcut.description);
            }
        }
    }
    
    // Search for manifest.json files in the project if a path was provided
    let mut project_conflicts = Vec::new();
    if let Some(path) = project_path {
        println!("\n{}", "=".repeat(80));
        println!("Searching for keyboard shortcuts in project...");
        println!("{}", "=".repeat(80));
        
        let manifest_files = find_manifest_files(path)?;
        println!("Found {} manifest.json file(s)\n", manifest_files.len());
        
        for manifest_path in &manifest_files {
            println!("Checking: {}", manifest_path.display());
            match extract_shortcuts_from_manifest(manifest_path) {
                Ok(shortcuts) => {
                    if !shortcuts.is_empty() {
                        println!("  Found {} keyboard shortcut(s):", shortcuts.len());
                        let conflicts = check_extension_shortcuts(&shortcuts, &firefox_shortcuts);
                        
                        for shortcut in &shortcuts {
                            let conflict_info = conflicts.iter()
                                .find(|c| c.chrome_shortcut == *shortcut);
                            
                            if let Some(conflict) = conflict_info {
                                println!("    ❌ CONFLICT {} (Firefox: {} - {})",
                                    shortcut,
                                    conflict.firefox_shortcut.normalized,
                                    conflict.firefox_shortcut.description);
                                
                                // Search for mentions of this shortcut in source code
                                if let Ok(mentions) = search_shortcut_in_source(path, shortcut) {
                                    if !mentions.is_empty() {
                                        println!("       Found {} mention(s) in source code:", mentions.len());
                                        for mention in &mentions {
                                            println!("         • {}:{}", mention.file, mention.line);
                                        }
                                    }
                                }
                            } else {
                                println!("    ✅ {} (no conflict)", shortcut);
                            }
                        }
                        
                        // Collect conflicts after displaying them
                        project_conflicts.extend(conflicts);
                    } else {
                        println!("  No keyboard shortcuts defined");
                    }
                }
                Err(e) => {
                    eprintln!("  Error reading manifest: {}", e);
                }
            }
            println!();
        }
        
        if !project_conflicts.is_empty() {
            println!("{}", "=".repeat(80));
            println!("⚠️  CONFLICT SUMMARY");
            println!("{}", "=".repeat(80));
            
            // Deduplicate conflicts (same shortcut may appear in multiple files)
            let mut seen_conflicts: HashSet<String> = HashSet::new();
            let mut unique_conflicts = Vec::new();
            
            for conflict in &project_conflicts {
                let key = format!("{}|{}", conflict.chrome_shortcut, conflict.firefox_shortcut.normalized);
                if seen_conflicts.insert(key) {
                    unique_conflicts.push(conflict);
                }
            }
            
            println!("Found {} unique keyboard shortcut conflict(s) in your project:\n", unique_conflicts.len());
            
            for conflict in &unique_conflicts {
                println!("  {} conflicts with Firefox's {} ({})",
                    conflict.chrome_shortcut,
                    conflict.firefox_shortcut.normalized,
                    conflict.firefox_shortcut.description);
            }
            println!();
        } else if !manifest_files.is_empty() {
            println!("{}", "=".repeat(80));
            println!("✅ No keyboard shortcut conflicts found in your project!");
            println!("{}", "=".repeat(80));
            println!();
        }
    }
    
    // Analyze available shortcut combinations
    println!("\n{}", "=".repeat(80));
    println!("Available Shortcut Analysis");
    println!("{}", "=".repeat(80));
    
    // Collect all shortcuts from Firefox and extension
    let firefox_used: HashSet<String> = firefox_shortcuts.iter()
        .map(|s| s.normalized.clone())
        .collect();
    
    let mut extension_used: HashSet<String> = HashSet::new();
    if let Some(path) = project_path {
        let manifest_files = find_manifest_files(path)?;
        for manifest_path in &manifest_files {
            if let Ok(shortcuts) = extract_shortcuts_from_manifest(manifest_path) {
                for shortcut in shortcuts {
                    extension_used.insert(normalize_shortcut(&shortcut));
                }
            }
        }
    }
    
    // Analyze Ctrl+Shift+[Letter] combinations
    println!("\nCtrl+Shift+[Letter] combinations:");
    println!("  Available: ", );
    let mut ctrl_shift_available = Vec::new();
    let mut ctrl_shift_firefox = Vec::new();
    let mut ctrl_shift_extension = Vec::new();
    
    for letter in 'A'..='Z' {
        let shortcut = format!("ctrl+shift+{}", letter.to_lowercase());
        if firefox_used.contains(&shortcut) {
            ctrl_shift_firefox.push(letter);
        } else if extension_used.contains(&shortcut) {
            ctrl_shift_extension.push(letter);
        } else {
            ctrl_shift_available.push(letter);
        }
    }
    
    if !ctrl_shift_available.is_empty() {
        println!("{}", ctrl_shift_available.iter().collect::<String>());
    } else {
        println!("(none)");
    }
    
    if !ctrl_shift_firefox.is_empty() {
        println!("  Used by Firefox: {}", ctrl_shift_firefox.iter().collect::<String>());
        // Show what each letter does
        println!("  Firefox usage details:");
        for letter in &ctrl_shift_firefox {
            let shortcut = format!("ctrl+shift+{}", letter.to_lowercase());
            let mut descriptions: Vec<String> = firefox_shortcuts.iter()
                .filter(|s| s.normalized == shortcut)
                .map(|s| s.description.clone())
                .collect();
            // Remove duplicates and empty descriptions
            descriptions.sort();
            descriptions.dedup();
            descriptions.retain(|d| !d.is_empty());
            
            if !descriptions.is_empty() {
                let desc = descriptions.join(", ");
                println!("    {} → {}", letter, desc);
            }
        }
    }
    
    if !ctrl_shift_extension.is_empty() {
        println!("  Used by your extension: {}", ctrl_shift_extension.iter().collect::<String>());
    }
    
    // Analyze Cmd+Shift+[Letter] combinations (macOS)
    println!("\nCmd+Shift+[Letter] combinations:");
    println!("  Available: ", );
    let mut cmd_shift_available = Vec::new();
    let mut cmd_shift_firefox = Vec::new();
    let mut cmd_shift_extension = Vec::new();
    
    for letter in 'A'..='Z' {
        let shortcut = format!("cmd+shift+{}", letter.to_lowercase());
        if firefox_used.contains(&shortcut) {
            cmd_shift_firefox.push(letter);
        } else if extension_used.contains(&shortcut) {
            cmd_shift_extension.push(letter);
        } else {
            cmd_shift_available.push(letter);
        }
    }
    
    if !cmd_shift_available.is_empty() {
        println!("{}", cmd_shift_available.iter().collect::<String>());
    } else {
        println!("(none)");
    }
    
    if !cmd_shift_firefox.is_empty() {
        println!("  Used by Firefox: {}", cmd_shift_firefox.iter().collect::<String>());
        // Show what each letter does
        println!("  Firefox usage details:");
        for letter in &cmd_shift_firefox {
            let shortcut = format!("cmd+shift+{}", letter.to_lowercase());
            let mut descriptions: Vec<String> = firefox_shortcuts.iter()
                .filter(|s| s.normalized == shortcut)
                .map(|s| s.description.clone())
                .collect();
            // Remove duplicates and empty descriptions
            descriptions.sort();
            descriptions.dedup();
            descriptions.retain(|d| !d.is_empty());
            
            if !descriptions.is_empty() {
                let desc = descriptions.join(", ");
                println!("    {} → {}", letter, desc);
            }
        }
    }
    
    if !cmd_shift_extension.is_empty() {
        println!("  Used by your extension: {}", cmd_shift_extension.iter().collect::<String>());
    }
    
    println!();
    
    Ok(())
}

/// Fetch and parse Firefox DevTools keyboard shortcuts from RST documentation
async fn fetch_firefox_devtools_shortcuts(client: &Client) -> Result<Vec<FirefoxShortcut>> {
    let response = client
        .get(FIREFOX_DEVTOOLS_SHORTCUTS_URL)
        .send()
        .await
        .context("failed to fetch Firefox DevTools shortcuts documentation")?
        .error_for_status()
        .context("Firefox documentation returned an error")?;
    
    let text = response
        .text()
        .await
        .context("failed to read response text")?;
    
    parse_firefox_devtools_shortcuts(&text)
}

/// Fetch and parse Firefox keyboard shortcuts from support page HTML
async fn fetch_firefox_support_shortcuts(client: &Client, url: &str, platform: &str) -> Result<Vec<FirefoxShortcut>> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to fetch Firefox {} shortcuts", platform))?
        .error_for_status()
        .with_context(|| format!("Firefox support page returned an error for {}", platform))?;
    
    let text = response
        .text()
        .await
        .context("failed to read response text")?;
    
    parse_firefox_support_shortcuts(&text, platform)
}

/// Parse Firefox DevTools keyboard shortcuts from RST documentation
fn parse_firefox_devtools_shortcuts(text: &str) -> Result<Vec<FirefoxShortcut>> {
    let mut shortcuts = Vec::new();
    let mut seen = HashSet::new();
    
    let lines: Vec<&str> = text.lines().collect();
    let mut current_description = String::new();
    
    for (_i, line) in lines.iter().enumerate() {
        // Check if this line starts a table row with shortcuts
        // Table rows look like: "  * - Command description"
        if line.trim_start().starts_with("* -") {
            // Extract the description (first column)
            let desc_part = line.trim_start().trim_start_matches("* -").trim();
            // Remove markdown bold markers
            current_description = desc_part.replace("**", "").to_string();
            continue;
        }
        
        // Check if this line contains keyboard shortcuts in :kbd: format
        // These are continuation lines like "    - :kbd:`Ctrl` + :kbd:`Shift` + :kbd:`I`"
        if line.trim_start().starts_with("-") && line.contains(":kbd:`") {
            // Extract all :kbd: directives
            let mut keys = Vec::new();
            for cap in KBD_PATTERN.captures_iter(line) {
                let key = cap.get(1).unwrap().as_str();
                keys.push(key.to_string());
            }
            
            // Build shortcut string intelligently
            // Keys extracted from :kbd: are the actual keys, including when one is "+"
            if !keys.is_empty() {
                let shortcut_str = if keys.len() == 1 {
                    keys[0].clone()
                } else {
                    // Join with + separator, but if a key IS "+", that's fine
                    keys.join("+")
                };
                
                let normalized = normalize_shortcut(&shortcut_str);
                
                // Skip duplicates
                if seen.contains(&normalized) {
                    continue;
                }
                seen.insert(normalized.clone());
                
                shortcuts.push(FirefoxShortcut {
                    shortcut: shortcut_str,
                    normalized,
                    description: clean_description(&current_description),
                });
            }
        }
    }
    
    Ok(shortcuts)
}

/// Parse Firefox keyboard shortcuts from support page HTML
fn parse_firefox_support_shortcuts(html: &str, platform: &str) -> Result<Vec<FirefoxShortcut>> {
    let mut shortcuts = Vec::new();
    
    // Extract the doc-content section
    let content = if let Some(captures) = DOC_CONTENT_PATTERN.captures(html) {
        captures.get(1).unwrap().as_str()
    } else {
        // Fallback to entire HTML if section not found
        html
    };
    
    // Split into lines and look for table rows with shortcuts
    let lines: Vec<&str> = content.lines().collect();
    let mut current_description = String::new();
    
    for i in 0..lines.len() {
        let line = lines[i];
        
        // Look for table rows that start with <tr>
        if line.contains("<tr>") {
            // Read the entire row (may span multiple lines)
            let mut row_content = line.to_string();
            let mut j = i;
            while j < lines.len() && !row_content.contains("</tr>") {
                j += 1;
                if j < lines.len() {
                    row_content.push(' ');
                    row_content.push_str(lines[j]);
                }
            }
            
            // Extract the first <td> for description
            if let Some(first_td_start) = row_content.find("<td>") {
                if let Some(first_td_end) = row_content[first_td_start..].find("</td>") {
                    let td_end = first_td_start + first_td_end;
                    let desc = &row_content[first_td_start+4..td_end];
                    
                    // Remove any HTML tags and clean up
                    let desc_clean = Regex::new(r"<[^>]+>").unwrap().replace_all(desc, "");
                    let cleaned = desc_clean.trim().to_string();
                    
                    // Only update if we got a non-empty description
                    if !cleaned.is_empty() {
                        current_description = cleaned;
                    }
                }
            }
        }
        
        // Look for lines with keyboard shortcuts
        if line.contains("<span class=\"key\">") {
            // Extract all key spans from this line and potentially the next few lines
            let mut full_line = line.to_string();
            
            // Check if the line contains "</td>" - if not, continue reading
            let mut j = i;
            while j < lines.len() && !full_line.contains("</td>") {
                j += 1;
                if j < lines.len() {
                    full_line.push(' ');
                    full_line.push_str(lines[j]);
                }
            }
            
            // Parse platform-specific shortcuts
            // Look for <span class="for" data-for="platform"> sections
            let platform_lower = platform.to_lowercase();
            let platform_check = if platform_lower.contains("mac") {
                "mac"
            } else if platform_lower.contains("win") {
                "win"
            } else if platform_lower.contains("linux") {
                "linux"
            } else {
                ""
            };
            
            // Split by <br> to get different shortcut variations
            let parts: Vec<&str> = full_line.split("<br>").collect();
            
            for part in parts {
                // Skip if this part is for a different platform
                if part.contains("data-for=") {
                    let matches_platform = part.contains(&format!("data-for=\"{}\"", platform_check))
                        || (platform_check != "mac" && part.contains("data-for=\"win,linux\""));
                    
                    if !matches_platform {
                        continue;
                    }
                }
                
                // Skip if it's hidden for this platform
                if part.contains("style=\"display: none;\"") {
                    continue;
                }
                
                // Skip if this part contains no visible keys (might be a platform marker)
                if !part.contains("<span class=\"key\">") {
                    continue;
                }
                
                // Extract keys, but only consecutive ones separated by + or space
                // This helps avoid merging unrelated shortcuts
                let mut keys = Vec::new();
                let mut last_end = 0;
                
                for cap in HTML_KEY_PATTERN.captures_iter(part) {
                    let key = cap.get(1).unwrap().as_str().trim();
                    let match_start = cap.get(0).unwrap().start();
                    
                    // Check if there's too much content between this key and the last
                    // (more than just +, spaces, or other formatting)
                    if !keys.is_empty() {
                        let between = &part[last_end..match_start];
                        // If there's more than 20 chars between keys, start a new shortcut
                        if between.len() > 20 {
                            // Process the previous shortcut
                            if !keys.is_empty() {
                                let shortcut_str = keys.join("+");
                                let normalized = normalize_shortcut(&shortcut_str);
                                
                                if !normalized.is_empty() {
                                    shortcuts.push(FirefoxShortcut {
                                        shortcut: shortcut_str,
                                        normalized,
                                        description: clean_description(&current_description),
                                    });
                                }
                            }
                            // Start new shortcut
                            keys = vec![key.to_string()];
                            last_end = cap.get(0).unwrap().end();
                            continue;
                        }
                    }
                    
                    keys.push(key.to_string());
                    last_end = cap.get(0).unwrap().end();
                }
                
                // Process the final accumulated shortcut
                if !keys.is_empty() {
                    let shortcut_str = keys.join("+");
                    let normalized = normalize_shortcut(&shortcut_str);
                    
                    if !normalized.is_empty() {
                        shortcuts.push(FirefoxShortcut {
                            shortcut: shortcut_str,
                            normalized,
                            description: current_description.clone(),
                        });
                    }
                }
            }
        }
    }
    
    Ok(shortcuts)
}

/// Clean up description text by removing RST markup and fixing formatting
fn clean_description(desc: &str) -> String {
    let mut cleaned = desc.to_string();
    
    // Remove RST :ref:`text <link>` markup - keep only the text part
    let ref_pattern = Regex::new(r":ref:`([^<]+)<[^>]+>`").unwrap();
    cleaned = ref_pattern.replace_all(&cleaned, "$1").to_string();
    
    // Remove footnote markers like [#]_
    let footnote_pattern = Regex::new(r"\s*\[#\]_").unwrap();
    cleaned = footnote_pattern.replace_all(&cleaned, "").to_string();
    
    // Fix various concatenated words patterns
    // These appear when description parsing merges multiple words
    if cleaned.contains("WindowReopen") {
        cleaned = cleaned.replace("WindowReopen", "Window / Reopen");
    }
    if cleaned.contains("TabReopen") {
        cleaned = cleaned.replace("TabReopen", "Tab / Reopen");
    }
    if cleaned.contains("ExitQuit") {
        cleaned = cleaned.replace("ExitQuit", "Exit / Quit");
    }
    
    cleaned.trim().to_string()
}

/// Normalize a keyboard shortcut to a standard format for comparison
fn normalize_shortcut(shortcut: &str) -> String {
    if shortcut.is_empty() {
        return shortcut.to_string();
    }
    
    // Split by + but handle the case where + is the actual key
    // Strategy: split normally, then if we have consecutive empty strings,
    // that means we had ++ which represents the + key
    let raw_parts: Vec<&str> = shortcut.split('+').collect();
    let mut parts: Vec<String> = Vec::new();
    
    let mut i = 0;
    while i < raw_parts.len() {
        let part = raw_parts[i].trim();
        
        if part.is_empty() {
            // Empty part means we hit a + separator
            // Check if this is actually the + key (two consecutive splits = ++)
            if i + 1 < raw_parts.len() && raw_parts[i + 1].trim().is_empty() {
                // This is Ctrl++ pattern - the + is the key
                parts.push("+".to_string());
                i += 2; // Skip both empty parts
                continue;
            }
        } else {
            parts.push(part.to_string());
        }
        i += 1;
    }
    
    // Remove any remaining empty parts
    parts.retain(|p| !p.is_empty());
    
    // Normalize modifier names
    for part in parts.iter_mut() {
        let lower = part.to_lowercase();
        *part = match lower.as_str() {
            "command" | "cmd" | "meta" => "cmd",
            "control" | "ctrl" => "ctrl",
            "alt" | "option" | "opt" => "alt",
            "shift" => "shift",
            other => other,
        }.to_string();
    }
    
    // Sort modifiers to ensure consistent order (except the key itself, which should be last)
    if parts.len() > 1 {
        let key = parts.pop().unwrap();
        parts.sort();
        parts.push(key);
    }
    
    parts.join("+")
}


/// Check a Chrome extension's keyboard shortcuts against Firefox shortcuts
pub fn check_extension_shortcuts(
    chrome_shortcuts: &[String],
    firefox_shortcuts: &[FirefoxShortcut],
) -> Vec<ShortcutConflict> {
    let mut conflicts = Vec::new();
    
    for chrome_shortcut in chrome_shortcuts {
        let normalized_chrome = normalize_shortcut(chrome_shortcut);
        
        for firefox_shortcut in firefox_shortcuts {
            if normalized_chrome == firefox_shortcut.normalized {
                conflicts.push(ShortcutConflict {
                    chrome_shortcut: chrome_shortcut.clone(),
                    firefox_shortcut: firefox_shortcut.clone(),
                    severity: ConflictSeverity::Exact,
                });
            }
        }
    }
    
    conflicts
}

#[derive(Debug)]
struct ShortcutMention {
    file: String,
    line: usize,
}

/// Search for mentions of a keyboard shortcut in source code
fn search_shortcut_in_source(root: &Path, shortcut: &str) -> Result<Vec<ShortcutMention>> {
    let mut mentions = Vec::new();
    let normalized = normalize_shortcut(shortcut);
    let parts: Vec<&str> = normalized.split('+').collect();
    
    if parts.is_empty() {
        return Ok(mentions);
    }
    
    // Build regex pattern: match all keys with 0-100 chars between them (case-insensitive)
    let mut pattern = String::from("(?i)");
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            pattern.push_str(".{0,100}?"); // Non-greedy match of 0-100 chars
        }
        // Escape special regex characters and match the key
        pattern.push_str(&regex::escape(part));
    }
    
    let re = match Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return Ok(mentions),
    };
    
    // Search through all text files
    fn search_in_dir(
        dir: &Path,
        re: &Regex,
        mentions: &mut Vec<ShortcutMention>,
    ) -> Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    let dir_name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    
                    if !dir_name.starts_with('.') &&
                       dir_name != "node_modules" &&
                       dir_name != "target" &&
                       dir_name != "dist" {
                        search_in_dir(&path, re, mentions)?;
                    }
                } else if let Some(ext) = path.extension() {
                    // Only search text files
                    let ext_str = ext.to_str().unwrap_or("");
                    if matches!(ext_str, "html" | "js" | "ts" | "jsx" | "tsx" | "css" | "json" | "md" | "txt") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            // Treat file as single line by replacing newlines with spaces
                            let single_line = content.replace('\n', " ").replace('\r', " ");
                            
                            if re.is_match(&single_line) {
                                // Just report that it was found in this file (line 1)
                                mentions.push(ShortcutMention {
                                    file: path.display().to_string(),
                                    line: 1,
                                });
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    
    search_in_dir(root, &re, &mut mentions)?;
    Ok(mentions)
}

/// Find all manifest.json files in a directory tree
fn find_manifest_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut manifest_files = Vec::new();
    
    fn visit_dirs(dir: &Path, manifest_files: &mut Vec<PathBuf>) -> Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    // Skip common directories that shouldn't contain extensions
                    let dir_name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    
                    if !dir_name.starts_with('.') &&
                       dir_name != "node_modules" &&
                       dir_name != "target" &&
                       dir_name != "dist" {
                        visit_dirs(&path, manifest_files)?;
                    }
                } else if path.file_name().and_then(|n| n.to_str()) == Some("manifest.json") {
                    manifest_files.push(path);
                }
            }
        }
        Ok(())
    }
    
    visit_dirs(root, &mut manifest_files)?;
    Ok(manifest_files)
}

/// Extract keyboard shortcuts from a manifest.json file
fn extract_shortcuts_from_manifest(manifest_path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(manifest_path)
        .context("Failed to read manifest.json")?;
    
    let json: Value = serde_json::from_str(&content)
        .context("Failed to parse manifest.json")?;
    
    let mut shortcuts = Vec::new();
    
    // Look for the "commands" section
    if let Some(commands) = json.get("commands").and_then(|c| c.as_object()) {
        for (_command_name, command_data) in commands {
            if let Some(suggested_key) = command_data.get("suggested_key") {
                // Check for default shortcut
                if let Some(default) = suggested_key.get("default").and_then(|d| d.as_str()) {
                    shortcuts.push(default.to_string());
                }
                
                // Check for platform-specific shortcuts
                for platform in &["windows", "mac", "linux", "chromeos"] {
                    if let Some(shortcut) = suggested_key.get(*platform).and_then(|s| s.as_str()) {
                        if !shortcuts.contains(&shortcut.to_string()) {
                            shortcuts.push(shortcut.to_string());
                        }
                    }
                }
            }
        }
    }
    
    Ok(shortcuts)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_normalize_shortcut() {
        assert_eq!(normalize_shortcut("Ctrl+Shift+I"), "ctrl+shift+i");
        assert_eq!(normalize_shortcut("Command+T"), "cmd+t");
        assert_eq!(normalize_shortcut("Alt+Shift+A"), "alt+shift+a");
        assert_eq!(normalize_shortcut("Ctrl + Shift + K"), "ctrl+shift+k");
        assert_eq!(normalize_shortcut("Ctrl++"), "ctrl++");
        assert_eq!(normalize_shortcut("Cmd++"), "cmd++");
        assert_eq!(normalize_shortcut("Ctrl+-"), "ctrl+-");
    }
    
    #[test]
    fn test_normalize_shortcut_order() {
        // Modifiers should be sorted
        assert_eq!(normalize_shortcut("Shift+Ctrl+A"), "ctrl+shift+a");
        assert_eq!(normalize_shortcut("Alt+Ctrl+Shift+B"), "alt+ctrl+shift+b");
    }
    
    #[test]
    fn test_check_conflicts() {
        let firefox_shortcuts = vec![
            FirefoxShortcut {
                shortcut: "Ctrl+Shift+I".to_string(),
                normalized: "ctrl+shift+i".to_string(),
                description: "Open DevTools".to_string(),
            },
        ];
        
        let chrome_shortcuts = vec![
            "Ctrl+Shift+I".to_string(),
            "Ctrl+Shift+Y".to_string(),
        ];
        
        let conflicts = check_extension_shortcuts(&chrome_shortcuts, &firefox_shortcuts);
        
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].severity, ConflictSeverity::Exact);
    }
}