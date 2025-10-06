# Implementation Guide: Chrome-to-Firefox Extension Converter

This document provides detailed implementation specifications for building the converter in Rust.

## Project Setup

### Cargo.toml Configuration

```toml
[package]
name = "chrome-to-firefox"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <email@example.com>"]
description = "Convert Chrome MV3 extensions to Firefox-compatible MV3"
license = "MIT"
repository = "https://github.com/yourusername/chrome-to-firefox"

[[bin]]
name = "chrome-to-firefox"
path = "src/main.rs"

[lib]
name = "chrome_to_firefox"
path = "src/lib.rs"
crate-type = ["cdylib", "rlib"]  # cdylib for WASM later

[dependencies]
# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# File handling
zip = "0.6"
walkdir = "2.4"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Pattern matching
regex = "1.10"
lazy_static = "1.4"

# CLI (for main.rs)
clap = { version = "4.4", features = ["derive"] }
dialoguer = "0.11"
colored = "2.1"
indicatif = "0.17"

# JavaScript parsing
swc_ecma_parser = "0.147"
swc_common = "0.34"
swc_ecma_ast = "0.113"
swc_ecma_visit = "0.113"

# String manipulation
inflector = "0.11"

[dev-dependencies]
tempfile = "3.8"
pretty_assertions = "1.4"
test-case = "3.3"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

### Directory Structure

```
chrome-to-firefox/
├── Cargo.toml
├── README.md
├── ARCHITECTURE.md
├── API_MAPPINGS.md
├── IMPLEMENTATION_GUIDE.md
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library root
│   ├── models/
│   │   ├── mod.rs
│   │   ├── manifest.rs      # Manifest data structures
│   │   ├── extension.rs     # Extension representation
│   │   ├── conversion.rs    # Conversion context
│   │   └── incompatibility.rs
│   ├── parser/
│   │   ├── mod.rs
│   │   ├── manifest.rs      # Parse manifest.json
│   │   └── javascript.rs    # Parse JavaScript files
│   ├── analyzer/
│   │   ├── mod.rs
│   │   ├── manifest.rs      # Analyze manifest
│   │   ├── api.rs           # Detect Chrome APIs
│   │   └── patterns.rs      # Known patterns
│   ├── transformer/
│   │   ├── mod.rs
│   │   ├── manifest.rs      # Transform manifest
│   │   ├── javascript.rs    # Transform JS code
│   │   └── shims.rs         # Generate shims
│   ├── decision/
│   │   ├── mod.rs
│   │   └── tree.rs          # Decision logic
│   ├── packager/
│   │   ├── mod.rs
│   │   ├── extractor.rs     # Extract ZIP/CRX
│   │   └── builder.rs       # Build XPI/ZIP
│   ├── validator/
│   │   ├── mod.rs
│   │   └── structure.rs     # Validate result
│   ├── report/
│   │   ├── mod.rs
│   │   └── generator.rs     # Generate report
│   └── utils/
│       ├── mod.rs
│       └── helpers.rs       # Utility functions
├── tests/
│   ├── integration_tests.rs
│   ├── manifest_tests.rs
│   ├── javascript_tests.rs
│   └── fixtures/
│       ├── simple_extension/
│       └── LatexToCalc/
├── shims/                   # Template shim files
│   ├── browser-polyfill.js
│   ├── promise-wrapper.js
│   └── action-compat.js
└── docs/
    └── CONVERSION_RULES.md
```

## Implementation Phase 1: Core Data Models

### models/manifest.rs

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub manifest_version: u8,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Background>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_action: Option<Action>, // MV2 legacy
    
    #[serde(default)]
    pub permissions: Vec<String>,
    
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub host_permissions: Vec<String>,
    
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_scripts: Vec<ContentScript>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_accessible_resources: Option<WebAccessibleResources>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_security_policy: Option<ContentSecurityPolicy>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_specific_settings: Option<BrowserSpecificSettings>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<HashMap<String, String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands: Option<HashMap<String, Command>>,
    
    // Catch-all for unknown fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Background {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_worker: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scripts: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_popup: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_icon: Option<IconSet>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_title: Option<String>,
    
    // Deprecated in MV3
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_style: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IconSet {
    Single(String),
    Multiple(HashMap<String, String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentScript {
    pub matches: Vec<String>,
    
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub js: Vec<String>,
    
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub css: Vec<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_at: Option<String>,
    
    #[serde(default)]
    pub all_frames: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebAccessibleResources {
    V2(Vec<String>),
    V3(Vec<WebAccessibleResourceV3>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAccessibleResourceV3 {
    pub resources: Vec<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matches: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension_ids: Option<Vec<String>>,
    
    // Not supported in Firefox
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_dynamic_url: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentSecurityPolicy {
    V2(String),
    V3(ContentSecurityPolicyV3),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSecurityPolicyV3 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension_pages: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSpecificSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gecko: Option<GeckoSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoSettings {
    pub id: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_min_version: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_max_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_key: Option<HashMap<String, String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
```

