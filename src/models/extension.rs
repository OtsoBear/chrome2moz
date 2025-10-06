//! Extension representation and metadata

use super::manifest::Manifest;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Extension {
    pub manifest: Manifest,
    pub files: HashMap<PathBuf, Vec<u8>>,
    pub metadata: ExtensionMetadata,
}

#[derive(Debug, Clone)]
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
    
    /// Get all JavaScript files in the extension
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
    
    /// Get file content as string (for text files)
    pub fn get_file_content(&self, path: &PathBuf) -> Option<String> {
        self.files.get(path).and_then(|bytes| {
            String::from_utf8(bytes.clone()).ok()
        })
    }
    
    /// Get background script paths
    pub fn get_background_scripts(&self) -> Vec<PathBuf> {
        let mut scripts = Vec::new();
        
        if let Some(background) = &self.manifest.background {
            if let Some(service_worker) = &background.service_worker {
                scripts.push(PathBuf::from(service_worker));
            }
            if let Some(script_list) = &background.scripts {
                scripts.extend(script_list.iter().map(PathBuf::from));
            }
        }
        
        scripts
    }
    
    /// Get content script paths
    pub fn get_content_script_paths(&self) -> Vec<PathBuf> {
        self.manifest
            .content_scripts
            .iter()
            .flat_map(|cs| cs.js.iter().map(PathBuf::from))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct JavaScriptFile {
    pub path: PathBuf,
    pub content: String,
    pub is_background: bool,
    pub is_content_script: bool,
    pub chrome_api_calls: Vec<ChromeApiCall>,
}

#[derive(Debug, Clone)]
pub struct ChromeApiCall {
    pub line: usize,
    pub column: usize,
    pub api_name: String,
    pub full_call: String,
    pub is_callback_style: bool,
    pub is_chrome_only: bool,
}