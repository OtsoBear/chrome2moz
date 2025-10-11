//! Code generation from AST
//! 
//! Converts modified AST back to JavaScript/TypeScript source code.

use anyhow::Result;

use swc_core::common::{SourceMap, FilePathMapping, sync::Lrc};
use swc_core::ecma::ast::Module;
use swc_core::ecma::codegen::{Emitter, Config, text_writer::JsWriter};

/// Code generator for producing JavaScript from AST
pub struct CodeGenerator {
    source_map: Lrc<SourceMap>,
}

impl CodeGenerator {
    /// Create a new code generator
    pub fn new() -> Self {
        Self {
            source_map: Lrc::new(SourceMap::new(FilePathMapping::empty())),
        }
    }
    
    /// Generate JavaScript code from an AST module
    pub fn generate(&self, module: &Module) -> Result<String> {
        use swc_core::common::GLOBALS;
        
        GLOBALS.set(&Default::default(), || {
            let mut buf = vec![];
            
            {
                let writer = JsWriter::new(
                    self.source_map.clone(),
                    "\n",
                    &mut buf,
                    None,
                );
                
                let mut emitter = Emitter {
                    cfg: Config::default(),
                    cm: self.source_map.clone(),
                    comments: None,
                    wr: Box::new(writer),
                };
                
                emitter.emit_module(module)
                    .map_err(|e| anyhow::anyhow!("Code generation error: {:?}", e))?;
            }
            
            String::from_utf8(buf)
                .map_err(|e| anyhow::anyhow!("UTF-8 conversion error: {}", e))
        })
    }
    
    /// Generate with custom configuration
    pub fn generate_with_config(&self, module: &Module, config: Config) -> Result<String> {
        use swc_core::common::GLOBALS;
        
        GLOBALS.set(&Default::default(), || {
            let mut buf = vec![];
            
            {
                let writer = JsWriter::new(
                    self.source_map.clone(),
                    "\n",
                    &mut buf,
                    None,
                );
                
                let mut emitter = Emitter {
                    cfg: config,
                    cm: self.source_map.clone(),
                    comments: None,
                    wr: Box::new(writer),
                };
                
                emitter.emit_module(module)
                    .map_err(|e| anyhow::anyhow!("Code generation error: {:?}", e))?;
            }
            
            String::from_utf8(buf)
                .map_err(|e| anyhow::anyhow!("UTF-8 conversion error: {}", e))
        })
    }
    
    /// Get the source map
    pub fn source_map(&self) -> Lrc<SourceMap> {
        self.source_map.clone()
    }
}

impl Default for CodeGenerator {
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
    fn test_roundtrip_simple_code() {
        let parser = AstParser::new();
        let codegen = CodeGenerator::new();
        
        let original = "const x = 1;\nconst y = 2;";
        let module = parser.parse(original, Path::new("test.js")).unwrap();
        let generated = codegen.generate(&module).unwrap();
        
        // Generated code should be functionally equivalent
        assert!(generated.contains("const x = 1"));
        assert!(generated.contains("const y = 2"));
    }
    
    #[test]
    fn test_generate_from_typescript() {
        let parser = AstParser::new();
        let codegen = CodeGenerator::new();
        
        let ts_code = "const x: number = 42;";
        let module = parser.parse(ts_code, Path::new("test.ts")).unwrap();
        let generated = codegen.generate(&module).unwrap();
        
        // Should include the type annotation in output (stripping happens in visitor)
        assert!(generated.contains("x"));
        assert!(generated.contains("42"));
    }
    
    #[test]
    fn test_generate_functions() {
        let parser = AstParser::new();
        let codegen = CodeGenerator::new();
        
        let code = "function hello(name) { return 'Hello ' + name; }";
        let module = parser.parse(code, Path::new("test.js")).unwrap();
        let generated = codegen.generate(&module).unwrap();
        
        assert!(generated.contains("function hello"));
        assert!(generated.contains("return"));
    }
}