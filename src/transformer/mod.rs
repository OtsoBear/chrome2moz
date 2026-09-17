//! Transformation modules for converting Chrome extensions to Firefox
//! Simplified: No AST transformation, just pass-through with runtime shims

pub mod manifest;
pub mod javascript;
pub mod shims;
pub mod tab_groups;
pub mod offscreen_converter;
pub mod declarative_content_converter;
pub mod chrome_only_converter;
pub mod html_inject;

pub use manifest::ManifestTransformer;
pub use javascript::JavaScriptTransformer;
pub use shims::generate_shims;
pub use tab_groups::TabGroupsConverter;
pub use offscreen_converter::OffscreenConverter;
pub use declarative_content_converter::DeclarativeContentConverter;
pub use chrome_only_converter::ChromeOnlyApiConverter;

use crate::models::{ConversionContext, ConversionResult};
use anyhow::Result;

/// Main transformation entry point (simplified pass-through)
pub fn transform_extension(context: ConversionContext) -> Result<ConversionResult> {
    let mut manifest_changes = Vec::new();
    let mut javascript_changes = Vec::new();
    let mut chrome_api_count = 0;
    let mut callback_count = 0;
    
    // 1. Transform manifest (pass source for importScripts detection)
    let manifest_transformer = ManifestTransformer::new(&context.selected_decisions);
    let transformed_manifest = manifest_transformer.transform(&context.source.manifest, Some(&context.source))?;
    
    // Track manifest changes
    if context.source.manifest.browser_specific_settings.is_none() {
        manifest_changes.push("Added browser_specific_settings.gecko.id for Firefox".to_string());
    }
    if context.source.manifest.background.as_ref().and_then(|b| b.service_worker.as_ref()).is_some() {
        manifest_changes.push("Added background.scripts for Firefox event page compatibility".to_string());
    }
    if shims::will_inject_offscreen_polyfill(&context.source) {
        manifest_changes.push("Injected offscreen polyfill (shims/offscreen-polyfill.js) for chrome.offscreen / getContexts(OFFSCREEN_DOCUMENT) compatibility".to_string());
    }

    // 2. Transform JavaScript files
    let mut js_transformer = JavaScriptTransformer::new(&context.selected_decisions);
    let mut modified_files = Vec::new();
    
    for js_path in context.source.get_javascript_files() {
        if let Some(content) = context.source.get_file_content(&js_path) {
            if let Ok(transformed) = js_transformer.transform(&content, &js_path) {
                if transformed.new_content != content {
                    // Count changes
                    chrome_api_count += transformed.changes.iter()
                        .filter(|c| c.description.contains("chrome.*"))
                        .count();
                    callback_count += transformed.changes.iter()
                        .filter(|c| c.description.contains("Callback"))
                        .count();
                    
                    javascript_changes.push(format!(
                        "{}: {} changes",
                        js_path.display(),
                        transformed.changes.len()
                    ));
                    
                    modified_files.push(transformed);
                }
            }
        }
    }
    
    // 2b. Inject the onMessage compat shim as the first <script> of every packaged HTML
    // page (offscreen documents / popups / options pages), so listeners there are fixed
    // too; background.scripts injection (manifest transform) alone cannot reach them.
    let html_paths: Vec<_> = context.source.files.keys()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"))
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    let mut html_pages_injected = 0usize;
    for html_path in &html_paths {
        if let Some(content) = context.source.get_file_content(html_path) {
            if let Some(new_content) = html_inject::inject_onmessage_shim(&content, html_path) {
                html_pages_injected += 1;
                modified_files.push(crate::models::ModifiedFile {
                    path: html_path.clone(),
                    original_content: content.clone(),
                    new_content,
                    changes: vec![crate::models::FileChange {
                        line_number: 0,
                        change_type: crate::models::ChangeType::Addition,
                        description: "Injected onMessage compat shim".to_string(),
                        old_code: None,
                        new_code: Some(
                            "<script src=\"shims/runtime-onmessage-compat.js\"></script>".to_string(),
                        ),
                    }],
                });
            }
        }
    }
    if html_pages_injected > 0 {
        manifest_changes.push(format!(
            "Injected runtime.onMessage compat shim into {} HTML page(s) (issue #8)",
            html_pages_injected
        ));
    }

    // 3. Generate compatibility shims
    let shims = generate_shims(&context)?;
    
    // 4. Build report
    let report = crate::models::ConversionReport {
        summary: crate::models::ReportSummary {
            extension_name: context.source.metadata.name.clone(),
            extension_version: context.source.metadata.version.clone(),
            conversion_successful: !context.has_blockers(),
            files_modified: modified_files.len(),
            files_added: shims.len(),
            total_changes: modified_files.iter().map(|f| f.changes.len()).sum(),
            chrome_api_calls_converted: chrome_api_count,
            callback_to_promise_conversions: callback_count,
        },
        manifest_changes,
        javascript_changes,
        blockers: context.incompatibilities.iter()
            .filter(|i| matches!(i.severity, crate::models::Severity::Blocker))
            .map(|i| format!("{}: {}", i.location, i.description))
            .collect(),
        manual_actions: context.incompatibilities.iter()
            .filter(|i| !i.auto_fixable && matches!(i.severity, crate::models::Severity::Major))
            .map(|i| format!("{}: {}", i.location, i.description))
            .collect(),
        warnings: context.warnings.iter()
            .map(|w| w.message.clone())
            .collect(),
    };
    
    Ok(ConversionResult {
        source: context.source,
        manifest: transformed_manifest,
        modified_files,
        new_files: shims,
        report,
    })
}

impl Default for crate::models::ConversionReport {
    fn default() -> Self {
        Self {
            summary: crate::models::ReportSummary {
                extension_name: String::new(),
                extension_version: String::new(),
                conversion_successful: false,
                files_modified: 0,
                files_added: 0,
                total_changes: 0,
                chrome_api_calls_converted: 0,
                callback_to_promise_conversions: 0,
            },
            manifest_changes: Vec::new(),
            javascript_changes: Vec::new(),
            blockers: Vec::new(),
            manual_actions: Vec::new(),
            warnings: Vec::new(),
        }
    }
}