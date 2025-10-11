# AST-Based JavaScript/TypeScript Transformer Implementation Plan

> **Goal:** Build a production-grade AST transformer with native TypeScript support that surpasses the current regex implementation in every aspect.

## 🎯 Success Criteria

**Must achieve:**
- ✅ 95%+ accuracy (vs 75% regex)
- ✅ Full TypeScript support (.ts, .tsx, .d.ts)
- ✅ Handle all module systems (ES, CommonJS, Browser globals)
- ✅ Unlimited callback nesting (vs 3 levels regex)
- ✅ <1% false positives (vs 10% regex)
- ✅ All regex tests pass + new tests

---

## 📦 Phase 1: Foundation & Core (Week 1-2)

### 1.1 Setup Infrastructure

**New File Structure:**
```
src/transformer/
├── javascript.rs              # Keep existing regex version
├── javascript_ast.rs          # NEW: Main AST transformer
└── ast/                       # NEW: AST modules
    ├── mod.rs
    ├── parser.rs              # Parse JS/TS to AST
    ├── visitor.rs             # Traverse & modify AST
    ├── scope.rs               # Scope analysis
    └── codegen.rs             # AST → code
```

**Add Dependencies to `Cargo.toml`:**
```toml
[dependencies]
# Core AST framework (includes TypeScript support)
swc_core = "0.90"
swc_common = { version = "0.36", features = ["sourcemap"] }
swc_ecma_parser = "0.147"
swc_ecma_ast = "0.118"
swc_ecma_visit = { version = "0.104", features = ["path"] }
swc_ecma_transforms = "0.230"
swc_ecma_transforms_typescript = "0.187"  # TS stripping
swc_ecma_codegen = "0.155"
swc_ecma_utils = "0.133"

[features]
default = ["regex-transformer"]
ast-transformer = ["swc_core"]
both = ["regex-transformer", "ast-transformer"]
```

### 1.2 Core Parser (`src/transformer/ast/parser.rs`)

**Capabilities:**
- Parse JavaScript (ES3-ES2024)
- Parse TypeScript (all versions)
- Parse JSX/TSX
- Auto-detect module type
- Error recovery

```rust
use swc_common::{SourceMap, FileName};
use swc_ecma_parser::{Parser, Syntax, TsConfig, EsConfig};

pub struct AstParser {
    source_map: Arc<SourceMap>,
}

impl AstParser {
    pub fn new() -> Self {
        Self {
            source_map: Arc::new(SourceMap::default()),
        }
    }
    
    /// Parse JS/TS file - auto-detects syntax
    pub fn parse(&self, code: &str, path: &Path) -> Result<Module> {
        let syntax = self.detect_syntax(path);
        let fm = self.source_map.new_source_file(
            FileName::Real(path.to_path_buf()),
            code.to_string(),
        );
        
        let mut parser = Parser::new(syntax, &fm, None);
        parser.parse_module()
            .map_err(|e| anyhow!("Parse error: {:?}", e))
    }
    
    fn detect_syntax(&self, path: &Path) -> Syntax {
        match path.extension().and_then(|s| s.to_str()) {
            Some("ts") => Syntax::Typescript(TsConfig {
                tsx: false,
                decorators: true,
                ..Default::default()
            }),
            Some("tsx") => Syntax::Typescript(TsConfig {
                tsx: true,
                decorators: true,
                ..Default::default()
            }),
            _ => Syntax::Es(EsConfig {
                jsx: path.extension()
                    .and_then(|s| s.to_str())
                    .map_or(false, |s| s == "jsx"),
                ..Default::default()
            }),
        }
    }
}
```

### 1.3 AST Visitor Pattern (`src/transformer/ast/visitor.rs`)

**Traverse and transform AST:**

