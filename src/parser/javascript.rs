//! JavaScript parsing and analysis using regex patterns

use crate::models::extension::ChromeApiCall;
use crate::models::chrome_api_data::ChromeApiDataset;
use regex::Regex;
use lazy_static::lazy_static;
use anyhow::Result;

pub const CHROME_ONLY_APIS: &[&str] = &[
    // Completely unsupported in Firefox
    "chrome.offscreen",
    "chrome.declarativeContent",
    "chrome.tabGroups",
    "chrome.sidePanel",
    "chrome.action.openPopup",

    // Limited or different implementation in Firefox
    "chrome.declarativeNetRequest",
    "chrome.userScripts",
    "chrome.storage.session",

    // Chrome-specific runtime methods
    "chrome.runtime.getPackageDirectoryEntry",

    // Legacy deprecated APIs (Chrome only)
    "chrome.tabs.getSelected",
    "chrome.tabs.getAllInWindow",

    // Chrome-specific downloads features
    "chrome.downloads.acceptDanger",
    "chrome.downloads.setShelfEnabled",
];

lazy_static! {
    /// Global Chrome API dataset loaded from embedded JSON
    static ref CHROME_API_DATASET: ChromeApiDataset = ChromeApiDataset::load();
    
    // Regex to match chrome.* API calls
    static ref CHROME_API_PATTERN: Regex = Regex::new(
        r"chrome\.([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)\s*\("
    ).unwrap();
    
    // Regex to match chrome.* property access
    static ref CHROME_PROPERTY_PATTERN: Regex = Regex::new(
        r"\bchrome\.([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)"
    ).unwrap();
    
    // Regex to detect callback-style (function as last parameter)
    static ref CALLBACK_PATTERN: Regex = Regex::new(
        r"chrome\.[^(]+\([^)]*,\s*(?:function\s*\(|(?:\w+\s*=>|\(\w+\)\s*=>))"
    ).unwrap();
}

/// Analyze JavaScript code for Chrome API usage
pub fn analyze_javascript(source: &str) -> Result<Vec<ChromeApiCall>> {
    let mut calls = Vec::new();
    
    // Find all chrome.* API calls
    for (line_num, line) in source.lines().enumerate() {
        // Check for API calls
        for cap in CHROME_API_PATTERN.captures_iter(line) {
            let api_name = format!("chrome.{}", &cap[1]);
            let is_callback = CALLBACK_PATTERN.is_match(line);
            let is_chrome_only = is_chrome_only_api(&api_name);
            
            calls.push(ChromeApiCall {
                line: line_num + 1,
                column: cap.get(0).map(|m| m.start()).unwrap_or(0),
                api_name: api_name.clone(),
                full_call: format!("{}(...)", api_name),
                is_callback_style: is_callback,
                is_chrome_only,
            });
        }
        
        // Also check for property access (not just calls)
        for cap in CHROME_PROPERTY_PATTERN.captures_iter(line) {
            let api_name = format!("chrome.{}", &cap[1]);
            
            // Skip if we already found this as a call
            if !calls.iter().any(|c| c.line == line_num + 1 && c.api_name == api_name) {
                calls.push(ChromeApiCall {
                    line: line_num + 1,
                    column: cap.get(0).map(|m| m.start()).unwrap_or(0),
                    api_name: api_name.clone(),
                    full_call: api_name.clone(),
                    is_callback_style: false,
                    is_chrome_only: is_chrome_only_api(&api_name),
                });
            }
        }
    }
    
    Ok(calls)
}

fn is_chrome_only_api(api_name: &str) -> bool {
    // Use the dynamic dataset first
    if CHROME_API_DATASET.is_chrome_only(api_name) {
        return true;
    }
    
    // Fallback to hardcoded list for safety
    CHROME_ONLY_APIS.iter().any(|&chrome_api| api_name.starts_with(chrome_api))
}

/// Get detailed information about a Chrome-only API
pub fn get_chrome_api_info(api_name: &str) -> Option<&'static crate::models::chrome_api_data::ChromeApiInfo> {
    CHROME_API_DATASET.get_info(api_name)
}

pub struct JavaScriptAnalyzer;

impl JavaScriptAnalyzer {
    pub fn new() -> Self {
        Self
    }
    
    pub fn analyze(&self, source: &str) -> Result<Vec<ChromeApiCall>> {
        analyze_javascript(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_chrome_storage() {
        let code = r#"
            chrome.storage.local.get('key', (result) => {
                console.log(result);
            });
        "#;
        
        let calls = analyze_javascript(code).unwrap();
        assert!(!calls.is_empty());
        assert!(calls.iter().any(|c| c.api_name.contains("chrome.storage")));
    }
    
    #[test]
    fn test_detect_callback_style() {
        let code = r#"
            chrome.tabs.query({active: true}, (tabs) => {
                console.log(tabs);
            });
        "#;
        
        let calls = analyze_javascript(code).unwrap();
        let tab_call = calls.iter().find(|c| c.api_name.contains("tabs.query"));
        assert!(tab_call.is_some());
        assert!(tab_call.unwrap().is_callback_style);
    }
    
    #[test]
    fn test_detect_chrome_only_api() {
        let code = r#"
            chrome.offscreen.createDocument({
                url: 'offscreen.html',
                reasons: ['DOM_SCRAPING']
            });
        "#;
        
        let calls = analyze_javascript(code).unwrap();
        let offscreen_call = calls.iter().find(|c| c.api_name.contains("offscreen"));
        assert!(offscreen_call.is_some());
        assert!(offscreen_call.unwrap().is_chrome_only);
    }
}