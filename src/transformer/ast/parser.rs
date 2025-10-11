//! AST Parser for JavaScript and TypeScript
//! 
//! Provides parsing capabilities with auto-detection of file types
//! and comprehensive error recovery.

use anyhow::{anyhow, Result};
use std::path::Path;

use swc_core::common::{SourceMap, FileName, FilePathMapping, sync::Lrc};
use swc_core::ecma::parser::{Parser, StringInput, Syntax, TsConfig, EsConfig};
use swc_core::ecma::ast::Module;

/// AST parser with TypeScript and JSX support
pub struct AstParser {
    source_map: Lrc<SourceMap>,
}

impl AstParser {
    /// Create a new AST parser
    pub fn new() -> Self {
        Self {
            source_map: Lrc::new(SourceMap::new(FilePathMapping::empty())),
        }
    }
    
    /// Parse JavaScript or TypeScript code into an AST
    /// 
    /// Automatically detects the syntax based on file extension:
    /// - `.ts` → TypeScript
    /// - `.tsx` → TypeScript with JSX
    /// - `.jsx` → JavaScript with JSX  
    /// - `.js` or other → JavaScript
    pub fn parse(&self, code: &str, path: &Path) -> Result<Module> {
        use swc_core::common::GLOBALS;
        
        GLOBALS.set(&Default::default(), || {
            let syntax = self.detect_syntax(path);
            
            let source_file = self.source_map.new_source_file(
                FileName::Real(path.to_path_buf()),
                code.to_string(),
            );
            
            let input = StringInput::from(&*source_file);
            let mut parser = Parser::new(syntax, input, None);
            
            parser
                .parse_module()
                .map_err(|e| anyhow!("Parse error at {:?}: {:?}", path, e))
        })
    }
    
    /// Detect syntax mode based on file extension
    fn detect_syntax(&self, path: &Path) -> Syntax {
        match path.extension().and_then(|s| s.to_str()) {
            Some("ts") => Syntax::Typescript(TsConfig {
                tsx: false,
                decorators: true,
                dts: false,
                no_early_errors: true,
                disallow_ambiguous_jsx_like: false,
            }),
            Some("tsx") => Syntax::Typescript(TsConfig {
                tsx: true,
                decorators: true,
                dts: false,
                no_early_errors: true,
                disallow_ambiguous_jsx_like: false,
            }),
            Some("d.ts") => Syntax::Typescript(TsConfig {
                tsx: false,
                decorators: true,
                dts: true,
                no_early_errors: true,
                disallow_ambiguous_jsx_like: false,
            }),
            Some("jsx") => Syntax::Es(EsConfig {
                jsx: true,
                fn_bind: false,
                decorators: false,
                decorators_before_export: false,
                export_default_from: true,
                import_attributes: true,
                allow_super_outside_method: false,
                allow_return_outside_function: false,
                auto_accessors: false,
                explicit_resource_management: false,
            }),
            _ => Syntax::Es(EsConfig {
                jsx: false,
                fn_bind: false,
                decorators: false,
                decorators_before_export: false,
                export_default_from: true,
                import_attributes: true,
                allow_super_outside_method: false,
                allow_return_outside_function: false,
                auto_accessors: false,
                explicit_resource_management: false,
            }),
        }
    }
    
    /// Get the source map for error reporting
    pub fn source_map(&self) -> Lrc<SourceMap> {
        self.source_map.clone()
    }
}

impl Default for AstParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_parse_javascript() {
        let parser = AstParser::new();
        let code = "const x = 1; chrome.storage.get('key');";
        let result = parser.parse(code, Path::new("test.js"));
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_parse_typescript() {
        let parser = AstParser::new();
        let code = "const x: string = 'test'; type Foo = { bar: number };";
        let result = parser.parse(code, Path::new("test.ts"));
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_parse_jsx() {
        let parser = AstParser::new();
        let code = "const Component = () => <div>Hello</div>;";
        let result = parser.parse(code, Path::new("test.jsx"));
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_parse_tsx() {
        let parser = AstParser::new();
        let code = "const Component: React.FC = () => <div>Hello</div>;";
        let result = parser.parse(code, Path::new("test.tsx"));
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_syntax_detection() {
        let parser = AstParser::new();
        
        // TypeScript
        match parser.detect_syntax(Path::new("test.ts")) {
            Syntax::Typescript(config) => assert!(!config.tsx),
            _ => panic!("Expected TypeScript syntax"),
        }
        
        // TSX
        match parser.detect_syntax(Path::new("test.tsx")) {
            Syntax::Typescript(config) => assert!(config.tsx),
            _ => panic!("Expected TypeScript with JSX syntax"),
        }
        
        // JSX
        match parser.detect_syntax(Path::new("test.jsx")) {
            Syntax::Es(config) => assert!(config.jsx),
            _ => panic!("Expected ES with JSX syntax"),
        }
        
        // Plain JS
        match parser.detect_syntax(Path::new("test.js")) {
            Syntax::Es(config) => assert!(!config.jsx),
            _ => panic!("Expected ES syntax"),
        }
    }
}