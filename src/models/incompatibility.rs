//! Incompatibility tracking and reporting

use std::path::PathBuf;
use serde::{Serialize, Deserialize};

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

#[derive(Debug, Clone, PartialEq, Eq)]
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
    ImportScripts,
    ServiceWorkerLifecycle,
}

#[derive(Debug, Clone)]
pub enum Location {
    Manifest,
    ManifestField(String),
    File(PathBuf),
    FileLocation(PathBuf, usize),
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

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "INFO"),
            Severity::Minor => write!(f, "MINOR"),
            Severity::Major => write!(f, "MAJOR"),
            Severity::Blocker => write!(f, "BLOCKER"),
        }
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Location::Manifest => write!(f, "manifest.json"),
            Location::ManifestField(field) => write!(f, "manifest.json:{}", field),
            Location::File(path) => write!(f, "{}", path.display()),
            Location::FileLocation(path, line) => write!(f, "{}:{}", path.display(), line),
        }
    }
}