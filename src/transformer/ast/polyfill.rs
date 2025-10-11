//! Smart polyfill injection based on detected module type
//!
//! Automatically injects the appropriate browser polyfill based on
//! whether the code uses ES Modules, CommonJS, or browser globals.

use swc_core::ecma::ast::*;
use swc_core::common::DUMMY_SP;
use crate::transformer::ast::module_detector::ModuleType;

/// Polyfill injector that adds appropriate browser-polyfill imports
pub struct PolyfillInjector {
    polyfill_path: String,
}

impl PolyfillInjector {
    /// Create a new polyfill injector
    pub fn new() -> Self {
        Self {
            polyfill_path: "./browser-polyfill.js".to_string(),
        }
    }
    
    /// Create with custom polyfill path
    pub fn with_path(path: String) -> Self {
        Self {
            polyfill_path: path,
        }
    }
    
    /// Inject polyfill into module based on detected type
    pub fn inject(&self, module: &mut Module, module_type: ModuleType) {
        let polyfill_item = match module_type {
            ModuleType::ESModule => self.create_es_import(),
            ModuleType::CommonJS => self.create_commonjs_require(),
            ModuleType::Script => self.create_script_check(),
        };
        
        // Insert at the beginning of the module
        module.body.insert(0, polyfill_item);
    }
    
    /// Create ES Module import: import './browser-polyfill.js';
    fn create_es_import(&self) -> ModuleItem {
        ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
            span: DUMMY_SP,
            specifiers: vec![],
            src: Box::new(Str {
                span: DUMMY_SP,
                value: self.polyfill_path.clone().into(),
                raw: None,
            }),
            type_only: false,
            with: None,
            phase: Default::default(),
        }))
    }
    
    /// Create CommonJS require: require('./browser-polyfill.js');
    fn create_commonjs_require(&self) -> ModuleItem {
        ModuleItem::Stmt(Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Call(CallExpr {
                span: DUMMY_SP,
                callee: Callee::Expr(Box::new(Expr::Ident(Ident {
                    span: DUMMY_SP,
                    sym: "require".into(),
                    optional: false,
                }))),
                args: vec![ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(Str {
                        span: DUMMY_SP,
                        value: self.polyfill_path.clone().into(),
                        raw: None,
                    }))),
                }],
                type_args: None,
            })),
        }))
    }
    
    /// Create script check: if (typeof browser === 'undefined') { this.browser = this.chrome; }
    fn create_script_check(&self) -> ModuleItem {
        ModuleItem::Stmt(Stmt::If(IfStmt {
            span: DUMMY_SP,
            test: Box::new(Expr::Bin(BinExpr {
                span: DUMMY_SP,
                op: BinaryOp::EqEqEq,
                left: Box::new(Expr::Unary(UnaryExpr {
                    span: DUMMY_SP,
                    op: UnaryOp::TypeOf,
                    arg: Box::new(Expr::Ident(Ident {
                        span: DUMMY_SP,
                        sym: "browser".into(),
                        optional: false,
                    })),
                })),
                right: Box::new(Expr::Lit(Lit::Str(Str {
                    span: DUMMY_SP,
                    value: "undefined".into(),
                    raw: None,
                }))),
            })),
            cons: Box::new(Stmt::Block(BlockStmt {
                span: DUMMY_SP,
                stmts: vec![Stmt::Expr(ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(Expr::Assign(AssignExpr {
                        span: DUMMY_SP,
                        op: AssignOp::Assign,
                        left: AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
                            span: DUMMY_SP,
                            obj: Box::new(Expr::This(ThisExpr { span: DUMMY_SP })),
                            prop: MemberProp::Ident(Ident {
                                span: DUMMY_SP,
                                sym: "browser".into(),
                                optional: false,
                            }),
                        })),
                        right: Box::new(Expr::Member(MemberExpr {
                            span: DUMMY_SP,
                            obj: Box::new(Expr::This(ThisExpr { span: DUMMY_SP })),
                            prop: MemberProp::Ident(Ident {
                                span: DUMMY_SP,
                                sym: "chrome".into(),
                                optional: false,
                            }),
                        })),
                    })),
                })],
            })),
            alt: None,
        }))
    }
}

impl Default for PolyfillInjector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::ast::{AstParser, CodeGenerator, ModuleDetector};
    use std::path::Path;
    
    #[test]
    fn test_inject_es_module() {
        let code = "export const x = 1;";
        let parser = AstParser::new();
        let mut module = parser.parse(code, Path::new("test.js")).unwrap();
        
        let module_type = ModuleDetector::detect(&module);
        let injector = PolyfillInjector::new();
        injector.inject(&mut module, module_type);
        
        let codegen = CodeGenerator::new();
        let result = codegen.generate(&module).unwrap();
        
        assert!(result.contains("import"));
        assert!(result.contains("browser-polyfill"));
    }
    
    #[test]
    fn test_inject_commonjs() {
        let code = "const x = require('y');";
        let parser = AstParser::new();
        let mut module = parser.parse(code, Path::new("test.js")).unwrap();
        
        let module_type = ModuleDetector::detect(&module);
        let injector = PolyfillInjector::new();
        injector.inject(&mut module, module_type);
        
        let codegen = CodeGenerator::new();
        let result = codegen.generate(&module).unwrap();
        
        assert!(result.contains("require"));
        assert!(result.contains("browser-polyfill"));
    }
    
    #[test]
    fn test_inject_script() {
        let code = "chrome.storage.get('key');";
        let parser = AstParser::new();
        let mut module = parser.parse(code, Path::new("test.js")).unwrap();
        
        let module_type = ModuleDetector::detect(&module);
        let injector = PolyfillInjector::new();
        injector.inject(&mut module, module_type);
        
        let codegen = CodeGenerator::new();
        let result = codegen.generate(&module).unwrap();
        
        assert!(result.contains("typeof browser"));
        assert!(result.contains("this.browser = this.chrome"));
    }
    
    #[test]
    fn test_custom_polyfill_path() {
        let code = "export const x = 1;";
        let parser = AstParser::new();
        let mut module = parser.parse(code, Path::new("test.js")).unwrap();
        
        let module_type = ModuleDetector::detect(&module);
        let injector = PolyfillInjector::with_path("../polyfill.js".to_string());
        injector.inject(&mut module, module_type);
        
        let codegen = CodeGenerator::new();
        let result = codegen.generate(&module).unwrap();
        
        assert!(result.contains("../polyfill.js"));
    }
}