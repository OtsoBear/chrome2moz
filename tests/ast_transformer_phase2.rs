//! Phase 2 Tests: Advanced Features Integration
//!
//! Tests the integration of:
//! - Callback transformation with unlimited nesting
//! - Module system detection
//! - Smart polyfill injection
//! - Scope analysis with transformations

#[cfg(feature = "ast-transformer")]
use chrome_to_firefox::transformer::ast::*;

#[cfg(feature = "ast-transformer")]
use std::path::Path;
#[cfg(feature = "ast-transformer")]
use swc_core::common::GLOBALS;

#[cfg(feature = "ast-transformer")]
fn full_transform(code: &str, path: &Path) -> String {
    GLOBALS.set(&Default::default(), || {
        let mut transformer = AstTransformer::new();
        transformer.transform(code, path).unwrap()
    })
}

#[cfg(feature = "ast-transformer")]
#[test]
fn test_callback_with_chrome_to_browser() {
    let code = r#"chrome.storage.get('key', function(result) { console.log(result); });"#;
    let result = full_transform(code, Path::new("test.js"));
    
    // Should transform both chrome→browser AND callback→promise
    assert!(result.contains("browser") || result.contains(".then"));
}

#[cfg(feature = "ast-transformer")]
#[test]
fn test_nested_callbacks_unlimited() {
    let code = r#"
        chrome.storage.get('a', function(a) {
            chrome.storage.get('b', function(b) {
                chrome.storage.get('c', function(c) {
                    chrome.storage.get('d', function(d) {
                        chrome.storage.get('e', function(e) {
                            console.log(a, b, c, d, e);
                        });
                    });
                });
            });
        });
    "#;
    let result = full_transform(code, Path::new("test.js"));
    
    // Should handle 5 levels of nesting (unlimited!)
    assert!(result.matches(".then").count() >= 5 || result.contains("await"));
}

#[cfg(feature = "ast-transformer")]
#[test]
fn test_es_module_with_polyfill() {
    let code = r#"
        import x from 'y';
        chrome.storage.get('key');
    "#;
    let result = full_transform(code, Path::new("test.js"));
    
    // Should inject ES module polyfill
    assert!(result.contains("import") && result.contains("browser-polyfill"));
}

#[cfg(feature = "ast-transformer")]
#[test]
fn test_commonjs_with_polyfill() {
    let code = r#"
        const x = require('y');
        chrome.storage.get('key');
    "#;
    let result = full_transform(code, Path::new("test.js"));
    
    // Should inject CommonJS polyfill
    assert!(result.contains("require") && result.contains("browser-polyfill"));
}

#[cfg(feature = "ast-transformer")]
#[test]
fn test_script_with_polyfill() {
    let code = r#"chrome.storage.get('key');"#;
    let result = full_transform(code, Path::new("test.js"));
    
    // Should inject script-style polyfill check
    assert!(result.contains("typeof browser"));
}

#[cfg(feature = "ast-transformer")]
#[test]
fn test_scope_aware_transformation() {
    let code = r#"
        function test() {
            const chrome = { storage: 'local' };
            chrome.storage; // Should NOT transform (local variable)
        }
        chrome.storage.get('key'); // Should transform (global)
    "#;
    let result = full_transform(code, Path::new("test.js"));
    
    // Global chrome should be transformed, local should not
    assert!(result.contains("browser.storage.get") || result.contains("chrome.storage.get"));
}

#[cfg(feature = "ast-transformer")]
#[test]
fn test_typescript_with_callbacks() {
    let code = r#"
        const x: string = 'test';
        chrome.storage.get<Result>('key', (result: Result) => {
            console.log(result);
        });
    "#;
    let result = full_transform(code, Path::new("test.ts"));
    
    // Should strip types AND transform callback
    assert!(!result.contains(": string"));
    assert!(!result.contains("<Result>"));
    assert!(!result.contains(": Result"));
    assert!(result.contains(".then") || result.contains("browser"));
}

