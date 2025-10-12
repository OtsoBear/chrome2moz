//! Chrome-only API data structures and loader

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dataset of Chrome-only APIs with their compatibility info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChromeApiDataset {
    /// When this data was last updated
    pub updated_at: String,
    /// Source commit/version
    pub source_version: String,
    /// Map of API path to info
    pub apis: HashMap<String, ChromeApiInfo>,
}

impl ChromeApiDataset {
    /// Load from embedded JSON or return empty dataset
    pub fn load() -> Self {
        // Try to load from embedded data first
        match Self::load_embedded() {
            Ok(dataset) => dataset,
            Err(_) => {
                eprintln!("Warning: Could not load Chrome API data, using fallback");
                Self::create_fallback()
            }
        }
    }

    /// Load from embedded JSON string
    fn load_embedded() -> Result<Self, Box<dyn std::error::Error>> {
        let json = include_str!("../../chrome_only_apis.json");
        let dataset: Self = serde_json::from_str(json)?;
        Ok(dataset)
    }

    /// Create fallback dataset with hardcoded APIs
    fn create_fallback() -> Self {
        let mut dataset = Self {
            updated_at: "fallback".to_string(),
            source_version: "hardcoded".to_string(),
            apis: HashMap::new(),
        };

        // Add known Chrome-only APIs from parser/javascript.rs
        let fallback_apis = [
            "chrome.offscreen",
            "chrome.declarativeContent",
            "chrome.tabGroups",
            "chrome.sidePanel",
            "chrome.action.openPopup",
            "chrome.declarativeNetRequest",
            "chrome.userScripts",
            "chrome.storage.session",
            "chrome.runtime.getPackageDirectoryEntry",
            "chrome.tabs.getSelected",
            "chrome.tabs.getAllInWindow",
            "chrome.downloads.acceptDanger",
            "chrome.downloads.setShelfEnabled",
        ];

        for api in fallback_apis {
            let category = ApiCategory::from_path(api);
            dataset.apis.insert(api.to_string(), ChromeApiInfo {
                path: api.to_string(),
                chrome_version: "supported".to_string(),
                firefox_status: FirefoxStatus::NotSupported,
                category,
                has_converter: matches!(
                    category,
                    ApiCategory::Offscreen
                        | ApiCategory::DeclarativeContent
                        | ApiCategory::TabGroups
                        | ApiCategory::SidePanel
                        | ApiCategory::Storage
                ),
                description: format!("Chrome-only API: {}", api),
            });
        }

        dataset
    }

    /// Check if an API is Chrome-only
    pub fn is_chrome_only(&self, api_path: &str) -> bool {
        self.apis.keys().any(|key| api_path.starts_with(key))
    }

    /// Get info for a specific API (finds best prefix match)
    pub fn get_info(&self, api_path: &str) -> Option<&ChromeApiInfo> {
        // Try exact match first
        if let Some(info) = self.apis.get(api_path) {
            return Some(info);
        }

        // Find longest matching prefix
        self.apis
            .iter()
            .filter(|(key, _)| api_path.starts_with(key.as_str()))
            .max_by_key(|(key, _)| key.len())
            .map(|(_, info)| info)
    }

    /// Get all API paths sorted
    pub fn get_all_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self.apis.keys().cloned().collect();
        paths.sort();
        paths
    }
}

/// Information about a Chrome-only API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromeApiInfo {
    /// API path (e.g., "chrome.offscreen")
    pub path: String,
    /// Chrome version that added this API
    pub chrome_version: String,
    /// Firefox support status
    pub firefox_status: FirefoxStatus,
    /// Category for grouping
    pub category: ApiCategory,
    /// Whether we have a converter
    pub has_converter: bool,
    /// Human-readable description
    pub description: String,
}

impl ChromeApiInfo {
    /// Get user-friendly warning message
    pub fn get_warning(&self) -> String {
        match self.firefox_status {
            FirefoxStatus::NotSupported => {
                format!("'{}' is not supported in Firefox", self.path)
            }
            FirefoxStatus::Partial => {
                format!("'{}' has limited support in Firefox", self.path)
            }
            FirefoxStatus::Deprecated => {
                format!("'{}' is deprecated", self.path)
            }
        }
    }

    /// Get suggestion for user
    pub fn get_suggestion(&self) -> String {
        if self.has_converter {
            format!(
                "This tool will automatically {}",
                match self.category {
                    ApiCategory::Offscreen => "convert to Web Workers or content scripts",
                    ApiCategory::DeclarativeContent => "convert to content script patterns",
                    ApiCategory::TabGroups => "provide a no-op stub",
                    ApiCategory::SidePanel => "map to Firefox sidebarAction",
                    ApiCategory::Storage => "provide an in-memory polyfill",
                    ApiCategory::DeclarativeNetRequest => "provide a stub with webRequest guidance",
                    ApiCategory::Other => "include a compatibility shim",
                }
            )
        } else {
            format!(
                "Manual conversion required. {}",
                match self.category {
                    ApiCategory::Offscreen => "Consider using Web Workers",
                    ApiCategory::DeclarativeContent => "Consider using content scripts",
                    ApiCategory::TabGroups => "Consider alternative UI",
                    ApiCategory::SidePanel => "Use Firefox sidebarAction API",
                    ApiCategory::Storage => "Use chrome.storage.local instead",
                    ApiCategory::DeclarativeNetRequest => "Use webRequest API",
                    ApiCategory::Other => "Check Firefox API docs",
                }
            )
        }
    }
}

/// Firefox support status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirefoxStatus {
    NotSupported,
    Partial,
    Deprecated,
}

/// API category for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiCategory {
    Offscreen,
    DeclarativeContent,
    TabGroups,
    SidePanel,
    Storage,
    DeclarativeNetRequest,
    Other,
}

impl ApiCategory {
    pub fn from_path(path: &str) -> Self {
        if path.contains("offscreen") {
            Self::Offscreen
        } else if path.contains("declarativeContent") {
            Self::DeclarativeContent
        } else if path.contains("tabGroups") {
            Self::TabGroups
        } else if path.contains("sidePanel") {
            Self::SidePanel
        } else if path.contains("storage.session") {
            Self::Storage
        } else if path.contains("declarativeNetRequest") {
            Self::DeclarativeNetRequest
        } else {
            Self::Other
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_fallback() {
        let dataset = ChromeApiDataset::create_fallback();
        assert!(!dataset.apis.is_empty());
        assert!(dataset.is_chrome_only("chrome.offscreen.createDocument"));
    }

    #[test]
    fn test_category_detection() {
        assert_eq!(
            ApiCategory::from_path("chrome.offscreen.createDocument"),
            ApiCategory::Offscreen
        );
        assert_eq!(
            ApiCategory::from_path("chrome.tabGroups.query"),
            ApiCategory::TabGroups
        );
    }

    #[test]
    fn test_get_info() {
        let dataset = ChromeApiDataset::create_fallback();
        let info = dataset.get_info("chrome.offscreen.createDocument");
        assert!(info.is_some());
        assert_eq!(info.unwrap().category, ApiCategory::Offscreen);
    }
}