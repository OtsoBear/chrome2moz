//! Phase 1 tests for AST transformer
//!
//! Tests the core functionality: parsing, code generation, and basic transformations

use chrome_to_firefox::transformer::ast::{AstParser, CodeGenerator, ChromeTransformVisitor};
use swc_core::ecma::visit::VisitMutWith;
use std::path::Path;

#[test]
fn test_parse_javascript() {
    let parser = AstParser::new();
    let code = "const x = 1; chrome.storage.get('key');";
    let result = parser.parse(code, Path::new("test.js"));
    assert!(result.is_ok(), "Should successfully parse JavaScript");
}

#[test]
fn test_parse_typescript() {
    let parser = AstParser::new();
    let code = "const x: string = 'test'; type Foo = { bar: number };";
    let result = parser.parse(code, Path::new("test.ts"));
    assert!(result.is_ok(), "Should successfully parse TypeScript");
}

#[test]
fn test_parse_jsx() {
    let parser = AstParser::new();
    let code = "const Component = () => <div>Hello</div>;";
    let result = parser.parse(code, Path::new("test.jsx"));
    assert!(result.is_ok(), "Should successfully parse JSX");
}

#[test]
fn test_parse_tsx() {
    let parser = AstParser::new();
    let code = "const Component: React.FC = () => <div>Hello</div>;";
    let result = parser.parse(code, Path::new("test.tsx"));
    assert!(result.is_ok(), "Should successfully parse TSX");
}

#[test]
fn test_chrome_to_browser_simple() {
    let parser = AstParser::new();
    let codegen = CodeGenerator::new();
    let mut visitor = ChromeTransformVisitor::new();
    
    let code = "chrome.storage.local.get('key');";
    let mut module = parser.parse(code, Path::new("test.js")).unwrap();
    
    module.visit_mut_with(&mut visitor);
    let result = codegen.generate(&module).unwrap();
    
    assert!(result.contains("browser.storage"), "Should transform chrome to browser");
    assert!(!result.contains("chrome.storage"), "Should not contain chrome anymore");
    assert!(visitor.changes_count() > 0, "Should record changes");
}

#[test]
fn test_chrome_to_browser_multiple() {
    let parser = AstParser::new();
    let codegen = CodeGenerator::new();
    let mut visitor = ChromeTransformVisitor::new();
    
    let code = r#"
        chrome.runtime.sendMessage({});
        chrome.tabs.query({}, callback);
        chrome.storage.local.set({key: 'value'});
    "#;
    let mut module = parser.parse(code, Path::new("test.js")).unwrap();
    
    module.visit_mut_with(&mut visitor);
    let result = codegen.generate(&module).unwrap();
    
    assert!(result.contains("browser.runtime"), "Should transform chrome.runtime");
    assert!(result.contains("browser.tabs"), "Should transform chrome.tabs");
    assert!(result.contains("browser.storage"), "Should transform chrome.storage");
    assert!(!result.contains("chrome."), "Should not contain chrome. anymore");
}

#[test]
fn test_local_chrome_variable_not_transformed() {
    let parser = AstParser::new();
    let codegen = CodeGenerator::new();
    let mut visitor = ChromeTransformVisitor::new();
    
    let code = r#"
        function test() {
            let chrome = { custom: 'object' };
            chrome.custom.method();
        }
    "#;
    let mut module = parser.parse(code, Path::new("test.js")).unwrap();
    
    module.visit_mut_with(&mut visitor);
    let result = codegen.generate(&module).unwrap();
    
    // Local 'chrome' variable should NOT be transformed
    assert!(result.contains("let chrome"), "Should keep local chrome declaration");
    assert!(result.contains("chrome.custom"), "Should keep local chrome usage");
}

#[test]
fn test_typescript_stripping() {
    use swc_core::common::GLOBALS;
    use swc_core::ecma::ast::Program;
    use swc_core::ecma::visit::FoldWith;
    
    GLOBALS.set(&Default::default(), || {
        let parser = AstParser::new();
        let codegen = CodeGenerator::new();
        
        let code = "const x: string = 'test'; const y: number = 42;";
        let module = parser.parse(code, Path::new("test.ts")).unwrap();
        
        // Apply TypeScript stripping - wrap in Program first
        use swc_core::ecma::transforms::typescript::strip;
        let program = Program::Module(module);
        let mut pass = strip(Default::default());
        let program = program.fold_with(&mut pass);
        
        let module = match program {
            Program::Module(m) => m,
            _ => panic!("Expected Module"),
        };
        
        let result = codegen.generate(&module).unwrap();
        
        assert!(!result.contains(": string"), "Should strip string type annotation");
        assert!(!result.contains(": number"), "Should strip number type annotation");
        assert!(result.contains("const x = "), "Should keep variable declaration");
        assert!(result.contains("'test'"), "Should keep string value");
        assert!(result.contains("42"), "Should keep number value");
    });
}

