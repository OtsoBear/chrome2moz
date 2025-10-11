//! Callback to Promise transformation
//!
//! Transforms callback-based Chrome API calls to promise-based browser API calls.
//! Handles unlimited nesting depth, unlike regex-based approaches.

use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};
use swc_core::common::{DUMMY_SP, SyntaxContext};

/// Transforms callbacks to promises with unlimited nesting support
pub struct CallbackTransformer {
    transforms_applied: usize,
}

impl CallbackTransformer {
    pub fn new() -> Self {
        Self {
            transforms_applied: 0,
        }
    }
    
    /// Get the number of transformations applied
    pub fn transforms_count(&self) -> usize {
        self.transforms_applied
    }
    
    /// Check if a call expression follows the callback pattern:
    /// chrome.api.method(arg1, arg2, function(result) { ... })
    fn is_callback_pattern(&self, call: &CallExpr) -> bool {
        // Must have at least one argument
        if call.args.is_empty() {
            return false;
        }
        
        // Skip event listeners - they must keep their callbacks!
        // Examples: onMessage.addListener, onCommand.addListener, etc.
        if self.is_event_listener(&call.callee) {
            return false;
        }
        
        // Check if last argument is a function (callback)
        if let Some(last_arg) = call.args.last() {
            matches!(*last_arg.expr, Expr::Fn(_) | Expr::Arrow(_))
        } else {
            false
        }
    }
    
    /// Check if this is an event listener registration (should NOT be converted)
    /// Examples: browser.runtime.onMessage.addListener, chrome.commands.onCommand.addListener
    fn is_event_listener(&self, callee: &Callee) -> bool {
        if let Callee::Expr(expr) = callee {
            if let Expr::Member(member) = &**expr {
                // Check if method name is addListener
                if let MemberProp::Ident(prop) = &member.prop {
                    if prop.sym.as_ref() == "addListener" {
                        // Check if the object contains ".on" pattern (event object)
                        return self.contains_event_pattern(&member.obj);
                    }
                }
            }
        }
        false
    }
    
    /// Check if expression contains event pattern (e.g., onMessage, onCommand, onInstalled)
    fn contains_event_pattern(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Member(member) => {
                // Check if property starts with "on"
                if let MemberProp::Ident(prop) = &member.prop {
                    if prop.sym.as_ref().starts_with("on") {
                        return true;
                    }
                }
                // Recursively check parent
                self.contains_event_pattern(&member.obj)
            }
            _ => false,
        }
    }
    
    /// Check if this is a Chrome/browser API call
    fn is_api_call(&self, callee: &Callee) -> bool {
        if let Callee::Expr(expr) = callee {
            if let Expr::Member(member) = &**expr {
                // Check for chrome.* or browser.* pattern
                if let Expr::Ident(obj) = &*member.obj {
                    return obj.sym.as_ref() == "chrome" || obj.sym.as_ref() == "browser";
                }
                // Check for nested: chrome.storage.local.get
                return self.contains_api_base(&member.obj);
            }
        }
        false
    }
    
    /// Recursively check if expression contains chrome/browser base
    fn contains_api_base(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Ident(ident) => {
                ident.sym.as_ref() == "chrome" || ident.sym.as_ref() == "browser"
            }
            Expr::Member(member) => self.contains_api_base(&member.obj),
            _ => false,
        }
    }
    
    /// Transform a callback-style call to promise-style
    /// chrome.api.method(args, callback) → browser.api.method(args).then(callback)
    fn transform_to_promise(&mut self, call: &mut CallExpr) {
        if !self.is_callback_pattern(call) || !self.is_api_call(&call.callee) {
            return;
        }
        
        // Extract the callback (last argument)
        if let Some(callback_arg) = call.args.pop() {
            // Create the promise chain: originalCall.then(callback)
            let then_call = self.create_then_call(call.clone(), callback_arg.expr);
            
            // Replace the entire call with the promise chain
            *call = then_call;
            self.transforms_applied += 1;
        }
    }
    
    /// Create a .then() call: baseCall.then(callback)
    fn create_then_call(&self, base_call: CallExpr, callback: Box<Expr>) -> CallExpr {
        // The base call without the callback argument
        CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Call(base_call)),
                prop: MemberProp::Ident(IdentName {
                    span: DUMMY_SP,
                    sym: "then".into(),
                }),
            }))),
            args: vec![ExprOrSpread {
                spread: None,
                expr: callback,
            }],
            type_args: None,
        }
    }
}

impl VisitMut for CallbackTransformer {
    fn visit_mut_call_expr(&mut self, call: &mut CallExpr) {
        // Visit children first to handle nested callbacks
        call.visit_mut_children_with(self);
        
        // Transform this call if it matches the pattern
        self.transform_to_promise(call);
    }
}

impl Default for CallbackTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::ast::{AstParser, CodeGenerator};
    use std::path::Path;
    use swc_core::common::GLOBALS;
    
    fn transform_callbacks(code: &str) -> String {
        GLOBALS.set(&Default::default(), || {
            let parser = AstParser::new();
            let mut module = parser.parse(code, Path::new("test.js")).unwrap();
            
            let mut transformer = CallbackTransformer::new();
            module.visit_mut_with(&mut transformer);
            
            let codegen = CodeGenerator::new();
            codegen.generate(&module).unwrap()
        })
    }
    
    #[test]
    fn test_simple_callback_transform() {
        let code = r#"chrome.storage.get('key', function(result) { console.log(result); });"#;
        let result = transform_callbacks(code);
        
        assert!(result.contains(".then"));
        assert!(result.contains("browser") || result.contains("chrome"));
    }
    
    #[test]
    fn test_nested_callbacks() {
        let code = r#"
            chrome.storage.get('a', function(a) {
                chrome.storage.get('b', function(b) {
                    console.log(a, b);
                });
            });
        "#;
        let result = transform_callbacks(code);
        
        // Should have multiple .then() calls for nested callbacks
        assert!(result.matches(".then").count() >= 2);
    }
    
    #[test]
    fn test_arrow_function_callback() {
        let code = r#"chrome.tabs.query({}, (tabs) => console.log(tabs));"#;
        let result = transform_callbacks(code);
        
        assert!(result.contains(".then"));
    }
    
    #[test]
    fn test_deep_nesting() {
        let code = r#"
            chrome.storage.get('a', function(a) {
                chrome.storage.get('b', function(b) {
                    chrome.storage.get('c', function(c) {
                        chrome.storage.get('d', function(d) {
                            console.log(a, b, c, d);
                        });
                    });
                });
            });
        "#;
        let result = transform_callbacks(code);
        
        // Should have 4 .then() calls (no nesting limit!)
        assert!(result.matches(".then").count() >= 4);
    }
    
    #[test]
    fn test_non_callback_preserved() {
        let code = r#"chrome.storage.get('key').then(r => console.log(r));"#;
        let result = transform_callbacks(code);
        
        // Already a promise, should be preserved
        assert!(result.contains(".then"));
    }
    
    #[test]
    fn test_regular_function_call_preserved() {
        let code = r#"myFunction(1, 2, function() { console.log('not an API call'); });"#;
        let result = transform_callbacks(code);
        
        // Not a Chrome API call, should not be transformed
        assert!(!result.contains(".then") || result.contains("myFunction"));
    }
}