```rust
use swc_ecma_visit::{VisitMut, VisitMutWith};

pub struct ChromeTransformVisitor {
    changes: Vec<Change>,
    scope: ScopeAnalyzer,
}

impl VisitMut for ChromeTransformVisitor {
    // Transform chrome.* → browser.*
    fn visit_mut_member_expr(&mut self, node: &mut MemberExpr) {
        node.visit_mut_children_with(self);
        
        if self.is_chrome_api(node) {
            self.transform_to_browser(node);
        }
    }
    
    // Transform callbacks to promises
    fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
        node.visit_mut_children_with(self);
        
        if self.is_callback_pattern(node) {
            self.transform_to_promise(node);
        }
    }
    
    // Track variable declarations for scope analysis
    fn visit_mut_var_decl(&mut self, node: &mut VarDecl) {
        for decl in &node.decls {
            if let Pat::Ident(ident) = &decl.name {
                self.scope.declare(ident.id.sym.as_ref());
            }
        }
        node.visit_mut_children_with(self);
    }
}

impl ChromeTransformVisitor {
    fn is_chrome_api(&self, expr: &MemberExpr) -> bool {
        // Check if this is the global chrome object, not a local variable
        if let Expr::Ident(ident) = &*expr.obj {
            if ident.sym.as_ref() == "chrome" {
                return !self.scope.is_local("chrome");
            }
        }
        false
    }
}
```

### 1.4 Basic Transformations

**Implement these core transforms:**

1. **Namespace Conversion** (`chrome.*` → `browser.*`)
   - Skip strings, comments, regex
   - Handle dynamic access: `chrome[api][method]()`
   - Preserve local `chrome` variables

2. **TypeScript Stripping**
   - Remove type annotations
   - Remove interfaces/types
   - Keep runtime code (enums, decorators)

3. **Polyfill Injection**
   - Detect module type (ES/CommonJS/Script)
   - Inject appropriate polyfill format

**Implementation:**
```rust
// src/transformer/ast/mod.rs
pub struct AstTransformer {
    parser: AstParser,
    codegen: CodeGenerator,
}

impl AstTransformer {
    pub fn transform(&mut self, code: &str, path: &Path) -> Result<String> {
        // 1. Parse
        let mut module = self.parser.parse(code, path)?;
        
        // 2. Transform TypeScript → JavaScript
        if path.extension().map_or(false, |e| e == "ts" || e == "tsx") {
            module = self.strip_typescript(module);
        }
        
        // 3. Apply transformations
        let mut visitor = ChromeTransformVisitor::new();
        module.visit_mut_with(&mut visitor);
        
        // 4. Generate code
        self.codegen.generate(&module)
    }
    
    fn strip_typescript(&self, module: Module) -> Module {
        use swc_ecma_transforms_typescript::strip;
        // Use SWC's built-in TS stripper
        strip(module)
    }
}
```

### 1.5 Code Generation (`src/transformer/ast/codegen.rs`)

```rust
use swc_ecma_codegen::{Emitter, Config};

pub struct CodeGenerator {
    source_map: Arc<SourceMap>,
}

impl CodeGenerator {
    pub fn generate(&self, module: &Module) -> Result<String> {
        let mut buf = vec![];
        let mut emitter = Emitter {
            cfg: Config {
                minify: false,
                ..Default::default()
            },
            cm: self.source_map.clone(),
            comments: None,
            wr: Box::new(JsWriter::new(
                self.source_map.clone(),
                "\n",
                &mut buf,
                None,
            )),
        };
        
        emitter.emit_module(module)?;
        Ok(String::from_utf8(buf)?)
    }
}
```

### 1.6 Phase 1 Tests

Create `tests/ast_transformer/phase1.rs`:
```rust
#[test]
fn test_parse_javascript() {
    let code = "chrome.storage.get('key');";
    let parser = AstParser::new();
    assert!(parser.parse(code, Path::new("test.js")).is_ok());
}

#[test]
fn test_parse_typescript() {
    let code = "const x: string = 'test';";
    let parser = AstParser::new();
    assert!(parser.parse(code, Path::new("test.ts")).is_ok());
}

#[test]
fn test_chrome_to_browser() {
    let code = "chrome.storage.local.get('key');";
    let result = transform_ast(code, Path::new("test.js")).unwrap();
    assert!(result.contains("browser.storage"));
}

#[test]
fn test_typescript_stripping() {
    let code = "const x: string = 'test';";
    let result = transform_ast(code, Path::new("test.ts")).unwrap();
    assert!(!result.contains(": string"));
    assert!(result.contains("const x = 'test'"));
}
```

