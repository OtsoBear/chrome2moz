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
    
    /// Simple pass-through with importScripts() removal
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
}