### models/extension.rs

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use super::manifest::Manifest;

pub struct Extension {
    pub manifest: Manifest,
    pub files: HashMap<PathBuf, Vec<u8>>,
    pub metadata: ExtensionMetadata,
}

pub struct ExtensionMetadata {
    pub name: String,
    pub version: String,
    pub manifest_version: u8,
    pub size_bytes: usize,
    pub file_count: usize,
    pub has_background: bool,
    pub has_content_scripts: bool,
    pub has_web_accessible_resources: bool,
}

impl Extension {
    pub fn new(manifest: Manifest, files: HashMap<PathBuf, Vec<u8>>) -> Self {
        let size_bytes = files.values().map(|v| v.len()).sum();
        let file_count = files.len();
        
        let metadata = ExtensionMetadata {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            manifest_version: manifest.manifest_version,
            size_bytes,
            file_count,
            has_background: manifest.background.is_some(),
            has_content_scripts: !manifest.content_scripts.is_empty(),
            has_web_accessible_resources: manifest.web_accessible_resources.is_some(),
        };
        
        Self {
            manifest,
            files,
            metadata,
        }
    }
    
    pub fn get_javascript_files(&self) -> Vec<PathBuf> {
        self.files
            .keys()
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == "js")
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }
}
```

### models/incompatibility.rs

```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Incompatibility {
    pub severity: Severity,
    pub category: IncompatibilityCategory,
    pub location: Location,
    pub description: String,
    pub suggestion: Option<String>,
    pub auto_fixable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Minor,
    Major,
    Blocker,
}

#[derive(Debug, Clone)]
pub enum IncompatibilityCategory {
    ManifestStructure,
    BackgroundWorker,
    ChromeOnlyApi,
    ApiNamespace,
    CallbackVsPromise,
    HostPermissions,
    WebRequest,
    WebAccessibleResources,
    ContentSecurityPolicy,
    MissingFirefoxId,
    BrowserStyle,
    VersionFormat,
}

#[derive(Debug, Clone)]
pub enum Location {
    Manifest,
    ManifestField(String),
    File(PathBuf),
    FileLocation(PathBuf, usize), // file, line number
}

impl Incompatibility {
    pub fn new(
        severity: Severity,
        category: IncompatibilityCategory,
        location: Location,
        description: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            category,
            location,
            description: description.into(),
            suggestion: None,
            auto_fixable: false,
        }
    }
    
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
    
    pub fn auto_fixable(mut self) -> Self {
        self.auto_fixable = true;
        self
    }
}
```

### models/conversion.rs

```rust
use super::{Extension, Incompatibility};

pub struct ConversionContext {
    pub source: Extension,
    pub incompatibilities: Vec<Incompatibility>,
    pub warnings: Vec<Warning>,
    pub decisions: Vec<UserDecision>,
    pub selected_decisions: Vec<SelectedDecision>,
}