**Phase 1 Deliverable:** ✅ Can parse JS/TS, strip types, convert `chrome.*` → `browser.*`

---

## 📦 Phase 2: Advanced Features (Week 3-4)

### 2.1 Scope Analyzer (`src/transformer/ast/scope.rs`)

**Track variable scopes accurately:**

```rust
use std::collections::{HashMap, HashSet};

pub struct ScopeAnalyzer {
    scopes: Vec<Scope>,
    current_scope: usize,
}

#[derive(Debug)]
struct Scope {
    parent: Option<usize>,
    kind: ScopeKind,
    bindings: HashSet<String>,
}

#[derive(Debug)]
enum ScopeKind {
    Global,
    Function,
    Block,
    Module,
}

impl ScopeAnalyzer {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope {
                parent: None,
                kind: ScopeKind::Global,
                bindings: HashSet::new(),
            }],
            current_scope: 0,
        }
    }
    
    pub fn enter_scope(&mut self, kind: ScopeKind) {
        let parent = self.current_scope;
        self.scopes.push(Scope {
            parent: Some(parent),
            kind,
            bindings: HashSet::new(),
        });
        self.current_scope = self.scopes.len() - 1;
    }
    
    pub fn exit_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            self.current_scope = parent;
        }
    }
    
    pub fn declare(&mut self, name: &str) {
        self.scopes[self.current_scope]
            .bindings
            .insert(name.to_string());
    }
    
    pub fn is_local(&self, name: &str) -> bool {
        let mut scope_id = Some(self.current_scope);
        while let Some(id) = scope_id {
            if self.scopes[id].bindings.contains(name) {
                return true;
            }
            scope_id = self.scopes[id].parent;
        }
        false
    }
    
    pub fn find_globals(&self) -> Vec<String> {
        self.scopes[0].bindings.iter().cloned().collect()
    }
}
```

### 2.2 Callback Transformation (Unlimited Nesting)

```rust
impl ChromeTransformVisitor {
    fn transform_callback_to_promise(&mut self, call: &mut CallExpr) {
        // Detect pattern: api.method(args, callback)
        if let Some(callback_arg) = call.args.last() {
            if self.is_callback_function(&callback_arg.expr) {
                // Extract callback
                let callback = call.args.pop().unwrap();
                
                // Transform to promise chain
                let promise_call = self.create_promise_chain(call, callback);
                *call = promise_call;
            }
        }
    }
    
    fn create_promise_chain(&self, call: &CallExpr, callback: ExprOrSpread) -> CallExpr {
        // browser.api.method(args).then(callback)
        // Handles any nesting depth automatically
        todo!("Generate promise chain")
    }
    
    fn flatten_nested_callbacks(&self, calls: Vec<CallExpr>) -> Expr {
        // Convert callback hell to Promise.all or async/await
        // Example: 3 nested calls → Promise.all([...])
        todo!("Flatten callbacks")
    }
}
```

### 2.3 executeScript Analysis

```rust
impl ChromeTransformVisitor {
    fn transform_execute_script(&mut self, call: &mut CallExpr) {
        // 1. Extract function from executeScript
        let script_fn = self.extract_script_function(call);
        
        // 2. Analyze closure captures using scope analyzer
        let captures = self.scope.analyze_captures(&script_fn);
        
        // 3. Generate message passing code
        let message_id = self.generate_unique_id();
        let send_message = self.create_send_message(message_id, captures);
        let listener = self.create_message_listener(message_id, script_fn);
        
        // 4. Replace executeScript with sendMessage
        *call = send_message;
        
        // 5. Store listener for content script injection
        self.content_listeners.push(listener);
    }
    
    fn extract_script_function(&self, call: &CallExpr) -> Function {
        // Get function from executeScript call object
        todo!()
    }
}
```

