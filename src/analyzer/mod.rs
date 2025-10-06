//! Analysis modules for detecting incompatibilities

pub mod manifest;
pub mod api;

use crate::models::{Extension, ConversionContext};
use anyhow::Result;

/// Analyze an extension for Chrome-to-Firefox incompatibilities
pub fn analyze_extension(extension: Extension) -> Result<ConversionContext> {
    let mut context = ConversionContext::new(extension);
    
    // 1. Analyze manifest
    let manifest_issues = manifest::analyze_manifest(&context.source.manifest);
    for issue in manifest_issues {
        context.add_incompatibility(issue);
    }
    
    // 2. Analyze JavaScript files for Chrome API usage
    for js_path in context.source.get_javascript_files() {
        if let Some(content) = context.source.get_file_content(&js_path) {
            let api_issues = api::analyze_javascript_apis(&content, &js_path);
            for issue in api_issues {
                context.add_incompatibility(issue);
            }
        }
    }
    
    // 3. Generate user decisions for non-auto-fixable issues
    generate_decisions(&mut context);
    
    Ok(context)
}

fn generate_decisions(context: &mut ConversionContext) {
    use crate::models::{UserDecision, DecisionCategory, DecisionOption};
    
    // Check if we need background architecture decision
    if context.source.manifest.background.as_ref()
        .and_then(|b| b.service_worker.as_ref())
        .is_some()
    {
        context.decisions.push(UserDecision {
            id: "background_architecture".to_string(),
            category: DecisionCategory::BackgroundArchitecture,
            question: "Your extension uses a service worker. How should we handle Firefox compatibility?".to_string(),
            context: "Firefox MV3 uses event pages instead of service workers.".to_string(),
            options: vec![
                DecisionOption {
                    label: "Create event page (recommended)".to_string(),
                    description: "Convert service worker to event page with equivalent functionality".to_string(),
                    recommended: true,
                },
                DecisionOption {
                    label: "Keep both".to_string(),
                    description: "Keep service_worker for Chrome and add scripts for Firefox".to_string(),
                    recommended: false,
                },
            ],
            default_index: 0,
        });
    }
    
    // Check if we need extension ID decision
    if context.source.manifest.browser_specific_settings.is_none() {
        context.decisions.push(UserDecision {
            id: "extension_id".to_string(),
            category: DecisionCategory::ExtensionId,
            question: "Choose Firefox extension ID format:".to_string(),
            context: "Firefox requires a unique extension ID for submission to AMO.".to_string(),
            options: vec![
                DecisionOption {
                    label: "Email-style (recommended)".to_string(),
                    description: format!("{}@converted.extension", 
                        context.source.manifest.name.to_lowercase().replace(' ', "-")),
                    recommended: true,
                },
                DecisionOption {
                    label: "UUID format".to_string(),
                    description: "{12345678-1234-1234-1234-123456789012}".to_string(),
                    recommended: false,
                },
            ],
            default_index: 0,
        });
    }
}