#[derive(Debug, Clone)]
pub struct Warning {
    pub message: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserDecision {
    pub id: String,
    pub category: DecisionCategory,
    pub question: String,
    pub context: String,
    pub options: Vec<DecisionOption>,
    pub default_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionCategory {
    BackgroundArchitecture,
    ApiStrategy,
    HostPermissions,
    WebRequest,
    Offscreen,
    ExtensionId,
    Other,
}

#[derive(Debug, Clone)]
pub struct DecisionOption {
    pub label: String,
    pub description: String,
    pub recommended: bool,
}

#[derive(Debug, Clone)]
pub struct SelectedDecision {
    pub decision_id: String,
    pub selected_index: usize,
}

pub struct ConversionResult {
    pub manifest: String,
    pub modified_files: Vec<ModifiedFile>,
    pub new_files: Vec<NewFile>,
    pub report: ConversionReport,
}

pub struct ModifiedFile {
    pub path: String,
    pub original_content: Vec<u8>,
    pub new_content: Vec<u8>,
    pub changes: Vec<FileChange>,
}

pub struct NewFile {
    pub path: String,
    pub content: Vec<u8>,
    pub purpose: String,
}

pub struct FileChange {
    pub line_number: usize,
    pub change_type: ChangeType,
    pub description: String,
}

pub enum ChangeType {
    Addition,
    Modification,
    Deletion,
}

pub struct ConversionReport {
    pub summary: ReportSummary,
    pub manifest_changes: Vec<String>,
    pub javascript_changes: Vec<String>,
    pub blockers: Vec<String>,
    pub manual_actions: Vec<String>,
    pub warnings: Vec<String>,
}

pub struct ReportSummary {
    pub extension_name: String,
    pub extension_version: String,
    pub conversion_successful: bool,
    pub files_modified: usize,
    pub files_added: usize,
    pub total_changes: usize,
}
```

## Implementation Phase 2: Parser Module

### parser/manifest.rs

```rust
use crate::models::manifest::Manifest;
use anyhow::{Context, Result};
use std::path::Path;

pub fn parse_manifest(content: &[u8]) -> Result<Manifest> {
    let manifest: Manifest = serde_json::from_slice(content)
        .context("Failed to parse manifest.json")?;
    
    // Basic validation
    if manifest.manifest_version != 2 && manifest.manifest_version != 3 {
        anyhow::bail!("Unsupported manifest version: {}", manifest.manifest_version);
    }
    
    Ok(manifest)
}

pub fn parse_manifest_from_file(path: impl AsRef<Path>) -> Result<Manifest> {
    let content = std::fs::read(path.as_ref())
        .context("Failed to read manifest file")?;
    parse_manifest(&content)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_manifest() {
        let json = r#"{
            "manifest_version": 3,
            "name": "Test Extension",
            "version": "1.0.0"
        }"#;
        
        let manifest = parse_manifest(json.as_bytes()).unwrap();
        assert_eq!(manifest.manifest_version, 3);
        assert_eq!(manifest.name, "Test Extension");
        assert_eq!(manifest.version, "1.0.0");
    }
}
```

### parser/javascript.rs

```rust
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_common::SourceMap;
use swc_ecma_ast::*;
use swc_ecma_visit::{Visit, VisitWith};
use std::sync::Arc;

pub struct ChromeApiVisitor {
    pub chrome_calls: Vec<ChromeApiCall>,
}

#[derive(Debug, Clone)]
pub struct ChromeApiCall {
    pub line: usize,
    pub column: usize,
    pub api_name: String,
    pub is_callback_style: bool,
}

impl Visit for ChromeApiVisitor {
    fn visit_call_expr(&mut self, call: &CallExpr) {
        // Detect chrome.* API calls
        if let Callee::Expr(expr) = &call.callee {
            if let Expr::Member(member) = &**expr {
                if let Expr::Ident(obj) = &*member.obj {
                    if obj.sym.as_ref() == "chrome" {
                        // Found chrome API call
                        let api_name = format_member_expr(member);
                        let is_callback = has_callback_param(&call.args);
                        
                        self.chrome_calls.push(ChromeApiCall {
                            line: 0, // TODO: get from span
                            column: 0,
                            api_name,
                            is_callback_style: is_callback,
                        });
                    }
                }
            }
        }
        
        call.visit_children_with(self);
    }
}

fn format_member_expr(member: &MemberExpr) -> String {
    // Recursively build API name like "chrome.storage.local.get"
    let mut parts = vec![];
    
    fn collect_parts(expr: &Expr, parts: &mut Vec<String>) {
        match expr {
            Expr::Ident(ident) => parts.push(ident.sym.to_string()),
            Expr::Member(member) => {
                collect_parts(&member.obj, parts);
                if let MemberProp::Ident(prop) = &member.prop {
                    parts.push(prop.sym.to_string());
                }
            }
            _ => {}
        }
    }
    
    collect_parts(&Expr::Member(member.clone()), &mut parts);
    parts.join(".")
}

fn has_callback_param(args: &[ExprOrSpread]) -> bool {
    args.last()
        .and_then(|arg| {
            if let Expr::Arrow(_) | Expr::Fn(_) = &*arg.expr {
                Some(true)
            } else {
                None
            }
        })
        .unwrap_or(false)
}

pub fn analyze_javascript(source: &str) -> Vec<ChromeApiCall> {
    let cm: Arc<SourceMap> = Default::default();
    let fm = cm.new_source_file(
        swc_common::FileName::Anon,
        source.to_string(),
    );
    
    let lexer = Lexer::new(
        Syntax::default(),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    
    let mut parser = Parser::new_from(lexer);
    
    match parser.parse_module() {
        Ok(module) => {
            let mut visitor = ChromeApiVisitor {
                chrome_calls: vec![],
            };
            module.visit_with(&mut visitor);
            visitor.chrome_calls
        }
        Err(_) => vec![],
    }
}
```

## Implementation Phase 3: Analyzer Module

### analyzer/manifest.rs

```rust
use crate::models::{
    manifest::*,
    incompatibility::*,
};

pub fn analyze_manifest(manifest: &Manifest) -> Vec<Incompatibility> {
    let mut issues = Vec::new();
    
    // Check manifest version
    if manifest.manifest_version != 3 {
        issues.push(
            Incompatibility::new(
                Severity::Blocker,
                IncompatibilityCategory::ManifestStructure,
                Location::ManifestField("manifest_version".to_string()),
                format!("Only Manifest V3 is supported. Found version {}", manifest.manifest_version)
            )
        );
        return issues; // Early return for unsupported version
    }
    
    // Check for browser_specific_settings
    if manifest.browser_specific_settings.is_none() {
        issues.push(
            Incompatibility::new(
                Severity::Major,
                IncompatibilityCategory::MissingFirefoxId,
                Location::ManifestField("browser_specific_settings".to_string()),
                "Firefox requires browser_specific_settings.gecko.id for submission"
            )
            .with_suggestion("Add a unique extension ID in email format")
            .auto_fixable()
        );
    }
    
    // Check background configuration
    if let Some(background) = &manifest.background {
        if background.service_worker.is_some() && background.scripts.is_none() {
            issues.push(
                Incompatibility::new(
                    Severity::Major,
                    IncompatibilityCategory::BackgroundWorker,
                    Location::ManifestField("background".to_string()),
                    "Service worker detected. Firefox MV3 uses event pages (background.scripts)"
                )
                .with_suggestion("Add background.scripts with same file for Firefox compatibility")
                .auto_fixable()
            );
        }
    }
    
    // Check host_permissions
    let has_host_patterns_in_permissions = manifest.permissions.iter()
        .any(|p| is_match_pattern(p));
    
    if has_host_patterns_in_permissions && manifest.host_permissions.is_empty() {
        issues.push(
            Incompatibility::new(
                Severity::Minor,
                IncompatibilityCategory::HostPermissions,
                Location::ManifestField("permissions".to_string()),
                "Match patterns found in permissions should be in host_permissions for MV3"
            )
            .with_suggestion("Move match patterns from permissions to host_permissions")
            .auto_fixable()
        );
    }
    
    // Check web_accessible_resources
    if let Some(WebAccessibleResources::V3(resources)) = &manifest.web_accessible_resources {
        for resource in resources {
            if resource.use_dynamic_url == Some(true) {
                issues.push(
                    Incompatibility::new(
                        Severity::Minor,
                        IncompatibilityCategory::WebAccessibleResources,
                        Location::ManifestField("web_accessible_resources".to_string()),
                        "use_dynamic_url is not supported in Firefox"
                    )
                    .with_suggestion("Remove use_dynamic_url and ensure matches or extension_ids are specified")
                    .auto_fixable()
                );
            }
        }
    }
    
    // Check CSP format
    if let Some(ContentSecurityPolicy::V2(_)) = &manifest.content_security_policy {
        issues.push(
            Incompatibility::new(
                Severity::Minor,
                IncompatibilityCategory::ContentSecurityPolicy,
                Location::ManifestField("content_security_policy".to_string()),
                "CSP must use object format in MV3"
            )
            .with_suggestion("Convert to { extension_pages: '...' } format")
            .auto_fixable()
        );
    }
    
    // Check for browser_style
    if let Some(action) = &manifest.action {
        if action.browser_style == Some(true) {
            issues.push(
                Incompatibility::new(
                    Severity::Minor,
                    IncompatibilityCategory::BrowserStyle,
                    Location::ManifestField("action.browser_style".to_string()),
                    "browser_style is not supported in MV3"
                )
                .with_suggestion("Remove browser_style property")
                .auto_fixable()
            );
        }
    }
    
    // Check browser_action (MV2 legacy)
    if manifest.browser_action.is_some() {
        issues.push(
            Incompatibility::new(
                Severity::Minor,
                IncompatibilityCategory::ManifestStructure,
                Location::ManifestField("browser_action".to_string()),
                "browser_action should be renamed to action in MV3"
            )
            .with_suggestion("Rename browser_action to action")
            .auto_fixable()
        );
    }
    
    issues
}

fn is_match_pattern(s: &str) -> bool {
    s.contains("://") || s.starts_with('<') || s.starts_with('*')
}
```

### analyzer/api.rs

```rust
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};

lazy_static! {
    // Chrome-only APIs (not available in Firefox)
    static ref CHROME_ONLY_APIS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.insert("chrome.offscreen");
        set.insert("chrome.declarativeContent");
        set.insert("chrome.tabGroups");
        set.insert("chrome.sidePanel");
        set
    };
    