#[test]
fn test_roundtrip_preserves_functionality() {
    let parser = AstParser::new();
    let codegen = CodeGenerator::new();
    
    let code = r#"
        const sum = (a, b) => a + b;
        function multiply(x, y) {
            return x * y;
        }
        const result = sum(2, 3) + multiply(4, 5);
    "#;
    
    let module = parser.parse(code, Path::new("test.js")).unwrap();
    let result = codegen.generate(&module).unwrap();
    
    // Should preserve all function definitions and calls
    assert!(result.contains("sum"), "Should preserve arrow function name");
    assert!(result.contains("multiply"), "Should preserve function declaration");
    assert!(result.contains("a + b"), "Should preserve arrow function body");
    assert!(result.contains("x * y"), "Should preserve function body");
}

#[test]
fn test_nested_scopes_transformation() {
    let parser = AstParser::new();
    let codegen = CodeGenerator::new();
    let mut visitor = ChromeTransformVisitor::new();
    
    let code = r#"
        chrome.runtime.onMessage.addListener((msg) => {
            let chrome = 'local';
            console.log(chrome);
        });
    "#;
    let mut module = parser.parse(code, Path::new("test.js")).unwrap();
    
    module.visit_mut_with(&mut visitor);
    let result = codegen.generate(&module).unwrap();
    
    // Outer chrome should be transformed
    assert!(result.contains("browser.runtime"), "Should transform outer chrome");
    
    // Inner chrome should remain as is
    assert!(result.contains("let chrome"), "Should preserve local chrome variable");
}

#[test]
fn test_chrome_in_strings_not_transformed() {
    let parser = AstParser::new();
    let codegen = CodeGenerator::new();
    let mut visitor = ChromeTransformVisitor::new();
    
    let code = r#"
        const url = "https://chrome.google.com";
        const message = 'Use chrome.storage API';
    "#;
    let mut module = parser.parse(code, Path::new("test.js")).unwrap();
    
    module.visit_mut_with(&mut visitor);
    let result = codegen.generate(&module).unwrap();
    
    // String literals should not be transformed
    assert!(result.contains("chrome.google.com"), "Should preserve chrome in URL");
    assert!(result.contains("chrome.storage API"), "Should preserve chrome in message");
}

#[test]
fn test_complex_typescript_code() {
    use swc_core::common::GLOBALS;
    use swc_core::ecma::ast::Program;
    use swc_core::ecma::visit::{FoldWith, VisitMutWith};
    
    GLOBALS.set(&Default::default(), || {
        let parser = AstParser::new();
        let codegen = CodeGenerator::new();
        let mut visitor = ChromeTransformVisitor::new();
        
        let code = r#"
            interface StorageData {
                key: string;
                value: number;
            }
            
            const getData = async (): Promise<StorageData> => {
                return chrome.storage.local.get('key');
            };
        "#;
        let module = parser.parse(code, Path::new("test.ts")).unwrap();
        
        // Strip TypeScript - wrap in Program first
        use swc_core::ecma::transforms::typescript::strip;
        let program = Program::Module(module);
        let mut pass = strip(Default::default());
        let program = program.fold_with(&mut pass);
        
        let mut module = match program {
            Program::Module(m) => m,
            _ => panic!("Expected Module"),
        };
        
        // Transform chrome to browser
        module.visit_mut_with(&mut visitor);
        
        let result = codegen.generate(&module).unwrap();
        
        // Interface should be removed
        assert!(!result.contains("interface"), "Should remove interface");
        
        // Function should remain with transformed API
        assert!(result.contains("browser.storage"), "Should transform chrome to browser");
        assert!(result.contains("getData"), "Should preserve function name");
    });
}

#[test]
fn test_error_recovery_invalid_syntax() {
    let parser = AstParser::new();
    let code = "const x = ; // Invalid syntax";
    let result = parser.parse(code, Path::new("test.js"));
    
    assert!(result.is_err(), "Should return error for invalid syntax");
}

#[test]
fn test_full_ast_transformer_integration() {
    use chrome_to_firefox::transformer::ast::AstTransformer;
    
    let mut transformer = AstTransformer::new();
    let code = "chrome.runtime.id; const x: string = 'test';";
    
    let result = transformer.transform(code, Path::new("test.ts"));
    assert!(result.is_ok(), "Should successfully transform");
    
    let transformed = result.unwrap();
    assert!(transformed.contains("browser.runtime"), "Should transform API");
    assert!(!transformed.contains(": string"), "Should strip types");
}