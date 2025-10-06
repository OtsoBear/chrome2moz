//! Manifest parsing functionality

use crate::models::Manifest;
use anyhow::{Context, Result};
use std::path::Path;

/// Parse manifest.json from bytes
pub fn parse_manifest(content: &[u8]) -> Result<Manifest> {
    let manifest: Manifest = serde_json::from_slice(content)
        .context("Failed to parse manifest.json")?;
    
    // Basic validation
    if manifest.manifest_version != 2 && manifest.manifest_version != 3 {
        anyhow::bail!("Unsupported manifest version: {}", manifest.manifest_version);
    }
    
    Ok(manifest)
}

/// Parse manifest.json from file path
pub fn parse_manifest_from_file(path: impl AsRef<Path>) -> Result<Manifest> {
    let content = std::fs::read(path.as_ref())
        .context("Failed to read manifest file")?;
    parse_manifest(&content)
}

/// Parse manifest.json from string
pub fn parse_manifest_from_str(content: &str) -> Result<Manifest> {
    parse_manifest(content.as_bytes())
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
        
        let manifest = parse_manifest_from_str(json).unwrap();
        assert_eq!(manifest.manifest_version, 3);
        assert_eq!(manifest.name, "Test Extension");
        assert_eq!(manifest.version, "1.0.0");
    }
    
    #[test]
    fn test_parse_with_background() {
        let json = r#"{
            "manifest_version": 3,
            "name": "Test",
            "version": "1.0",
            "background": {
                "service_worker": "background.js"
            }
        }"#;
        
        let manifest = parse_manifest_from_str(json).unwrap();
        assert!(manifest.background.is_some());
        assert_eq!(
            manifest.background.unwrap().service_worker.unwrap(),
            "background.js"
        );
    }
}