    // APIs that work differently between browsers
    static ref DIFFERENT_APIS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("chrome.proxy", "Different implementation in Firefox");
        map.insert("chrome.tabs.executeScript", "Use chrome.scripting.executeScript in MV3");
        map.insert("chrome.tabs.insertCSS", "Use chrome.scripting.insertCSS in MV3");
        map
    };
}

pub fn is_chrome_only_api(api_name: &str) -> bool {
    CHROME_ONLY_APIS.iter().any(|&chrome_api| api_name.starts_with(chrome_api))
}

pub fn get_api_note(api_name: &str) -> Option<&'static str> {
    DIFFERENT_APIS.iter()
        .find(|&(key, _)| api_name.starts_with(key))
        .map(|(_, note)| *note)
}

pub fn should_convert_to_browser_namespace(api_name: &str) -> bool {
    api_name.starts_with("chrome.") && !is_chrome_only_api(api_name)
}
```

## Implementation Phase 4: Transformer Module

### transformer/manifest.rs

```rust
use crate::models::manifest::*;
use crate::models::conversion::SelectedDecision;

pub struct ManifestTransformer {
    decisions: Vec<SelectedDecision>,
}

impl ManifestTransformer {
    pub fn new(decisions: Vec<SelectedDecision>) -> Self {
        Self { decisions }
    }
    
