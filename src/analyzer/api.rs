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
                    issues.push(
                        Incompatibility::new(
                            Severity::Major,
                            IncompatibilityCategory::ChromeOnlyApi,
                            Location::FileLocation(path.clone(), call.line),
                            format!("Chrome-only API detected: {}", call.api_name)
                        )
                        .with_suggestion("This API is not available in Firefox. Consider alternative approaches.")
                    );
                }
                
                // Check for callback-style APIs
                if call.is_callback_style {
                    issues.push(
                        Incompatibility::new(
                            Severity::Minor,
                            IncompatibilityCategory::CallbackVsPromise,
                            Location::FileLocation(path.clone(), call.line),
                            format!("Callback-style API: {}", call.api_name)
                        )
                        .with_suggestion("Firefox prefers promise-based APIs. Consider converting to promises.")
                        .auto_fixable()
                    );
                }
                
                // Check for chrome namespace usage
                if call.api_name.starts_with("chrome.") {
                    issues.push(
                        Incompatibility::new(
                            Severity::Info,
                            IncompatibilityCategory::ApiNamespace,
                            Location::FileLocation(path.clone(), call.line),
                            format!("Chrome namespace usage: {}", call.api_name)
                        )
                        .with_suggestion("Will be converted to browser namespace")
                        .auto_fixable()
                    );
                }
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