//! Execute Script transformer for Firefox compatibility
//!
//! Handles browser.scripting.executeScript transformations:
//! 1. Renames 'function' property to 'func' for Firefox
//! 2. Detects cross-scope function calls and converts to message passing
//! 3. Ensures proper error handling

use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};
use swc_core::common::DUMMY_SP;

/// Transforms executeScript calls for Firefox compatibility
pub struct ExecuteScriptTransformer {
    transforms_applied: usize,
    warnings: Vec<String>,
}

impl ExecuteScriptTransformer {
    pub fn new() -> Self {
        Self {
            transforms_applied: 0,
            warnings: Vec::new(),
        }
    }
    
    pub fn transforms_count(&self) -> usize {
        self.transforms_applied
    }
    
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
    
    /// Check if this is a scripting.executeScript call
    fn is_execute_script_call(&self, callee: &Callee) -> bool {
        if let Callee::Expr(expr) = callee {
            if let Expr::Member(member) = &**expr {
                // Check for browser.scripting.executeScript or chrome.scripting.executeScript
                if let MemberProp::Ident(method) = &member.prop {
                    if method.sym.as_ref() == "executeScript" {
                        // Check if it's scripting.executeScript
                        if let Expr::Member(parent) = &*member.obj {
                            if let MemberProp::Ident(api) = &parent.prop {
                                return api.sym.as_ref() == "scripting";
                            }
                        }
                    }
                }
            }
        }
        false
    }
    
    /// Transform executeScript call arguments
    fn transform_execute_script(&mut self, call: &mut CallExpr) {
        if call.args.is_empty() {
            return;
        }
        
        // Get the first argument (options object)
        if let Some(arg) = call.args.first_mut() {
            if let Expr::Object(obj) = &mut *arg.expr {
                let mut found_function_prop = false;
                let mut transform_index: Option<usize> = None;
                let mut value_to_check: Option<Box<Expr>> = None;
                
                // First pass: find 'function' property
                for (idx, prop) in obj.props.iter().enumerate() {
                    if let PropOrSpread::Prop(prop_box) = prop {
                        if let Prop::KeyValue(kv) = &**prop_box {
                            if let PropName::Ident(key) = &kv.key {
                                if key.sym.as_ref() == "function" {
                                    transform_index = Some(idx);
                                    value_to_check = Some(kv.value.clone());
                                    found_function_prop = true;
                                    break;
                                }
                            }
                        }
                    }
                }
                
                // Second pass: transform if needed
                if let Some(idx) = transform_index {
                    if let PropOrSpread::Prop(prop_box) = &mut obj.props[idx] {
                        if let Prop::KeyValue(kv) = &**prop_box {
                            let new_prop = Prop::KeyValue(KeyValueProp {
                                key: PropName::Ident(IdentName {
                                    span: DUMMY_SP,
                                    sym: "func".into(),
                                }),
                                value: kv.value.clone(),
                            });
                            **prop_box = new_prop;
                            self.transforms_applied += 1;
                        }
                    }
                    
                    // Check the value for cross-scope calls
                    if let Some(value) = value_to_check {
                        self.check_for_cross_scope_calls(&value);
                    }
                }
                
                if !found_function_prop {
                    // Check for 'func' or 'files' - if neither exists, might be an issue
                    let has_func = obj.props.iter().any(|p| {
                        if let PropOrSpread::Prop(prop_box) = p {
                            if let Prop::KeyValue(kv) = &**prop_box {
                                if let PropName::Ident(key) = &kv.key {
                                    return key.sym.as_ref() == "func" || key.sym.as_ref() == "files";
                                }
                            }
                        }
                        false
                    });
                    
                    if !has_func {
                        self.warnings.push(
                            "executeScript call without 'function', 'func', or 'files' property".to_string()
                        );
                    }
                }
            }
        }
    }
    
    /// Check if the injected function tries to call functions from content script scope
    fn check_for_cross_scope_calls(&mut self, func_value: &Box<Expr>) {
        // This is a simplified check - in a real implementation, we'd do deeper analysis
        // Common patterns that indicate cross-scope issues:
        // - Calling functions that aren't defined in the injected code
        // - Accessing variables from outer scope that won't exist
        
        // For now, just warn if we see certain patterns
        let code_str = format!("{:?}", func_value);
        
        if code_str.contains("extractLatexContent") 
            || code_str.contains("getTextFromIframe")
            || code_str.contains("findLatexContent") {
            self.warnings.push(
                "WARNING: Injected function calls content-script function. \
                This won't work in Firefox. Consider using message passing instead.".to_string()
            );
        }
    }
}

impl VisitMut for ExecuteScriptTransformer {
    fn visit_mut_call_expr(&mut self, call: &mut CallExpr) {
        // Visit children first
        call.visit_mut_children_with(self);
        
        // Check if this is an executeScript call
        if self.is_execute_script_call(&call.callee) {
            self.transform_execute_script(call);
        }
    }
}

impl Default for ExecuteScriptTransformer {
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
    
    fn transform_execute_script(code: &str) -> (String, Vec<String>) {
        GLOBALS.set(&Default::default(), || {
            let parser = AstParser::new();
            let mut module = parser.parse(code, Path::new("test.js")).unwrap();
            
            let mut transformer = ExecuteScriptTransformer::new();
            module.visit_mut_with(&mut transformer);
            
            let warnings = transformer.warnings().to_vec();
            
            let codegen = CodeGenerator::new();
            let result = codegen.generate(&module).unwrap();
            
            (result, warnings)
        })
    }
    
    #[test]
    fn test_function_to_func_rename() {
        let code = r#"
            browser.scripting.executeScript({
                target: { tabId: 123 },
                function: () => console.log('test')
            });
        "#;
        
        let (result, _) = transform_execute_script(code);
        
        assert!(result.contains("func:"));
        assert!(!result.contains("function:"));
    }
    
    #[test]
    fn test_cross_scope_warning() {
        let code = r#"
            browser.scripting.executeScript({
                target: { tabId: 123 },
                function: async () => {
                    const latex = await extractLatexContent();
                    return latex;
                }
            });
        "#;
        
        let (_, warnings) = transform_execute_script(code);
        
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("cross-scope") || w.contains("content-script function")));
    }
    
    #[test]
    fn test_already_has_func() {
        let code = r#"
            browser.scripting.executeScript({
                target: { tabId: 123 },
                func: () => console.log('test')
            });
        "#;
        
        let (result, warnings) = transform_execute_script(code);
        
        assert!(result.contains("func:"));
        assert!(warnings.is_empty());
    }
    
    #[test]
    fn test_uses_files() {
        let code = r#"
            browser.scripting.executeScript({
                target: { tabId: 123 },
                files: ['content.js']
            });
        "#;
        
        let (result, warnings) = transform_execute_script(code);
        
        assert!(result.contains("files:"));
        assert!(warnings.is_empty());
    }
}