    pub fn transform(&self, manifest: &Manifest) -> Manifest {
        let mut result = manifest.clone();
        
        // Add Firefox-specific settings
        self.add_firefox_settings(&mut result);
        
        // Transform background
        self.transform_background(&mut result);
        
        // Transform permissions
        self.transform_permissions(&mut result);
        
        // Transform web_accessible_resources
        self.transform_web_accessible_resources(&mut result);
        
        // Transform CSP
        self.transform_csp(&mut result);
        
        // Transform action
        self.transform_action(&mut result);
        
        result
    }
    
    fn add_firefox_settings(&self, manifest: &mut Manifest) {
        if manifest.browser_specific_settings.is_none() {
            // Check if user provided an ID through decisions
            let extension_id = self.get_decision_value("extension_id")
                .unwrap_or_else(|| format!("{}@yourdomain.com", 
                    manifest.name.to_lowercase().replace(' ', "")));
            
            manifest.browser_specific_settings = Some(BrowserSpecificSettings {
                gecko: Some(GeckoSettings {
                    id: extension_id,
                    strict_min_version: Some("121.0".to_string()),
                    strict_max_version: None,
                }),
            });
        }
    }
    
    fn transform_background(&self, manifest: &mut Manifest) {
        if let Some(background) = &mut manifest.background {
            // If has service_worker but no scripts, add scripts for Firefox
            if background.service_worker.is_some() && background.scripts.is_none() {
                background.scripts = background.service_worker.as_ref()
                    .map(|sw| vec![sw.clone()]);
            }
        }
    }
    