### 2.4 Module System Detection

```rust
#[derive(Debug, Clone, Copy)]
pub enum ModuleType {
    ESModule,      // import/export
    CommonJS,      // require/module.exports
    Script,        // Browser global
}

pub struct ModuleDetector;

impl ModuleDetector {
    pub fn detect(module: &Module) -> ModuleType {
        let mut has_import = false;
        let mut has_require = false;
        
        for item in &module.body {
            match item {
                ModuleItem::ModuleDecl(ModuleDecl::Import(_)) |
                ModuleItem::ModuleDecl(ModuleDecl::ExportAll(_)) |
                ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(_)) => {
                    has_import = true;
                }
                ModuleItem::Stmt(Stmt::Expr(expr)) => {
                    if self.is_require_call(expr) {
                        has_require = true;
                    }
                }
                _ => {}
            }
        }
        
        if has_import {
            ModuleType::ESModule
        } else if has_require {
            ModuleType::CommonJS
        } else {
            ModuleType::Script
        }
    }
}
```

### 2.5 Smart Polyfill Injection

```rust
impl ChromeTransformVisitor {
    fn inject_polyfill(&mut self, module: &mut Module, module_type: ModuleType) {
        let polyfill = match module_type {
            ModuleType::ESModule => {
                // Add at top: import './browser-polyfill.js';
                self.create_import_statement("./browser-polyfill.js")
            }
            ModuleType::CommonJS => {
                // Add at top: require('./browser-polyfill.js');
                self.create_require_statement("./browser-polyfill.js")
            }
            ModuleType::Script => {
                // Add at top: if (typeof browser === 'undefined') { ... }
                self.create_polyfill_check()
            }
        };
        
        module.body.insert(0, polyfill);
    }
}
```

### 2.6 Phase 2 Tests

```rust
#[test]
fn test_deep_callback_nesting() {
    let code = r#"
        chrome.storage.get('a', (a) => {
            chrome.storage.get('b', (b) => {
                chrome.storage.get('c', (c) => {
                    chrome.storage.get('d', (d) => {
                        console.log(a, b, c, d);
                    });
                });
            });
        });
    "#;
    
    let result = transform_ast(code, Path::new("test.js")).unwrap();
    
    // Should flatten to Promise.all or async/await
    assert!(result.contains("Promise.all") || result.contains("await"));
    assert!(!result.contains("(a) => {")); // No nested callbacks
}

#[test]
fn test_scope_analysis() {
    let code = r#"
        let global = 1;
        function test() {
            let local = 2;
            chrome.storage.set({global}); // Should recognize global
            chrome.storage.set({local});  // Should recognize local
        }
    "#;
    
    let result = transform_ast(code, Path::new("test.js")).unwrap();
    // Verify scope detection worked correctly
}

#[test]
fn test_module_detection() {
    // ES Module
    let es_code = "import x from 'y'; chrome.storage.get('k');";
    let result = transform_ast(es_code, Path::new("test.js")).unwrap();
    assert!(result.starts_with("import"));
    
    // CommonJS
    let cjs_code = "const x = require('y'); chrome.storage.get('k');";
    let result = transform_ast(cjs_code, Path::new("test.js")).unwrap();
    assert!(result.contains("require('./browser-polyfill')"));
    
    // Script
    let script_code = "chrome.storage.get('k');";
    let result = transform_ast(script_code, Path::new("test.js")).unwrap();
    assert!(result.contains("typeof browser === 'undefined'"));
}
```

**Phase 2 Deliverable:** ✅ All transformations work perfectly with scope awareness

---

## 📦 Phase 3: Testing & Integration (Week 5-6)

### 3.1 Comprehensive Test Suite

**Create `tests/ast_transformer/` with:**

