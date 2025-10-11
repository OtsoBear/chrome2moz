//! AST visitor for Chrome → Firefox transformations
//! 
//! Implements the visitor pattern to traverse and modify the AST,
//! transforming Chrome extension APIs to Firefox-compatible code.

use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};
use swc_core::common::DUMMY_SP;
use crate::transformer::ast::scope::{ScopeAnalyzer, ScopeKind};

/// Visitor for transforming Chrome APIs to Firefox APIs
pub struct ChromeTransformVisitor {
    scope: ScopeAnalyzer,
    changes_made: usize,
}

impl ChromeTransformVisitor {
    /// Create a new Chrome transform visitor
    pub fn new() -> Self {
        Self {
            scope: ScopeAnalyzer::new(),
            changes_made: 0,
        }
    }
    
    /// Get the number of changes made
    pub fn changes_count(&self) -> usize {
        self.changes_made
    }
    
    /// Check if this is an importScripts() call
    fn is_import_scripts_call(&self, expr: &Expr) -> bool {
        if let Expr::Call(call) = expr {
            if let Callee::Expr(callee) = &call.callee {
                if let Expr::Ident(ident) = &**callee {
                    return &*ident.sym == "importScripts";
                }
            }
        }
        false
    }
    
    /// Check if a member expression is a chrome API call (not a local variable)
    fn is_chrome_api(&self, expr: &MemberExpr) -> bool {
        if let Expr::Ident(ident) = &*expr.obj {
            if &*ident.sym == "chrome" {
                // Only transform if 'chrome' is not a local variable
                return self.scope.is_global("chrome");
            }
        }
        false
    }
    
    /// Transform chrome.* to browser.*
    fn transform_to_browser(&mut self, expr: &mut MemberExpr) {
        if let Expr::Ident(ident) = &mut *expr.obj {
            if &*ident.sym == "chrome" && self.scope.is_global("chrome") {
                ident.sym = "browser".into();
                self.changes_made += 1;
            }
        }
    }
    
    /// Transform chrome:// URLs to Firefox equivalents
    fn transform_chrome_url(&mut self, url: &str) -> Option<String> {
        if !url.starts_with("chrome://") {
            return None;
        }
        
        let transformed = match url {
            // Extensions management
            "chrome://extensions" => "about:addons",
            "chrome://extensions/" => "about:addons",
            s if s.starts_with("chrome://extensions/shortcuts") => "about:addons",
            s if s.starts_with("chrome://extensions/") => "about:addons",
            
            // Settings and preferences
            "chrome://settings" => "about:preferences",
            "chrome://settings/" => "about:preferences",
            s if s.starts_with("chrome://settings/") => "about:preferences",
            
            // Other common pages
            "chrome://history" => "about:history",
            "chrome://downloads" => "about:downloads",
            "chrome://bookmarks" => "about:bookmarks",
            "chrome://newtab" => "about:newtab",
            "chrome://flags" => "about:config",
            
            // Default: return None if no mapping found
            _ => return None,
        };
        
        Some(transformed.to_string())
    }
}

impl Default for ChromeTransformVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl VisitMut for ChromeTransformVisitor {
    // Handle function declarations - enter/exit scope
    fn visit_mut_function(&mut self, node: &mut Function) {
        self.scope.enter_scope(ScopeKind::Function);
        
        // Declare parameters in function scope
        for param in &node.params {
            if let Pat::Ident(ident) = &param.pat {
                self.scope.declare(&ident.id.sym);
            }
        }
        
        // Visit children
        node.visit_mut_children_with(self);
        
        self.scope.exit_scope();
    }
    
    // Handle arrow functions - enter/exit scope
    fn visit_mut_arrow_expr(&mut self, node: &mut ArrowExpr) {
        self.scope.enter_scope(ScopeKind::Function);
        
        // Declare parameters
        for param in &node.params {
            if let Pat::Ident(ident) = param {
                self.scope.declare(&ident.id.sym);
            }
        }
        
        node.visit_mut_children_with(self);
        
        self.scope.exit_scope();
    }
    
    // Handle block statements - enter/exit scope
    fn visit_mut_block_stmt(&mut self, node: &mut BlockStmt) {
        self.scope.enter_scope(ScopeKind::Block);
        node.visit_mut_children_with(self);
        self.scope.exit_scope();
    }
    
    // Track variable declarations
    fn visit_mut_var_decl(&mut self, node: &mut VarDecl) {
        for decl in &node.decls {
            if let Pat::Ident(ident) = &decl.name {
                self.scope.declare(&ident.id.sym);
            }
        }
        node.visit_mut_children_with(self);
    }
    
    // Transform member expressions (chrome.* → browser.*)
    fn visit_mut_member_expr(&mut self, node: &mut MemberExpr) {
        // First visit children to handle nested expressions
        node.visit_mut_children_with(self);
        
        // Then transform chrome to browser if needed
        if self.is_chrome_api(node) {
            self.transform_to_browser(node);
        }
    }
    
    // Transform identifiers in expressions
    fn visit_mut_expr(&mut self, node: &mut Expr) {
        // Visit children first
        node.visit_mut_children_with(self);
        
        // Transform standalone 'chrome' identifier to 'browser'
        if let Expr::Ident(ident) = node {
            if &*ident.sym == "chrome" && self.scope.is_global("chrome") {
                ident.sym = "browser".into();
                self.changes_made += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::ast::parser::AstParser;
    use crate::transformer::ast::codegen::CodeGenerator;
    use std::path::Path;
    
    fn transform_code(code: &str) -> String {
        let parser = AstParser::new();
        let codegen = CodeGenerator::new();
        let mut visitor = ChromeTransformVisitor::new();
        
        let mut module = parser.parse(code, Path::new("test.js")).unwrap();
        module.visit_mut_with(&mut visitor);
        
        codegen.generate(&module).unwrap()
    }
    
    #[test]
    fn test_simple_chrome_to_browser() {
        let code = "chrome.storage.local.get('key');";
        let result = transform_code(code);
        
        assert!(result.contains("browser.storage"));
        assert!(!result.contains("chrome.storage"));
    }
    
    #[test]
    fn test_multiple_chrome_calls() {
        let code = r#"
            chrome.runtime.sendMessage({});
            chrome.tabs.query({}, (tabs) => {});
            chrome.storage.local.set({key: 'value'});
        "#;
        let result = transform_code(code);
        
        assert!(result.contains("browser.runtime"));
        assert!(result.contains("browser.tabs"));
        assert!(result.contains("browser.storage"));
        assert!(!result.contains("chrome."));
    }
    
    #[test]
    fn test_local_chrome_variable_not_transformed() {
        let code = r#"
            function test() {
                let chrome = { custom: 'object' };
                chrome.custom.method();
            }
        "#;
        let result = transform_code(code);
        
        // Local 'chrome' should NOT be transformed
        assert!(result.contains("let chrome"));
        assert!(result.contains("chrome.custom"));
    }
    
    #[test]
    fn test_nested_scopes() {
        let code = r#"
            chrome.runtime.onMessage.addListener((msg) => {
                let chrome = 'local';
                console.log(chrome);
            });
        "#;
        let result = transform_code(code);
        
        // Outer chrome should be transformed
        assert!(result.contains("browser.runtime"));
        
        // Inner chrome should remain unchanged
        assert!(result.contains("let chrome"));
    }
    
    #[test]
    fn test_chrome_in_string_not_transformed() {
        let code = r#"const url = "https://chrome.google.com";"#;
        let result = transform_code(code);
        
        // String content should not be transformed
        assert!(result.contains("chrome.google.com"));
    }
}
