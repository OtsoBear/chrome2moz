//! JavaScript API analysis

use crate::models::{Incompatibility, Severity, IncompatibilityCategory, Location};
use crate::parser::javascript::analyze_javascript;
use std::path::PathBuf;

pub fn analyze_javascript_apis(content: &str, path: &PathBuf) -> Vec<Incompatibility> {
    let mut issues = Vec::new();
    
    // Parse and analyze JavaScript
    match analyze_javascript(content) {
        Ok(api_calls) => {
            for call in api_calls {
                // Check for Chrome-only APIs
                if call.is_chrome_only {
                    let api_name = &call.api_name;
                    let suggestion = if api_name.contains("storage.session") {
                        "Will provide in-memory polyfill (runtime shim)"
                    } else if api_name.contains("sidePanel") {
                        "Will map to Firefox sidebarAction (runtime shim)"
                    } else if api_name.contains("declarativeNetRequest") {
                        "Will provide stub with guidance to use webRequest API"
                    } else if api_name.contains("tabGroups") {
                        "Will provide no-op stub (Firefox doesn't support tab groups)"
                    } else if api_name.contains("offscreen") {
                        "Chrome-only API. Consider using Web Workers or content scripts"
                    } else {
                        "Chrome-only API. Will include runtime compatibility shim"
                    };
                    
                    issues.push(
                        Incompatibility::new(
                            Severity::Major,
                            IncompatibilityCategory::ChromeOnlyApi,
                            Location::FileLocation(path.clone(), call.line),
                            format!("Chrome-only API: {}", call.api_name)
                        )
                        .with_suggestion(suggestion)
                    );
                }
                
                // Note: We don't report chrome.* namespace usage because Firefox supports it natively!
                // JavaScript passes through unchanged. Runtime shims handle compatibility.
            }
        }
        Err(e) => {
            eprintln!("Failed to analyze {}: {}", path.display(), e);
        }
    }
    
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_chrome_only_api() {
        let code = r#"
            chrome.offscreen.createDocument({
                url: 'offscreen.html'
            });
        "#;
        
        let path = PathBuf::from("test.js");
        let issues = analyze_javascript_apis(code, &path);
        
        assert!(issues.iter().any(|i| matches!(i.category, IncompatibilityCategory::ChromeOnlyApi)));
    }
}