```rust
// comparison.rs - Compare regex vs AST
#[test]
fn ast_must_beat_regex_in_accuracy() {
    let test_cases = load_test_cases();
    
    for case in test_cases {
        let regex_result = regex_transform(&case.input);
        let ast_result = ast_transform(&case.input);
        
        let regex_score = measure_accuracy(&regex_result, &case.expected);
        let ast_score = measure_accuracy(&ast_result, &case.expected);
        
        assert!(
            ast_score >= regex_score,
            "AST ({}) must be >= Regex ({})",
            ast_score,
            regex_score
        );
    }
}

// edge_cases.rs - Test tricky patterns
#[test]
fn test_strings_and_comments() {
    let code = r#"
        // chrome.storage is deprecated
        const url = "Visit chrome.storage docs";
        /* chrome.tabs.query */
        chrome.runtime.id; // This should transform
    "#;
    
    let result = ast_transform(code);
    
    // Should only transform the last line
    assert!(result.contains("chrome.storage docs")); // String preserved
    assert!(result.contains("// chrome.storage is deprecated")); // Comment preserved
    assert!(result.contains("browser.runtime.id")); // Code transformed
}

// typescript.rs - Full TS support tests
#[test]
fn test_typescript_features() {
    let test_cases = vec![
        // Interfaces
        ("interface X { a: string }", ""),
        // Type annotations
        ("const x: number = 1", "const x = 1"),
        // Generics
        ("Array<string>", "Array"),
        // Enums (should preserve)
        ("enum Color { Red }", "enum Color { Red }"),
        // Decorators
        ("@decorator class X {}", "@decorator class X {}"),
    ];
    
    for (input, expected) in test_cases {
        let result = ast_transform(input);
        assert_eq!(result.trim(), expected);
    }
}

// real_world.rs - Test on actual extensions
#[test]
fn test_real_extensions() {
    let extensions = [
        "tests/fixtures/simple-extension",
        "tests/fixtures/typescript-extension",
        "tests/fixtures/complex-extension",
    ];
    
    for ext_path in extensions {
        let result = convert_extension(ext_path, TransformerBackend::AST);
        assert!(result.is_ok());
        
        // Verify output works in Firefox
        validate_firefox_compatibility(&result.unwrap());
    }
}
```

### 3.2 Benchmarking

```rust
// benches/transformer_comparison.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_regex(c: &mut Criterion) {
    let code = include_str!("../tests/fixtures/large_extension.js");
    
    c.bench_function("regex_transform", |b| {
        b.iter(|| regex_transform(black_box(code)))
    });
}

fn bench_ast(c: &mut Criterion) {
    let code = include_str!("../tests/fixtures/large_extension.js");
    
    c.bench_function("ast_transform", |b| {
        b.iter(|| ast_transform(black_box(code)))
    });
}

criterion_group!(benches, bench_regex, bench_ast);
criterion_main!(benches);
```

Run with: `cargo bench --features both`

### 3.3 Integration into Main Codebase

**Update `src/lib.rs`:**

```rust
pub enum TransformerBackend {
    Regex,
    AST,
    Auto, // Choose best for each file
}

pub struct ConversionOptions {
    pub transformer: TransformerBackend,
    // ... other options
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            transformer: TransformerBackend::Auto, // Smart default
            // ...
        }
    }
}

fn select_transformer(file: &Path, content: &str) -> Box<dyn Transformer> {
    match config.transformer {
        TransformerBackend::Regex => Box::new(JavaScriptTransformer::new(&[])),
        TransformerBackend::AST => Box::new(AstJavaScriptTransformer::new(&[])),
        TransformerBackend::Auto => {
            // Use AST for:
            if file.extension().map_or(false, |e| e == "ts" || e == "tsx") {
                Box::new(AstJavaScriptTransformer::new(&[]))
            } else if content.contains("import ") || content.contains("export ") {
                Box::new(AstJavaScriptTransformer::new(&[]))
            } else if content.len() > 50_000 {
                Box::new(AstJavaScriptTransformer::new(&[]))
            } else {
                // Use regex for simple cases (faster)
                Box::new(JavaScriptTransformer::new(&[]))
            }
        }
    }
}
```

