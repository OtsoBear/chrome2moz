//! Models for Chrome-only API conversion

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLocation {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
}

impl FileLocation {
    pub fn new(file: PathBuf, line: usize, column: usize) -> Self {
        Self { file, line, column }
    }
}

impl std::fmt::Display for FileLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file.display(), self.line, self.column)
    }
}

// ============================================================================
// Offscreen API Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffscreenUsage {
    pub call_location: FileLocation,
    pub document_url: String,
    pub reasons: Vec<String>,
    pub justification: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentAnalysis {
    pub primary_purpose: OffscreenPurpose,
    pub secondary_purposes: Vec<OffscreenPurpose>,
    pub complexity_score: u8,  // 0-100
    pub dependencies: Vec<String>,
    pub dom_operations: Vec<DomOperation>,
    pub canvas_operations: Vec<CanvasOperation>,
    pub audio_operations: Vec<AudioOperation>,
    pub network_operations: Vec<NetworkOperation>,
    pub message_handlers: Vec<MessageHandler>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OffscreenPurpose {
    CanvasRendering,        // Canvas 2D/WebGL operations
    AudioProcessing,        // Web Audio API
    ImageProcessing,        // Image manipulation, OCR
    DomParsing,            // HTML parsing, scraping
    NetworkProxying,       // Fetch/XHR operations
    LibraryExecution,      // Running libraries needing DOM
    DataProcessing,        // Heavy computation
    CryptoOperations,      // Crypto libraries
    Mixed(Vec<Box<OffscreenPurpose>>),
    Unknown,
}

impl Default for OffscreenPurpose {
    fn default() -> Self {
        Self::Unknown
    }
}

impl OffscreenPurpose {
    pub fn name(&self) -> &str {
        match self {
            Self::CanvasRendering => "Canvas Rendering",
            Self::AudioProcessing => "Audio Processing",
            Self::ImageProcessing => "Image Processing",
            Self::DomParsing => "DOM Parsing",
            Self::NetworkProxying => "Network Proxying",
            Self::LibraryExecution => "Library Execution",
            Self::DataProcessing => "Data Processing",
            Self::CryptoOperations => "Crypto Operations",
            Self::Mixed(_) => "Mixed Purpose",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomOperation {
    pub operation_type: String,  // querySelector, createElement, etc.
    pub target_url: Option<String>,
    pub selector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasOperation {
    pub operation_type: String,  // getContext, fillRect, etc.
    pub context_type: Option<String>,  // 2d, webgl, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioOperation {
    pub operation_type: String,  // AudioContext, createOscillator, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkOperation {
    pub operation_type: String,  // fetch, XMLHttpRequest, etc.
    pub target_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHandler {
    pub handler_type: String,  // onMessage, addEventListener
    pub message_type: Option<String>,
}

// ============================================================================
// DeclarativeContent API Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclarativeContentRule {
    pub conditions: Vec<PageCondition>,
    pub actions: Vec<PageAction>,
    pub location: FileLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageCondition {
    PageStateMatcher {
        page_url: UrlFilter,
        css: Option<Vec<String>>,
        is_bookmarked: Option<bool>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlFilter {
    pub host_equals: Option<String>,
    pub host_contains: Option<String>,
    pub host_prefix: Option<String>,
    pub host_suffix: Option<String>,
    pub path_equals: Option<String>,
    pub path_contains: Option<String>,
    pub path_prefix: Option<String>,
    pub path_suffix: Option<String>,
    pub query_equals: Option<String>,
    pub query_contains: Option<String>,
    pub query_prefix: Option<String>,
    pub query_suffix: Option<String>,
    pub url_matches: Option<String>,
    pub schemes: Option<Vec<String>>,
}

impl UrlFilter {
    pub fn to_match_pattern(&self) -> String {
        // Convert UrlFilter to a content script match pattern
        if let Some(host) = &self.host_equals {
            format!("*://{host}/*")
        } else if let Some(pattern) = &self.url_matches {
            pattern.clone()
        } else {
            "*://*/*".to_string()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageAction {
    ShowPageAction,
    SetIcon { icon_path: String },
}

// ============================================================================
// TabGroups API Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabGroupsUsage {
    pub location: FileLocation,
    pub operation: String,  // create, update, query, etc.
}

// ============================================================================
// Conversion Strategy Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversionStrategy {
    // Fully automatic conversions
    CanvasWorker {
        worker_path: PathBuf,
        transfer_canvas: bool,
    },
    AudioWorker {
        worker_path: PathBuf,
    },
    BackgroundIntegration {
        merge_into_background: bool,
    },
    ContentScript {
        target_urls: Vec<String>,
        all_urls: bool,
    },
    
    // Semi-automatic (needs user input)
    InteractiveContentScript {
        suggested_urls: Vec<String>,
    },
    SplitConversion {
        strategies: Vec<Box<ConversionStrategy>>,
    },
    
    // Fallback
    ManualGuidance {
        reason: String,
        suggestions: Vec<String>,
    },
    
    // Tab Groups stub
    NoOpStub {
        api_name: String,
    },
}

// Re-export types from conversion module to avoid duplication
use super::conversion::{NewFile, ModifiedFile};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ManifestChange {
    AddContentScript {
        matches: Vec<String>,
        js: Vec<String>,
        run_at: String,
    },
    AddPermission(String),
    AddBackgroundScript(String),
    RemovePermission(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromeOnlyConversionResult {
    pub new_files: Vec<NewFile>,
    pub modified_files: Vec<ModifiedFile>,
    pub manifest_changes: Vec<ManifestChange>,
    pub removed_files: Vec<PathBuf>,
    pub instructions: Vec<String>,
}

impl Default for ChromeOnlyConversionResult {
    fn default() -> Self {
        Self {
            new_files: Vec::new(),
            modified_files: Vec::new(),
            manifest_changes: Vec::new(),
            removed_files: Vec::new(),
            instructions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionPreferences {
    pub prefer_workers: bool,
    pub inline_simple_ops: bool,
    pub create_polyfills: bool,
    pub prompt_for_urls: bool,
}

impl Default for ConversionPreferences {
    fn default() -> Self {
        Self {
            prefer_workers: true,
            inline_simple_ops: false,
            create_polyfills: true,
            prompt_for_urls: true,
        }
    }
}