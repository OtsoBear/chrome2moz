//! Extension extraction from archives and directories

use crate::models::Extension;
use crate::parser::manifest::parse_manifest;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::ZipArchive;

/// Load extension from directory
pub fn load_from_directory(dir: &Path) -> Result<Extension> {
    let mut files = HashMap::new();
    
    // Read manifest first
    let manifest_path = dir.join("manifest.json");
    let manifest_content = fs::read(&manifest_path)
        .context("Failed to read manifest.json")?;
    let manifest = parse_manifest(&manifest_content)?;
    
    // Read all files
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let path = entry.path();
            let relative_path = path.strip_prefix(dir)
                .context("Failed to get relative path")?;
            
            let content = fs::read(path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            
            files.insert(relative_path.to_path_buf(), content);
        }
    }
    
    Ok(Extension::new(manifest, files))
}

/// Load extension from ZIP or CRX archive
pub fn load_from_archive(archive_path: &Path) -> Result<Extension> {
    let file = fs::File::open(archive_path)
        .context("Failed to open archive")?;
    
    let mut archive = ZipArchive::new(file)
        .context("Failed to read ZIP archive")?;
    
    let mut files = HashMap::new();
    let mut manifest_content = None;
    
    // Extract all files
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .context("Failed to read file from archive")?;
        
        if file.is_file() {
            let path = PathBuf::from(file.name());
            let mut content = Vec::new();
            std::io::copy(&mut file, &mut content)
                .context("Failed to read file content")?;
            
            // Save manifest content separately
            if path.file_name().and_then(|n| n.to_str()) == Some("manifest.json") {
                manifest_content = Some(content.clone());
            }
            
            files.insert(path, content);
        }
    }
    
    // Parse manifest
    let manifest = manifest_content
        .ok_or_else(|| anyhow::anyhow!("manifest.json not found in archive"))
        .and_then(|content| parse_manifest(&content))?;
    
    Ok(Extension::new(manifest, files))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_load_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_content = r#"{
            "manifest_version": 3,
            "name": "Test",
            "version": "1.0"
        }"#;
        
        fs::write(temp_dir.path().join("manifest.json"), manifest_content).unwrap();
        fs::write(temp_dir.path().join("background.js"), "console.log('test');").unwrap();
        
        let extension = load_from_directory(temp_dir.path()).unwrap();
        assert_eq!(extension.manifest.name, "Test");
        assert_eq!(extension.files.len(), 2);
    }
}