//! Chrome to Firefox Extension Converter
//! 
//! A library for converting Chrome MV3 extensions to Firefox-compatible MV3 extensions.
//! Handles manifest transformation, JavaScript code rewriting, and compatibility shims.

pub mod models;
pub mod parser;
pub mod analyzer;
pub mod transformer;
pub mod packager;
pub mod validator;
pub mod report;
pub mod utils;
pub mod scripts;
pub mod cli;

pub use models::{Extension, Manifest, ConversionContext, ConversionResult};
pub use analyzer::analyze_extension;
pub use transformer::transform_extension;

use anyhow::Result;
use std::path::Path;

/// Main entry point for converting a Chrome extension to Firefox
pub fn convert_extension(
    input_path: &Path,
    output_path: &Path,
    options: ConversionOptions,
) -> Result<ConversionResult> {
    // 1. Extract/load extension
    let extension = packager::load_extension(input_path)?;
    
    // 2. Analyze for incompatibilities
    let context = analyze_extension(extension)?;
    
    // 3. Get user decisions if needed
    let context = if options.interactive {
        get_user_decisions(context)?
    } else {
        apply_default_decisions(context)
    };
    
    // 4. Transform extension
    let result = transform_extension(context)?;
    
    // 5. Validate result
    validator::validate_extension(&result)?;
    
    // 6. Package output (extension is now in result.source)
    packager::build_complete_extension(&result.source, &result, output_path)?;
    
    // 7. Generate report
    let _report = report::generate_report(&result)?;
    
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct ConversionOptions {
    pub interactive: bool,
    pub target_calculator: CalculatorType,
    pub preserve_chrome_compatibility: bool,
    pub generate_report: bool,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            interactive: true,
            target_calculator: CalculatorType::Both,
            preserve_chrome_compatibility: true,
            generate_report: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalculatorType {
    TiNspire,
    Speedcrunch,
    Both,
}

fn get_user_decisions(context: ConversionContext) -> Result<ConversionContext> {
    // TODO: Implement interactive decision gathering
    Ok(context)
}

fn apply_default_decisions(mut context: ConversionContext) -> ConversionContext {
    // Apply default decisions for non-interactive mode
    use models::{SelectedDecision, DecisionCategory};
    
    // Clone decisions to avoid borrow issues
    let decisions = context.decisions.clone();
    
    for decision in &decisions {
        let selected_index = decision.default_index;
        
        context.selected_decisions.push(SelectedDecision {
            decision_id: decision.id.clone(),
            selected_index,
        });
        
        // Add specific handling based on decision category
        match decision.category {
            DecisionCategory::BackgroundArchitecture => {
                context.add_warning(
                    "Using default: Converting service worker to event page",
                    Some("manifest.json".to_string())
                );
            }
            DecisionCategory::ExtensionId => {
                let ext_id = format!("{}@converted.extension",
                    context.source.metadata.name.to_lowercase().replace(' ', "-"));
                context.add_warning(
                    format!("Using default extension ID: {}", ext_id),
                    Some("manifest.json".to_string())
                );
            }
            _ => {}
        }
    }
    
    context
}