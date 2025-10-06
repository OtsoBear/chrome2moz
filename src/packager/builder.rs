//! Firefox extension package builder

use crate::models::ConversionResult;
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;

/// Build Firefox XPI package
pub fn build_xpi(result: &ConversionResult, output_path: &Path) -> Result<()> {
    build_directory(result, output_path)?;
    
    // Then create ZIP from directory
    let zip_path = output_path.with_extension("xpi");
    create_zip_from_directory(output_path, &zip_path)?;
    
    Ok(())
}

pub fn create_zip_from_directory(source_dir: &Path, zip_path: &Path) -> Result<()> {
    use walkdir::WalkDir;
    
    let file = File::create(zip_path)
        .context("Failed to create ZIP file")?;
    let mut zip = ZipWriter::new(file);
    
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o755);
    
    for entry in WalkDir::new(source_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let relative_path = path.strip_prefix(source_dir)
                .context("Failed to get relative path")?;
            
            zip.start_file(relative_path.to_string_lossy().as_ref(), options)?;
            let content = fs::read(path)?;
            zip.write_all(&content)?;
        }
    }
    
    zip.finish()?;
    Ok(())
}

/// Build directory structure (for development)
/// This function is now used internally by build_xpi
pub fn build_directory(result: &ConversionResult, output_path: &Path) -> Result<()> {
    fs::create_dir_all(output_path)?;
    
    // Write manifest.json
    let manifest_json = serde_json::to_string_pretty(&result.manifest)?;
    fs::write(output_path.join("manifest.json"), manifest_json)?;
    
    // Write modified files
    for modified in &result.modified_files {
        let file_path = output_path.join(&modified.path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, &modified.new_content)?;
    }
    
    // Write new files (shims)
    for new_file in &result.new_files {
        let file_path = output_path.join(&new_file.path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, &new_file.content)?;
    }
    
    Ok(())
}

/// Build directory with all original files plus modifications
pub fn build_complete_directory(
    source_extension: &crate::models::Extension,
    result: &ConversionResult,
    output_path: &Path
) -> Result<()> {
    use std::collections::HashSet;
    
    fs::create_dir_all(output_path)?;
    
    // Track which files have been modified
    let modified_paths: HashSet<_> = result.modified_files.iter()
        .map(|f| f.path.clone())
        .collect();
    
    // 1. Copy all original files (except those that will be modified)
    for (path, content) in &source_extension.files {
        if path.file_name().and_then(|n| n.to_str()) == Some("manifest.json") {
            continue; // Skip manifest, we'll write the transformed one
        }
        
        if !modified_paths.contains(path) {
            let dest_path = output_path.join(path);
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(dest_path, content)?;
        }
    }
    
    // 2. Write transformed manifest
    let manifest_json = serde_json::to_string_pretty(&result.manifest)?;
    fs::write(output_path.join("manifest.json"), manifest_json)?;
    
    // 3. Write modified files
    for modified in &result.modified_files {
        let file_path = output_path.join(&modified.path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, &modified.new_content)?;
    }
    
    // 4. Write new files (shims)
    for new_file in &result.new_files {
        let file_path = output_path.join(&new_file.path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, &new_file.content)?;
    }
    
    Ok(())
}