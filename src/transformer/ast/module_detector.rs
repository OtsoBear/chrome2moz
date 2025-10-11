//! Module system detection for automatic polyfill injection
//!
//! Detects whether code uses ES Modules, CommonJS, or browser globals.

use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitWith};

/// The type of module system detected in the code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleType {
    /// ES6 Modules (import/export)
    ESModule,
    /// CommonJS (require/module.exports)
    CommonJS,
    /// Browser script (no module system)
    Script,
}

/// Detects the module system used in a file
pub struct ModuleDetector {
    has_import: bool,
    has_export: bool,
    has_require: bool,
    has_module_exports: bool,
}

impl ModuleDetector {
    /// Create a new module detector
    pub fn new() -> Self {
        Self {
            has_import: false,
            has_export: false,
            has_require: false,
            has_module_exports: false,
        }
    }
    
    /// Detect the module type from an AST module
    pub fn detect(module: &Module) -> ModuleType {
        let mut detector = Self::new();
        module.visit_with(&mut detector);
        detector.get_type()
    }
    
    /// Get the detected module type
    fn get_type(&self) -> ModuleType {
        // ES Modules take priority
        if self.has_import || self.has_export {
            ModuleType::ESModule
        } else if self.has_require || self.has_module_exports {
            ModuleType::CommonJS
        } else {
            ModuleType::Script
        }
    }
    
    /// Check if an expression is a require() call
    fn is_require_call(&self, expr: &Expr) -> bool {
        if let Expr::Call(call) = expr {
            if let Callee::Expr(callee) = &call.callee {
                if let Expr::Ident(ident) = &**callee {
                    return ident.sym.as_ref() == "require";
                }
            }
        }
        false
    }
    
    /// Check if an expression is module.exports
    fn is_module_exports(&self, expr: &Expr) -> bool {
        if let Expr::Member(member) = expr {
            if let Expr::Ident(obj) = &*member.obj {
                if obj.sym.as_ref() == "module" {
                    if let MemberProp::Ident(prop) = &member.prop {
                        return prop.sym.as_ref() == "exports";
                    }
                }
            }
        }
        false
    }
}

impl Visit for ModuleDetector {
    // Detect ES Module imports
    fn visit_import_decl(&mut self, _: &ImportDecl) {
        self.has_import = true;
    }
    
    // Detect ES Module exports
    fn visit_export_decl(&mut self, _: &ExportDecl) {
        self.has_export = true;
    }
    
    fn visit_export_default_decl(&mut self, _: &ExportDefaultDecl) {
        self.has_export = true;
    }
    
    fn visit_export_default_expr(&mut self, _: &ExportDefaultExpr) {
        self.has_export = true;
    }
    
    fn visit_export_all(&mut self, _: &ExportAll) {
        self.has_export = true;
    }
    
    fn visit_named_export(&mut self, _: &NamedExport) {
        self.has_export = true;
    }
    
    // Detect CommonJS require()
    fn visit_call_expr(&mut self, call: &CallExpr) {
        if self.is_require_call(&Expr::Call(call.clone())) {
            self.has_require = true;
        }
        call.visit_children_with(self);
    }
    
    // Detect CommonJS module.exports
    fn visit_assign_expr(&mut self, assign: &AssignExpr) {
        // Check if left side is module.exports
        if let AssignTarget::Simple(simple) = &assign.left {
            if let SimpleAssignTarget::Member(member) = simple {
                if let Expr::Ident(obj) = &*member.obj {
                    if obj.sym.as_ref() == "module" {
                        if let MemberProp::Ident(prop) = &member.prop {
                            if prop.sym.as_ref() == "exports" {
                                self.has_module_exports = true;
                            }
                        }
                    }
                }
            }
        }
        assign.visit_children_with(self);
    }
    
    // Detect exports.x = ...
    fn visit_member_expr(&mut self, member: &MemberExpr) {
        if let Expr::Ident(obj) = &*member.obj {
            if obj.sym.as_ref() == "exports" {
                self.has_module_exports = true;
            }
        }
        member.visit_children_with(self);
    }
}

impl Default for ModuleDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::ast::parser::AstParser;
    use std::path::Path;
    
    #[test]
    fn test_detect_es_module_import() {
        let code = "import x from 'y'; console.log(x);";
        let parser = AstParser::new();
        let module = parser.parse(code, Path::new("test.js")).unwrap();
        
        let module_type = ModuleDetector::detect(&module);
        assert_eq!(module_type, ModuleType::ESModule);
    }
    
    #[test]
    fn test_detect_es_module_export() {
        let code = "export const x = 1;";
        let parser = AstParser::new();
        let module = parser.parse(code, Path::new("test.js")).unwrap();
        
        let module_type = ModuleDetector::detect(&module);
        assert_eq!(module_type, ModuleType::ESModule);
    }
    
    #[test]
    fn test_detect_commonjs_require() {
        let code = "const x = require('y'); console.log(x);";
        let parser = AstParser::new();
        let module = parser.parse(code, Path::new("test.js")).unwrap();
        
        let module_type = ModuleDetector::detect(&module);
        assert_eq!(module_type, ModuleType::CommonJS);
    }
    
    #[test]
    fn test_detect_commonjs_exports() {
        let code = "module.exports = { x: 1 };";
        let parser = AstParser::new();
        let module = parser.parse(code, Path::new("test.js")).unwrap();
        
        let module_type = ModuleDetector::detect(&module);
        assert_eq!(module_type, ModuleType::CommonJS);
    }
    
    #[test]
    fn test_detect_script() {
        let code = "chrome.storage.get('key');";
        let parser = AstParser::new();
        let module = parser.parse(code, Path::new("test.js")).unwrap();
        
        let module_type = ModuleDetector::detect(&module);
        assert_eq!(module_type, ModuleType::Script);
    }
}