//! Conversion context and results

use super::{Extension, Incompatibility, Manifest};
use std::path::PathBuf;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct ConversionResult {
    pub source: Extension,
    pub manifest: Manifest,
    pub modified_files: Vec<ModifiedFile>,
    pub new_files: Vec<NewFile>,
    pub report: ConversionReport,
}

#[derive(Debug, Clone)]
pub struct ModifiedFile {
    pub path: PathBuf,
    pub original_content: String,
    pub new_content: String,
    pub changes: Vec<FileChange>,
}

#[derive(Debug, Clone)]
pub struct NewFile {
    pub path: PathBuf,
    pub content: String,
    pub purpose: String,
}

#[derive(Debug, Clone)]
pub struct FileChange {
    pub line_number: usize,
    pub change_type: ChangeType,
    pub description: String,
    pub old_code: Option<String>,
    pub new_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeType {
    Addition,
    Modification,
    Deletion,
}

#[derive(Debug, Clone)]
pub struct ConversionReport {
    pub summary: ReportSummary,
    pub manifest_changes: Vec<String>,
    pub javascript_changes: Vec<String>,
    pub blockers: Vec<String>,
    pub manual_actions: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReportSummary {
    pub extension_name: String,
    pub extension_version: String,
    pub conversion_successful: bool,
    pub files_modified: usize,
    pub files_added: usize,
    pub total_changes: usize,
    pub chrome_api_calls_converted: usize,
    pub callback_to_promise_conversions: usize,
}

impl ConversionContext {
    pub fn new(extension: Extension) -> Self {
        Self {
            source: extension,
            incompatibilities: Vec::new(),
            warnings: Vec::new(),
            decisions: Vec::new(),
            selected_decisions: Vec::new(),
        }
    }
    
    pub fn add_incompatibility(&mut self, incompatibility: Incompatibility) {
        self.incompatibilities.push(incompatibility);
    }
    
    pub fn add_warning(&mut self, message: impl Into<String>, location: Option<String>) {
        self.warnings.push(Warning {
            message: message.into(),
            location,
        });
    }
    
    pub fn has_blockers(&self) -> bool {
        self.incompatibilities
            .iter()
            .any(|i| matches!(i.severity, super::incompatibility::Severity::Blocker))
    }
}