    fn transform_permissions(&self, manifest: &mut Manifest) {
        // Move match patterns from permissions to host_permissions
        let (api_perms, host_perms): (Vec<_>, Vec<_>) = manifest.permissions
            .iter()
            .partition(|p| !is_match_pattern(p));
        
        manifest.permissions = api_perms.into_iter().cloned().collect();
        
        let mut all_host_perms = host_perms.into_iter().cloned().collect::<Vec<_>>();
        all_host_perms.extend(manifest.host_permissions.iter().cloned());
        manifest.host_permissions = all_host_perms;
    }
    
    fn transform_web_accessible_resources(&self, manifest: &mut Manifest) {
        if let Some(WebAccessibleResources::V3(resources)) = &mut manifest.web_accessible_resources {
            for resource in resources {
                // Remove use_dynamic_url
                resource.use_dynamic_url = None;
            }
        }
    }
    
    fn transform_csp(&self, manifest: &mut Manifest) {
        // Convert V2 CSP to V3 format
        if let Some(ContentSecurityPolicy::V2(csp_string)) = &manifest.content_security_policy {
            manifest.content_security_policy = Some(ContentSecurityPolicy::V3(
                ContentSecurityPolicyV3 {
                    extension_pages: Some(csp_string.clone()),
                    sandbox: None,
                }
            ));
        }
        
        // Add wasm-unsafe-eval if needed
        // TODO: Check if extension uses WebAssembly
    }
    
    fn transform_action(&self, manifest: &mut Manifest) {
        // Rename browser_action to action
        if manifest.browser_action.is_some() && manifest.action.is_none() {
            manifest.action = manifest.browser_action.clone();
            manifest.browser_action = None;
        }
        
        // Remove browser_style
        if let Some(action) = &mut manifest.action {
            action.browser_style = None;
        }
    }
    
    fn get_decision_value(&self, decision_id: &str) -> Option<String> {
        // TODO: Implement decision value retrieval
        None
    }
}

fn is_match_pattern(s: &str) -> bool {
    s.contains("://") || s.starts_with('<') || s.starts_with('*')
}
```

## CLI Implementation (main.rs)

```rust
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "chrome-to-firefox")]
#[command(about = "Convert Chrome MV3 extensions to Firefox-compatible MV3", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a Chrome extension to Firefox format
    Convert {
        /// Path to the Chrome extension ZIP or directory
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output directory for the converted extension
        #[arg(short, long)]
        output: PathBuf,
        
        /// Skip user prompts and use defaults
        #[arg(short = 'y', long)]
        yes: bool,
        
        /// Generate detailed report
        #[arg(short, long)]
        report: bool,
    },
    
    /// Analyze an extension without converting
    Analyze {
        /// Path to the extension
        #[arg(short, long)]
        input: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Convert { input, output, yes, report } => {
            println!("{}", "Chrome to Firefox Extension Converter".bold().blue());
            println!();
            
            // TODO: Implement conversion flow
        }
        Commands::Analyze { input } => {
            println!("{}", "Analyzing extension...".bold());
            
            // TODO: Implement analysis
        }
    }
}
```

## Testing Strategy

### Integration Test Example

```rust
// tests/integration_tests.rs
use chrome_to_firefox::*;
use tempfile::TempDir;

#[test]
fn test_latex_to_calc_conversion() {
    let input_dir = Path::new("tests/fixtures/LatexToCalc");
    let temp_dir = TempDir::new().unwrap();
    
    let result = convert_extension(
        input_dir,
        temp_dir.path(),
        ConversionOptions::default(),
    ).unwrap();
    
    assert!(result.success);
    assert_eq!(result.blockers.len(), 0);
    
    // Verify manifest was transformed correctly
    let manifest_path = temp_dir.path().join("manifest.json");
    let manifest = parse_manifest_from_file(manifest_path).unwrap();
    
    assert!(manifest.browser_specific_settings.is_some());
    assert!(manifest.background.as_ref().unwrap().scripts.is_some());
}
```

## Next Steps

This implementation guide provides the foundation for building the converter. The key next steps are:

1. Implement the basic Rust project structure
2. Build out the data models completely
3. Implement manifest parsing and analysis
4. Create the transformation engine
5. Add JavaScript analysis capabilities
6. Build the CLI interface
7. Test with real extensions
8. Iterate and refine

The architecture is designed to be modular, allowing for incremental development and easy testing. Each module can be developed and tested independently before integration.