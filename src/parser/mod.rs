//! Parsing modules for manifest and JavaScript files

pub mod manifest;
pub mod javascript;

pub use manifest::parse_manifest;
pub use javascript::{analyze_javascript, JavaScriptAnalyzer};