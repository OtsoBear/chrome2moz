//! JavaScript pass-through transformer
//!
//! NOTE: Firefox natively supports chrome.* namespace, so no transformation needed!
//!
//! Assumptions:
//! - Extensions are pre-compiled from TypeScript to JavaScript
//! - Runtime shims handle all API compatibility
//! - No code transformation needed - just pass through

use crate::models::{ModifiedFile, FileChange, SelectedDecision};
use anyhow::Result;
use std::path::PathBuf;

/// Simple pass-through transformer (no AST parsing needed!)
pub struct JavaScriptTransformer {
    _decisions: Vec<SelectedDecision>,
}

impl JavaScriptTransformer {
    /// Create a new pass-through transformer
    pub fn new(decisions: &[SelectedDecision]) -> Self {
        Self {
            _decisions: decisions.to_vec(),
        }
    }
    
    /// Get handlers generated during the last transform (always empty now)
    pub fn get_generated_handlers(&self) -> Option<Vec<String>> {
        None
    }
    
    /// Pass-through with handler injection (simple string concatenation)
    pub fn transform_with_handlers(&mut self, content: &str, _path: &PathBuf, handlers: &[String]) -> Result<ModifiedFile> {
        let original_content = content.to_string();
        
        // Simple string concatenation - prepend handlers
        let mut new_content = String::new();
        for handler in handlers {
            new_content.push_str(handler);
            new_content.push('\n');
        }
        new_content.push_str(content);
        
        let changes = vec![
            FileChange {
                line_number: 1,
                change_type: crate::models::ChangeType::Addition,
                description: format!("Injected {} handler(s) at top of file", handlers.len()),
                old_code: None,
                new_code: None,
            }
        ];
        
        Ok(ModifiedFile {
            path: _path.clone(),
            original_content,
            new_content,
            changes,
        })
    }
    
    /// Simple pass-through with importScripts() removal and Firefox self-uninstall fix
    pub fn transform(&mut self, content: &str, path: &PathBuf) -> Result<ModifiedFile> {
        let original_content = content.to_string();
        let mut new_content = content.to_string();
        let mut changes = Vec::new();
        
        // Check if this is a background script that might have importScripts()
        let is_background = path.to_string_lossy().contains("background");
        
        if is_background {
            // Remove or comment out importScripts() calls
            // These scripts are now loaded via manifest.background.scripts
            let import_scripts_pattern = regex::Regex::new(r"(?m)^\s*importScripts\s*\([^)]*\)\s*;?\s*$").unwrap();
            
            if import_scripts_pattern.is_match(&new_content) {
                // Comment out the lines instead of removing (safer)
                new_content = import_scripts_pattern.replace_all(&new_content, |caps: &regex::Captures| {
                    format!("// {} // Moved to manifest.background.scripts for Firefox compatibility", &caps[0].trim())
                }).to_string();
                
                changes.push(FileChange {
                    line_number: 0,
                    change_type: crate::models::ChangeType::Modification,
                    description: "Commented out importScripts() calls (scripts now loaded via manifest)".to_string(),
                    old_code: None,
                    new_code: None,
                });
            }
        }
        
        // Remove Firefox self-uninstall behavior
        // The extension calls browser.management.uninstallSelf() when it detects Firefox
        // This prevents the extension from working in Firefox
        
        // Pattern 1: Remove any call to .management.uninstallSelf()
        let uninstall_pattern = regex::Regex::new(
            r"(\w+\.)?browser\.management\.uninstallSelf\(\)|(\w+\.)?management\.uninstallSelf\(\)"
        ).unwrap();
        
        if uninstall_pattern.is_match(&new_content) {
            // Replace with void(0) to maintain code flow
            new_content = uninstall_pattern.replace_all(&new_content,
                "/* DISABLED: browser.management.uninstallSelf() */ void(0)"
            ).to_string();
            
            changes.push(FileChange {
                line_number: 0,
                change_type: crate::models::ChangeType::Modification,
                description: "Disabled browser.management.uninstallSelf() calls for Firefox compatibility".to_string(),
                old_code: None,
                new_code: None,
            });
        }
        
        // Pattern 2: Also check for Firefox-specific conditionals that might disable functionality
        // e.g., if (clipperType !== FirefoxExtension) { doSomething(); }
        // We want to ensure Firefox gets the same behavior as Chrome
        let firefox_check_pattern = regex::Regex::new(
            r"(\.get\(\)\.clipperType|clipperType)\s*!==\s*3"
        ).unwrap();
        
        if firefox_check_pattern.is_match(&new_content) {
            // Log this but don't auto-fix - might break intended behavior
            changes.push(FileChange {
                line_number: 0,
                change_type: crate::models::ChangeType::Modification,
                description: "INFO: Found Firefox-specific conditional check (clipperType !== 3) - manual review may be needed".to_string(),
                old_code: None,
                new_code: None,
            });
        }
        
        Ok(ModifiedFile {
            path: path.clone(),
            original_content,
            new_content,
            changes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transform_simple_code() {
        let mut transformer = JavaScriptTransformer::new(&[]);
        let code = "chrome.storage.local.get('key');";
        let path = PathBuf::from("test.js");
        
        let result = transformer.transform(code, &path).unwrap();
        
        // chrome.* should remain unchanged (Firefox supports it natively)
        assert!(result.new_content.contains("chrome.storage"));
    }
    
    #[test]
    fn test_transform_typescript() {
        let mut transformer = JavaScriptTransformer::new(&[]);
        let code = "const x: string = 'test'; chrome.runtime.id;";
        let path = PathBuf::from("test.ts");
        
        let result = transformer.transform(code, &path).unwrap();
        
        // Should keep chrome.* unchanged (Firefox supports it natively)
        assert!(result.new_content.contains("chrome.runtime"));
        // Note: TypeScript stripping is not implemented as extensions are typically
        // pre-compiled to JS before packaging. Users should compile TS first.
    }
    
    #[test]
    fn test_no_changes_needed() {
        let mut transformer = JavaScriptTransformer::new(&[]);
        let code = "const x = 1; console.log(x);";
        let path = PathBuf::from("test.js");
        
        let result = transformer.transform(code, &path).unwrap();
        
        // Code without chrome APIs should still be valid
        assert!(result.new_content.contains("const x = 1"));
    }
    
    #[test]
    fn test_remove_firefox_uninstall_self() {
        let mut transformer = JavaScriptTransformer::new(&[]);
        // Simulating the pattern from the source code
        let code = r#"
            if (this.clientInfo.get().clipperType === 3) {
                WebExtension.browser.management.uninstallSelf();
                resolve(true);
            }
        "#;
        let path = PathBuf::from("webExtensionWorker.js");
        
        let result = transformer.transform(code, &path).unwrap();
        
        // The uninstallSelf call should be disabled
        assert!(result.new_content.contains("DISABLED"));
        assert!(!result.changes.is_empty());
    }
    
    #[test]
    fn test_remove_standalone_uninstall_self() {
        let mut transformer = JavaScriptTransformer::new(&[]);
        let code = "browser.management.uninstallSelf();";
        let path = PathBuf::from("test.js");
        
        let result = transformer.transform(code, &path).unwrap();
        
        // Should replace with void(0) to avoid breaking code flow
        assert!(result.new_content.contains("void(0)"));
        assert!(result.new_content.contains("DISABLED"));
        assert!(!result.changes.is_empty());
    }
}