//! Keyboard shortcut conflict detection for WASM
//! This module checks Chrome extension keyboard shortcuts against Firefox's built-in shortcuts

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use crate::models::Extension;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConflict {
    pub chrome_shortcut: String,
    pub firefox_shortcut: String,
    pub firefox_description: String,
    pub suggested_alternatives: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutAnalysis {
    pub conflicts: Vec<ShortcutConflict>,
    pub safe_shortcuts: Vec<String>,
    pub available_alternatives: Vec<String>,
}

/// Firefox shortcuts database (commonly used shortcuts across all platforms)
fn get_firefox_shortcuts() -> HashMap<String, String> {
    let mut shortcuts = HashMap::new();
    
    // Navigation & Tabs
    shortcuts.insert("ctrl+t".to_string(), "Open New Tab".to_string());
    shortcuts.insert("ctrl+shift+t".to_string(), "Reopen Closed Tab".to_string());
    shortcuts.insert("ctrl+w".to_string(), "Close Tab".to_string());
    shortcuts.insert("ctrl+shift+w".to_string(), "Close Window".to_string());
    shortcuts.insert("ctrl+tab".to_string(), "Next Tab".to_string());
    shortcuts.insert("ctrl+shift+tab".to_string(), "Previous Tab".to_string());
    shortcuts.insert("ctrl+1".to_string(), "Go to Tab 1".to_string());
    shortcuts.insert("ctrl+2".to_string(), "Go to Tab 2".to_string());
    shortcuts.insert("ctrl+3".to_string(), "Go to Tab 3".to_string());
    shortcuts.insert("ctrl+4".to_string(), "Go to Tab 4".to_string());
    shortcuts.insert("ctrl+5".to_string(), "Go to Tab 5".to_string());
    shortcuts.insert("ctrl+6".to_string(), "Go to Tab 6".to_string());
    shortcuts.insert("ctrl+7".to_string(), "Go to Tab 7".to_string());
    shortcuts.insert("ctrl+8".to_string(), "Go to Tab 8".to_string());
    shortcuts.insert("ctrl+9".to_string(), "Go to Last Tab".to_string());
    
    // Navigation
    shortcuts.insert("alt+left".to_string(), "Back".to_string());
    shortcuts.insert("alt+right".to_string(), "Forward".to_string());
    shortcuts.insert("ctrl+r".to_string(), "Reload".to_string());
    shortcuts.insert("ctrl+shift+r".to_string(), "Hard Reload".to_string());
    shortcuts.insert("ctrl+l".to_string(), "Focus Address Bar".to_string());
    shortcuts.insert("ctrl+k".to_string(), "Focus Search Bar".to_string());
    
    // Page operations
    shortcuts.insert("ctrl+f".to_string(), "Find in Page".to_string());
    shortcuts.insert("ctrl+g".to_string(), "Find Next".to_string());
    shortcuts.insert("ctrl+shift+g".to_string(), "Find Previous".to_string());
    shortcuts.insert("ctrl+p".to_string(), "Print".to_string());
    shortcuts.insert("ctrl+s".to_string(), "Save Page".to_string());
    shortcuts.insert("ctrl+plus".to_string(), "Zoom In".to_string());
    shortcuts.insert("ctrl+minus".to_string(), "Zoom Out".to_string());
    shortcuts.insert("ctrl+0".to_string(), "Reset Zoom".to_string());
    
    // Browser UI
    shortcuts.insert("ctrl+h".to_string(), "Show History".to_string());
    shortcuts.insert("ctrl+shift+h".to_string(), "Show Library".to_string());
    shortcuts.insert("ctrl+b".to_string(), "Show Bookmarks".to_string());
    shortcuts.insert("ctrl+shift+b".to_string(), "Show Bookmarks Toolbar".to_string());
    shortcuts.insert("ctrl+d".to_string(), "Bookmark Page".to_string());
    shortcuts.insert("ctrl+shift+d".to_string(), "Bookmark All Tabs".to_string());
    shortcuts.insert("ctrl+j".to_string(), "Show Downloads".to_string());
    shortcuts.insert("ctrl+shift+a".to_string(), "Open Add-ons".to_string());
    
    // Developer tools
    shortcuts.insert("ctrl+shift+a".to_string(), "Add-ons".to_string());
    shortcuts.insert("ctrl+shift+b".to_string(), "Toggle conditional breakpoint".to_string());
    shortcuts.insert("ctrl+shift+c".to_string(), "Inspector".to_string());
    shortcuts.insert("ctrl+shift+d".to_string(), "Toggle toolbox docking mode".to_string());
    shortcuts.insert("ctrl+shift+e".to_string(), "Network Monitor".to_string());
    shortcuts.insert("ctrl+shift+f".to_string(), "Search in all files".to_string());
    shortcuts.insert("ctrl+shift+g".to_string(), "Find Previous".to_string());
    shortcuts.insert("ctrl+shift+h".to_string(), "Library window (History)".to_string());
    shortcuts.insert("ctrl+shift+i".to_string(), "Toggle Developer Tools".to_string());
    shortcuts.insert("ctrl+shift+j".to_string(), "Browser Console".to_string());
    shortcuts.insert("ctrl+shift+k".to_string(), "Web Console".to_string());
    shortcuts.insert("ctrl+shift+l".to_string(), "Clear output".to_string());
    shortcuts.insert("ctrl+shift+m".to_string(), "Responsive Design Mode".to_string());
    shortcuts.insert("ctrl+shift+n".to_string(), "Undo Close Window".to_string());
    shortcuts.insert("ctrl+shift+o".to_string(), "Show All Bookmarks (Library)".to_string());
    shortcuts.insert("ctrl+shift+p".to_string(), "New Private Window".to_string());
    shortcuts.insert("ctrl+shift+q".to_string(), "Exit / Quit".to_string());
    shortcuts.insert("ctrl+shift+r".to_string(), "Reload (override cache)".to_string());
    shortcuts.insert("ctrl+shift+s".to_string(), "Take a screenshot".to_string());
    shortcuts.insert("ctrl+shift+t".to_string(), "Undo Close Tab".to_string());
    shortcuts.insert("ctrl+shift+v".to_string(), "Paste (as plain text)".to_string());
    shortcuts.insert("ctrl+shift+w".to_string(), "Close Window".to_string());
    shortcuts.insert("ctrl+shift+x".to_string(), "Move URL in address bar".to_string());
    shortcuts.insert("ctrl+shift+y".to_string(), "Downloads".to_string());
    shortcuts.insert("ctrl+shift+z".to_string(), "Open Debugger".to_string());
    
    // Other
    shortcuts.insert("ctrl+n".to_string(), "New Window".to_string());
    shortcuts.insert("ctrl+shift+n".to_string(), "New Private Window".to_string());
    shortcuts.insert("ctrl+q".to_string(), "Quit Firefox".to_string());
    shortcuts.insert("f5".to_string(), "Reload".to_string());
    shortcuts.insert("f11".to_string(), "Toggle Full Screen".to_string());
    shortcuts.insert("f12".to_string(), "Toggle Developer Tools".to_string());
    
    // macOS variants (Cmd instead of Ctrl)
    shortcuts.insert("cmd+t".to_string(), "Open New Tab".to_string());
    shortcuts.insert("cmd+shift+t".to_string(), "Reopen Closed Tab".to_string());
    shortcuts.insert("cmd+w".to_string(), "Close Tab".to_string());
    shortcuts.insert("cmd+shift+w".to_string(), "Close Window".to_string());
    shortcuts.insert("cmd+l".to_string(), "Focus Address Bar".to_string());
    shortcuts.insert("cmd+k".to_string(), "Focus Search Bar".to_string());
    shortcuts.insert("cmd+f".to_string(), "Find in Page".to_string());
    shortcuts.insert("cmd+h".to_string(), "Hide Window".to_string());
    shortcuts.insert("cmd+b".to_string(), "Show Bookmarks".to_string());
    shortcuts.insert("cmd+d".to_string(), "Bookmark Page".to_string());
    shortcuts.insert("cmd+n".to_string(), "New Window".to_string());
    shortcuts.insert("cmd+q".to_string(), "Quit Firefox".to_string());
    
    // macOS Cmd+Shift combinations
    shortcuts.insert("cmd+shift+a".to_string(), "Add-ons".to_string());
    shortcuts.insert("cmd+shift+b".to_string(), "Toggle conditional breakpoint".to_string());
    shortcuts.insert("cmd+shift+c".to_string(), "Inspect Element".to_string());
    shortcuts.insert("cmd+shift+d".to_string(), "Toggle toolbox docking mode".to_string());
    shortcuts.insert("cmd+shift+f".to_string(), "Search in all files".to_string());
    shortcuts.insert("cmd+shift+g".to_string(), "Find Previous".to_string());
    shortcuts.insert("cmd+shift+h".to_string(), "History sidebar".to_string());
    shortcuts.insert("cmd+shift+j".to_string(), "Browser Console".to_string());
    shortcuts.insert("cmd+shift+n".to_string(), "Undo Close Window".to_string());
    shortcuts.insert("cmd+shift+o".to_string(), "Show All Bookmarks (Library)".to_string());
    shortcuts.insert("cmd+shift+p".to_string(), "New Private Window".to_string());
    shortcuts.insert("cmd+shift+r".to_string(), "Reload (override cache)".to_string());
    shortcuts.insert("cmd+shift+s".to_string(), "Take a screenshot".to_string());
    shortcuts.insert("cmd+shift+t".to_string(), "Undo Close Tab".to_string());
    shortcuts.insert("cmd+shift+v".to_string(), "Paste (as plain text)".to_string());
    shortcuts.insert("cmd+shift+w".to_string(), "Close Window".to_string());
    shortcuts.insert("cmd+shift+x".to_string(), "Move URL in address bar".to_string());
    shortcuts.insert("cmd+shift+z".to_string(), "Redo".to_string());
    
    shortcuts
}

/// Normalize a keyboard shortcut to a standard format for comparison
fn normalize_shortcut(shortcut: &str) -> String {
    if shortcut.is_empty() {
        return shortcut.to_string();
    }
    
    let raw_parts: Vec<&str> = shortcut.split('+').collect();
    let mut parts: Vec<String> = Vec::new();
    
    let mut i = 0;
    while i < raw_parts.len() {
        let part = raw_parts[i].trim();
        
        if part.is_empty() {
            if i + 1 < raw_parts.len() && raw_parts[i + 1].trim().is_empty() {
                parts.push("+".to_string());
                i += 2;
                continue;
            }
        } else {
            parts.push(part.to_string());
        }
        i += 1;
    }
    
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

/// Extract keyboard shortcuts from a Chrome extension manifest
pub fn extract_shortcuts(extension: &Extension) -> Vec<String> {
    let mut shortcuts = Vec::new();
    
    if let Some(commands) = extension.manifest.commands.as_ref() {
        for (_command_name, command_data) in commands {
            if let Some(suggested_key) = &command_data.suggested_key {
                // Collect all shortcuts from the HashMap
                for (_platform, shortcut) in suggested_key {
                    if !shortcuts.contains(shortcut) {
                        shortcuts.push(shortcut.clone());
                    }
                }
            }
        }
    }
    
    shortcuts
}

/// Generate alternative shortcut suggestions
fn generate_alternatives(conflicted: &HashSet<String>) -> Vec<String> {
    let mut alternatives = Vec::new();
    
    // Try Ctrl+Shift+[Letter] combinations
    for letter in 'A'..='Z' {
        let shortcut = format!("ctrl+shift+{}", letter.to_lowercase());
        if !conflicted.contains(&shortcut) {
            alternatives.push(format!("Ctrl+Shift+{}", letter));
        }
    }
    
    // Try Alt+Shift+[Letter] combinations
    for letter in 'A'..='Z' {
        let shortcut = format!("alt+shift+{}", letter.to_lowercase());
        if !conflicted.contains(&shortcut) {
            alternatives.push(format!("Alt+Shift+{}", letter));
        }
    }
    
    // Limit to reasonable number
    alternatives.truncate(20);
    alternatives
}

/// Analyze keyboard shortcuts for conflicts with Firefox
pub fn analyze_shortcuts(extension: &Extension) -> ShortcutAnalysis {
    let chrome_shortcuts = extract_shortcuts(extension);
    let firefox_shortcuts = get_firefox_shortcuts();
    
    let mut conflicts = Vec::new();
    let mut safe_shortcuts = Vec::new();
    let mut conflicted_normalized = HashSet::new();
    
    for chrome_shortcut in &chrome_shortcuts {
        let normalized = normalize_shortcut(chrome_shortcut);
        
        if let Some(firefox_desc) = firefox_shortcuts.get(&normalized) {
            conflicted_normalized.insert(normalized.clone());
            conflicts.push(ShortcutConflict {
                chrome_shortcut: chrome_shortcut.clone(),
                firefox_shortcut: normalized,
                firefox_description: firefox_desc.clone(),
                suggested_alternatives: Vec::new(), // Will be filled below
            });
        } else {
            safe_shortcuts.push(chrome_shortcut.clone());
        }
    }
    
    // Generate alternatives for all conflicted shortcuts
    let available_alternatives = generate_alternatives(&conflicted_normalized);
    
    // Add alternatives to each conflict
    for conflict in &mut conflicts {
        conflict.suggested_alternatives = available_alternatives.clone();
    }
    
    ShortcutAnalysis {
        conflicts,
        safe_shortcuts,
        available_alternatives,
    }
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
    }
    
    #[test]
    fn test_conflict_detection() {
        let firefox_shortcuts = get_firefox_shortcuts();
        assert!(firefox_shortcuts.contains_key("ctrl+shift+i"));
        assert!(firefox_shortcuts.contains_key("ctrl+t"));
    }
}