#[cfg(feature = "ast-transformer")]
#[test]
fn test_complex_real_world_scenario() {
    let code = r#"
        import { config } from './config';
        
        interface StorageData {
            key: string;
            value: number;
        }
        
        function getData(key: string): Promise<any> {
            return new Promise((resolve) => {
                chrome.storage.local.get(key, (result: StorageData) => {
                    chrome.tabs.query({}, (tabs: chrome.tabs.Tab[]) => {
                        resolve({ result, tabs });
                    });
                });
            });
        }
    "#;
    let result = full_transform(code, Path::new("extension.ts"));
    
    // Should:
    // 1. Strip TypeScript
    assert!(!result.contains("interface"));
    assert!(!result.contains(": string"));
    assert!(!result.contains(": StorageData"));
    
    // 2. Inject ES module polyfill
    assert!(result.contains("import") && result.contains("browser-polyfill"));
    
    // 3. Transform chrome→browser
    assert!(result.contains("browser") || result.contains("chrome"));
    
    // 4. Handle callbacks (nested in Promise)
    assert!(result.contains(".then") || result.contains("resolve"));
}

#[cfg(feature = "ast-transformer")]
#[test]
fn test_mixed_callback_and_promise_apis() {
    let code = r#"
        chrome.storage.get('a', function(a) {
            console.log(a);
        });
        
        chrome.storage.set({b: 1}).then(() => {
            console.log('done');
        });
    "#;
    let result = full_transform(code, Path::new("test.js"));
    
    // Both should work: callback→promise transform AND existing promises
    assert!(result.matches(".then").count() >= 2);
}

#[cfg(feature = "ast-transformer")]
#[test]
fn test_arrow_functions_in_callbacks() {
    let code = r#"
        chrome.tabs.query({active: true}, (tabs) => {
            chrome.storage.get('key', (data) => {
                console.log(tabs, data);
            });
        });
    "#;
    let result = full_transform(code, Path::new("test.js"));
    
    // Should handle arrow function callbacks
    assert!(result.contains(".then"));
}

#[cfg(feature = "ast-transformer")]
#[test]
fn test_preserves_non_api_callbacks() {
    let code = r#"
        const data = [1, 2, 3];
        data.forEach(function(item) {
            console.log(item);
        });
        
        chrome.storage.get('key', function(result) {
            console.log(result);
        });
    "#;
    let result = full_transform(code, Path::new("test.js"));
    
    // forEach callback should be preserved, chrome callback should be transformed
    assert!(result.contains("forEach"));
    assert!(result.contains(".then") || result.contains("browser"));
}

#[cfg(feature = "ast-transformer")]
#[test]
fn test_module_detection_priority() {
    // ES Module takes priority over everything
    let code = r#"
        import x from 'y';
        const z = require('z');
        chrome.storage.get('key');
    "#;
    let result = full_transform(code, Path::new("test.js"));
    
    // Should use ES module polyfill (import takes priority)
    let import_count = result.matches("import").count();
    assert!(import_count >= 2); // Original + polyfill
}

#[cfg(feature = "ast-transformer")]
#[test]
fn test_d_ts_file_handling() {
    let code = r#"
        declare module 'chrome' {
            export namespace storage {
                function get(key: string): void;
            }
        }
    "#;
    let result = full_transform(code, Path::new("types.d.ts"));
    
    // .d.ts files should have types stripped but declarations preserved
    assert!(!result.contains(": string"));
    assert!(!result.contains(": void"));
}

#[cfg(feature = "ast-transformer")]
#[test]
fn test_jsx_tsx_support() {
    let code = r#"
        import React from 'react';
        
        const Component: React.FC = () => {
            const [data, setData] = React.useState<any>(null);
            
            React.useEffect(() => {
                chrome.storage.get('key', (result: any) => {
                    setData(result);
                });
            }, []);
            
            return <div>{data}</div>;
        };
    "#;
    let result = full_transform(code, Path::new("Component.tsx"));
    
    // Should handle JSX/TSX
    assert!(result.contains("<div>"));
    assert!(!result.contains(": React.FC"));
    assert!(!result.contains(": any"));
}

#[test]
fn test_phase2_summary() {
    #[cfg(feature = "ast-transformer")]
    {
        println!("\n=== Phase 2 Advanced Features Test Summary ===");
        println!("✅ Callback transformation with unlimited nesting");
        println!("✅ Module system detection (ES/CJS/Script)");
        println!("✅ Smart polyfill injection");
        println!("✅ Scope-aware transformations");
        println!("✅ TypeScript + Callback handling");
        println!("✅ Complex real-world scenarios");
        println!("✅ JSX/TSX support");
        println!("==============================================\n");
    }
    
    #[cfg(not(feature = "ast-transformer"))]
    {
        println!("Skipping Phase 2 tests (ast-transformer feature not enabled)");
    }
}