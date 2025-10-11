//! Tests for executeScript function → func transformation

use chrome_to_firefox::transformer::ast::{AstParser, CodeGenerator, ExecuteScriptTransformer};
use std::path::Path;
use swc_core::common::GLOBALS;
use swc_core::ecma::visit::VisitMutWith;

fn transform_code(code: &str) -> (String, Vec<String>) {
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
fn test_execute_script_function_to_func() {
    let input = r#"
browser.scripting.executeScript({
    target: { tabId: tabs[0].id },
    function: async (reqId) => {
        const latexContent = await extractLatexContent();
        browser.runtime.sendMessage({ type: "LATEX_EXTRACTED", latexContent: latexContent });
    },
    args: [requestId]
});
"#;
    
    let (output, warnings) = transform_code(input);
    
    // Should rename 'function:' to 'func:'
    assert!(output.contains("func:"), "Output should contain 'func:' instead of 'function:'");
    assert!(!output.contains("function:"), "Output should not contain 'function:' property");
    
    // Should warn about cross-scope call
    assert!(!warnings.is_empty(), "Should have warnings about extractLatexContent");
    assert!(warnings.iter().any(|w| w.contains("extractLatexContent") || w.contains("content-script")));
}

#[test]
fn test_already_has_func() {
    let input = r#"
browser.scripting.executeScript({
    target: { tabId: 123 },
    func: () => console.log('test')
});
"#;
    
    let (output, warnings) = transform_code(input);
    
    // Should keep 'func:' as is
    assert!(output.contains("func:"));
    // Should not have warnings for self-contained code
    assert!(warnings.is_empty());
}

#[test]
fn test_self_contained_function() {
    let input = r#"
browser.scripting.executeScript({
    target: { tabId: 123 },
    function: () => {
        const div = document.querySelector('.test');
        return div.textContent;
    }
});
"#;
    
    let (output, _warnings) = transform_code(input);
    
    // Should rename to func
    assert!(output.contains("func:"));
    assert!(!output.contains("function:"));
}

#[test]
fn test_uses_files_instead() {
    let input = r#"
browser.scripting.executeScript({
    target: { tabId: 123 },
    files: ['content.js']
});
"#;
    
    let (output, warnings) = transform_code(input);
    
    // Should not modify files-based executeScript
    assert!(output.contains("files:"));
    assert!(warnings.is_empty());
}

#[test]
fn test_multiple_execute_script_calls() {
    let input = r#"
browser.scripting.executeScript({
    target: { tabId: 1 },
    function: () => console.log('first')
});

browser.scripting.executeScript({
    target: { tabId: 2 },
    function: () => console.log('second')
});
"#;
    
    let (output, _) = transform_code(input);
    
    // Both should be renamed
    let func_count = output.matches("func:").count();
    assert_eq!(func_count, 2, "Should rename both function properties");
    
    let function_count = output.matches("function:").count();
    assert_eq!(function_count, 0, "Should not have any 'function:' properties left");
}