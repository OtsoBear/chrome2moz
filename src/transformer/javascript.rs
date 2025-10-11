//! AST-based JavaScript/TypeScript transformer
//! 
//! Production-grade transformer using SWC for accurate Chrome → Firefox conversion.
//! Provides superior accuracy and TypeScript support.

use crate::models::{ModifiedFile, FileChange, ChangeType, SelectedDecision};
use crate::transformer::ast::{AstTransformer as CoreAstTransformer};
use anyhow::Result;
use std::path::PathBuf;

/// AST-based JavaScript transformer for Chrome to Firefox conversion
pub struct JavaScriptTransformer {
    decisions: Vec<SelectedDecision>,
    transformer: CoreAstTransformer,
    last_generated_handlers: Vec<String>,
}

impl JavaScriptTransformer {
    /// Create a new AST-based transformer
    pub fn new(decisions: &[SelectedDecision]) -> Self {
        Self {
            decisions: decisions.to_vec(),
            transformer: CoreAstTransformer::new(),
            last_generated_handlers: Vec::new(),
        }
    }
    
    /// Get handlers generated during the last transform
    pub fn get_generated_handlers(&self) -> Option<Vec<String>> {
        if self.last_generated_handlers.is_empty() {
            None
        } else {
            Some(self.last_generated_handlers.clone())
        }
    }
    
    /// Transform with handler injection for content scripts
    pub fn transform_with_handlers(&mut self, content: &str, path: &PathBuf, handlers: &[String]) -> Result<ModifiedFile> {
        let original_content = content.to_string();
        
        // Use the transformer's handler injection method
        let new_content = self.transformer.transform_with_handlers(content, path, handlers)?;
        
        // Generate change description
        let mut changes = vec![
            FileChange {
                line_number: 1,
                change_type: ChangeType::Addition,
                description: format!("Injected {} auto-generated executeScript handler(s)", handlers.len()),
                old_code: None,
                new_code: None,
            }
        ];
        
        // Add other transformations
        if new_content.contains("browser.") && !original_content.contains("browser.") {
            changes.push(FileChange {
                line_number: 1,
                change_type: ChangeType::Modification,
                description: "Converted chrome.* calls to browser.*".to_string(),
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
    
    /// Transform JavaScript/TypeScript code from Chrome to Firefox compatibility
    pub fn transform(&mut self, content: &str, path: &PathBuf) -> Result<ModifiedFile> {
        let original_content = content.to_string();
        
        // Perform AST transformation
        let new_content = self.transformer.transform(content, path)?;
        
        // Store any generated handlers for later injection
        self.last_generated_handlers = self.transformer.get_generated_handlers();
        
        // Generate change description
        let mut changes = Vec::new();
        
        // Analyze what changed
        if new_content != original_content {
            // Count transformations
            let chrome_count = original_content.matches("chrome.").count();
            let browser_count = new_content.matches("browser.").count();
            let transformed_count = browser_count.saturating_sub(original_content.matches("browser.").count());
            
            if transformed_count > 0 {
                changes.push(FileChange {
                    line_number: 1,
                    change_type: ChangeType::Modification,
                    description: format!("Converted {} chrome.* calls to browser.*", transformed_count),
                    old_code: None,
                    new_code: None,
                });
            }
            
            // Check if TypeScript was stripped
            if path.extension().map_or(false, |e| e == "ts" || e == "tsx") {
                let had_types = original_content.contains(": ") && 
                               (original_content.contains("string") || 
                                original_content.contains("number") ||
                                original_content.contains("boolean"));
                                
                if had_types && !new_content.contains(": string") {
                    changes.push(FileChange {
                        line_number: 1,
                        change_type: ChangeType::Modification,
                        description: "Stripped TypeScript type annotations".to_string(),
                        old_code: None,
                        new_code: None,
                    });
                }
            }
            
            // Add polyfill if chrome APIs were used
            if chrome_count > 0 && !original_content.contains("typeof browser === 'undefined'") {
                changes.push(FileChange {
                    line_number: 1,
                    change_type: ChangeType::Addition,
                    description: "Added browser namespace polyfill".to_string(),
                    old_code: None,
                    new_code: Some("Browser namespace compatibility check".to_string()),
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
    use std::path::Path;
    
    #[test]
    fn test_transform_simple_code() {
        let mut transformer = JavaScriptTransformer::new(&[]);
        let code = "chrome.storage.local.get('key');";
        let path = PathBuf::from("test.js");
        
        let result = transformer.transform(code, &path).unwrap();
        
        assert!(result.new_content.contains("browser.storage"));
        assert!(!result.new_content.contains("chrome.storage"));
        assert!(!result.changes.is_empty());
    }
    
    #[test]
    fn test_transform_typescript() {
        let mut transformer = JavaScriptTransformer::new(&[]);
        let code = "const x: string = 'test'; chrome.runtime.id;";
        let path = PathBuf::from("test.ts");
        
        let result = transformer.transform(code, &path).unwrap();
        
        // Should strip types and transform chrome
        assert!(result.new_content.contains("browser.runtime"));
        assert!(!result.new_content.contains(": string"));
        assert!(result.changes.iter().any(|c| c.description.contains("TypeScript")));
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