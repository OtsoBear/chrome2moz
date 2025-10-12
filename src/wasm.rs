//! WebAssembly bindings for Chrome to Firefox converter

use wasm_bindgen::prelude::*;
use std::io::{Cursor, Write};
use std::collections::HashMap;
use std::path::PathBuf;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

/// Initialize panic hook for better error messages in browser console
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Convert a Chrome extension ZIP to Firefox format
/// Returns the converted extension as a ZIP file (bytes) or an error message
#[wasm_bindgen]
pub fn convert_extension_zip(zip_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    console_log!("Starting conversion...");
    
    // 1. Load extension from ZIP bytes
    let extension = load_extension_from_bytes(zip_data)
        .map_err(|e| JsValue::from_str(&format!("Failed to load extension: {}", e)))?;
    
    console_log!("Extension loaded: {} v{}", extension.metadata.name, extension.metadata.version);
    
    // 2. Analyze extension
    let context = crate::analyzer::analyze_extension(extension)
        .map_err(|e| JsValue::from_str(&format!("Analysis failed: {}", e)))?;
    
    console_log!("Analysis complete. Found {} incompatibilities", context.incompatibilities.len());
    
    // 3. Apply default decisions (non-interactive)
    let context = crate::apply_default_decisions(context);
    
    // 4. Transform extension
    let result = crate::transformer::transform_extension(context)
        .map_err(|e| JsValue::from_str(&format!("Transformation failed: {}", e)))?;
    
    console_log!("Transformation complete. Modified {} files", result.modified_files.len());
    
    // 5. Package as ZIP
    let zip_bytes = create_zip_from_result(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to create ZIP: {}", e)))?;
    
    console_log!("Conversion successful! ZIP size: {} bytes", zip_bytes.len());
    
    Ok(zip_bytes)
}

/// Analyze keyboard shortcuts for conflicts with Firefox
#[wasm_bindgen]
pub fn analyze_keyboard_shortcuts(zip_data: &[u8]) -> Result<String, JsValue> {
    // Load extension
    let extension = load_extension_from_bytes(zip_data)
        .map_err(|e| JsValue::from_str(&format!("Failed to load extension: {}", e)))?;
    
    // Analyze shortcuts
    let analysis = crate::analyzer::analyze_shortcuts(&extension);
    
    // Convert to JSON
    serde_json::to_string_pretty(&analysis)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize shortcuts: {}", e)))
}

/// Get conversion report as JSON without converting
#[wasm_bindgen]
pub fn analyze_extension_zip(zip_data: &[u8]) -> Result<String, JsValue> {
    // 1. Load extension
    let extension = load_extension_from_bytes(zip_data)
        .map_err(|e| JsValue::from_str(&format!("Failed to load extension: {}", e)))?;
    
    // 2. Analyze
    let context = crate::analyzer::analyze_extension(extension)
        .map_err(|e| JsValue::from_str(&format!("Analysis failed: {}", e)))?;
    
    // 3. Create report JSON
    let report = serde_json::json!({
        "extension_name": context.source.metadata.name,
        "extension_version": context.source.metadata.version,
        "manifest_version": context.source.metadata.manifest_version,
        "file_count": context.source.metadata.file_count,
        "line_count": context.source.metadata.line_count,
        "size_bytes": context.source.metadata.size_bytes,
        "incompatibilities": context.incompatibilities.iter().map(|i| {
            serde_json::json!({
                "severity": format!("{:?}", i.severity),
                "location": format!("{}", &i.location),
                "description": &i.description,
                "auto_fixable": i.auto_fixable,
                "suggestion": &i.suggestion,
            })
        }).collect::<Vec<_>>(),
        "warnings": context.warnings.iter().map(|w| {
            serde_json::json!({
                "message": &w.message,
                "location": w.location.as_ref().map(|l| l.as_str()).unwrap_or(""),
            })
        }).collect::<Vec<_>>(),
        "decisions": context.decisions.iter().map(|d| {
            serde_json::json!({
                "question": d.question,
                "context": d.context,
                "options": d.options.iter().map(|o| o.label.clone()).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
    });
    
    serde_json::to_string_pretty(&report)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize report: {}", e)))
}

/// Load extension from raw ZIP bytes
fn load_extension_from_bytes(zip_data: &[u8]) -> anyhow::Result<crate::models::Extension> {
    use zip::ZipArchive;
    use crate::parser::manifest::parse_manifest;
    
    let cursor = Cursor::new(zip_data);
    let mut archive = ZipArchive::new(cursor)?;
    
    let mut files = HashMap::new();
    let mut manifest_content = None;
    
    // Extract all files
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        
        if file.is_file() {
            let path = PathBuf::from(file.name());
            let mut content = Vec::new();
            std::io::copy(&mut file, &mut content)?;
            
            // Save manifest separately
            if path.file_name().and_then(|n| n.to_str()) == Some("manifest.json") {
                manifest_content = Some(content.clone());
            }
            
            files.insert(path, content);
        }
    }
    
    // Parse manifest
    let manifest = manifest_content
        .ok_or_else(|| anyhow::anyhow!("manifest.json not found in ZIP"))
        .and_then(|content| parse_manifest(&content))?;
    
    Ok(crate::models::Extension::new(manifest, files))
}

/// Create ZIP file from conversion result
fn create_zip_from_result(result: &crate::models::ConversionResult) -> anyhow::Result<Vec<u8>> {
    use zip::write::{FileOptions, ZipWriter};
    use zip::CompressionMethod;
    use std::collections::HashSet;
    
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    
    // FileOptions without time feature (WASM compatible)
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o755);
    
    // Track which files have been written
    let mut written_files = HashSet::new();
    
    // 1. Write manifest.json
    let manifest_json = serde_json::to_string_pretty(&result.manifest)?;
    zip.start_file("manifest.json", options)?;
    zip.write_all(manifest_json.as_bytes())?;
    written_files.insert(PathBuf::from("manifest.json"));
    
    // 2. Write modified files
    for modified in &result.modified_files {
        let path_str: String = modified.path.to_string_lossy().into_owned();
        zip.start_file(path_str, options)?;
        zip.write_all(modified.new_content.as_bytes())?;
        written_files.insert(modified.path.clone());
    }
    
    // 3. Write new files (shims)
    for new_file in &result.new_files {
        let path_str: String = new_file.path.to_string_lossy().into_owned();
        zip.start_file(path_str, options)?;
        zip.write_all(new_file.content.as_bytes())?;
        written_files.insert(new_file.path.clone());
    }
    
    // 4. Copy all other original files
    for (path, content) in &result.source.files {
        if !written_files.contains(path) {
            let path_str: String = path.to_string_lossy().into_owned();
            zip.start_file(path_str, options)?;
            zip.write_all(content)?;
        }
    }
    
    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}