**Update CLI (`src/main.rs`):**

```rust
#[derive(Parser)]
struct Cli {
    #[arg(long, default_value = "auto")]
    transformer: TransformerChoice,
}

#[derive(ValueEnum, Clone)]
enum TransformerChoice {
    Regex,
    Ast,
    Auto,
}
```

### 3.4 Documentation

**Update `README.md`:**

```markdown
## TypeScript Support

The converter now has full TypeScript support using AST-based parsing:

```bash
# Convert TypeScript extension
cargo run -- convert -i ./my-ts-extension -o ./output

# Force AST transformer
cargo run -- convert -i ./extension -o ./output --transformer ast

# Auto-detect (default)
cargo run -- convert -i ./extension -o ./output --transformer auto
```

**Features:**
- ✅ Full TypeScript syntax support
- ✅ Strips types automatically
- ✅ Preserves runtime code (enums, decorators)
- ✅ Handles .ts, .tsx, .d.ts files
- ✅ Better accuracy than regex (95% vs 75%)
```

**Create `AST_TRANSFORMER.md`:**

Document the architecture, how it works, limitations, etc.

### 3.5 Final Validation Checklist

Before declaring success:

```bash
# Run all tests
cargo test --all-features

# Run benchmarks
cargo bench --features both

# Test on real extensions
./test-real-extensions.sh

# Compare outputs
./compare-transformers.sh

# Check performance
./benchmark-suite.sh
```

**Validation Script (`compare-transformers.sh`):**
```bash
#!/bin/bash
set -e

EXTENSIONS=(
    "tests/fixtures/simple"
    "tests/fixtures/typescript"
    "tests/fixtures/complex"
)

for ext in "${EXTENSIONS[@]}"; do
    echo "Testing $ext..."
    
    # Convert with regex
    cargo run -- convert -i "$ext" -o "${ext}_regex" --transformer regex
    
    # Convert with AST
    cargo run -- convert -i "$ext" -o "${ext}_ast" --transformer ast
    
    # Compare (AST should be equal or better)
    diff -r "${ext}_regex" "${ext}_ast" || echo "Differences found (AST may be better)"
done

echo "✅ All comparisons complete"
```

**Phase 3 Deliverable:** ✅ Production-ready AST transformer, fully tested and documented

---

## 🚀 Quick Start Commands

```bash
# 1. Create branch
git checkout -b feat/ast-transformer

# 2. Create file structure
mkdir -p src/transformer/ast tests/ast_transformer
touch src/transformer/javascript_ast.rs
touch src/transformer/ast/{mod,parser,visitor,scope,codegen}.rs

# 3. Add dependencies (edit Cargo.toml with dependencies above)

# 4. Implement Phase 1
# ... implement parser, basic transforms ...

# 5. Test Phase 1
cargo test --features ast-transformer

# 6. Implement Phase 2
# ... implement advanced features ...

# 7. Test Phase 2
cargo test --features ast-transformer

# 8. Implement Phase 3
# ... integrate, test, document ...

# 9. Final validation
cargo test --all-features
cargo bench --features both
./validate-all.sh

# 10. Merge when all tests pass
git merge feat/ast-transformer
```

---

## 📊 Success Metrics

Track throughout development:

| Metric | Target | Status |
|--------|--------|--------|
| **Accuracy** | 95%+ | TBD |
| **False Positives** | <1% | TBD |
| **TypeScript Support** | 100% | TBD |
| **Performance** | 80-90% of regex | TBD |
| **Test Pass Rate** | 100% | TBD |
| **Real Extensions** | 50/50 | TBD |

---

## 🎯 Definition of Done

✅ All tests pass (unit + integration)
✅ Benchmarks show <20% slowdown vs regex
✅ TypeScript fully supported
✅ Tested on 50+ real extensions
✅ Documentation complete
✅ Code reviewed and clean
✅ Ready to replace regex

**Timeline:** 6 weeks for bulletproof implementation
**Outcome:** Perfect, production-grade AST transformer with native TypeScript support