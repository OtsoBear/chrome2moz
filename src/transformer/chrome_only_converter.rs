//! Main coordinator for Chrome-only API conversions

use crate::analyzer::{OffscreenAnalyzer, DeclarativeContentAnalyzer};
use crate::transformer::{
    OffscreenConverter, DeclarativeContentConverter, TabGroupsConverter,
};
use crate::models::chrome_only::*;
use crate::models::ConversionContext;
use anyhow::Result;
use std::path::PathBuf;

pub struct ChromeOnlyApiConverter {
    offscreen_analyzer: OffscreenAnalyzer,
    offscreen_converter: OffscreenConverter,
    declarative_content_analyzer: DeclarativeContentAnalyzer,
    declarative_content_converter: DeclarativeContentConverter,
    tab_groups_converter: TabGroupsConverter,
}

impl ChromeOnlyApiConverter {
    pub fn new(source_dir: PathBuf) -> Self {
        let preferences = ConversionPreferences::default();

        Self {
            offscreen_analyzer: OffscreenAnalyzer::new(source_dir.clone()),
            offscreen_converter: OffscreenConverter::new(source_dir, preferences),
            declarative_content_analyzer: DeclarativeContentAnalyzer::new(),
            declarative_content_converter: DeclarativeContentConverter::new(),
            tab_groups_converter: TabGroupsConverter::new(),
        }
    }

    /// Convert all Chrome-only APIs found in the extension
    pub fn convert_all(&self, context: &ConversionContext) -> Result<Vec<ChromeOnlyConversionResult>> {
        let mut all_results = Vec::new();

        // 1. Detect and convert chrome.offscreen usage
        let offscreen_results = self.convert_offscreen_apis(context)?;
        all_results.extend(offscreen_results);

        // 2. Detect and convert chrome.declarativeContent
        let declarative_results = self.convert_declarative_content(context)?;
        all_results.extend(declarative_results);

        // 3. Detect and convert chrome.tabGroups
        let tab_groups_result = self.convert_tab_groups(context)?;
        if let Some(result) = tab_groups_result {
            all_results.push(result);
        }

        Ok(all_results)
    }

    fn convert_offscreen_apis(
        &self,
        context: &ConversionContext,
    ) -> Result<Vec<ChromeOnlyConversionResult>> {
        let mut results = Vec::new();

        // Scan all JavaScript files for offscreen usage
        for js_path in context.source.get_javascript_files() {
            if let Some(content) = context.source.get_file_content(&js_path) {
                let usages = self.offscreen_analyzer.detect_usage(&content, &js_path)?;

                for usage in usages {
                    // Analyze the offscreen document
                    let analysis = self
                        .offscreen_analyzer
                        .analyze_offscreen_document(&usage.document_url)?;

                    // Determine conversion strategy
                    let strategy = self.offscreen_converter.determine_strategy(&analysis, &usage);

                    // Execute conversion based on strategy
                    let result = match strategy {
                        ConversionStrategy::CanvasWorker { .. } => {
                            self.offscreen_converter
                                .convert_canvas_to_worker(&analysis, &usage)?
                        }
                        ConversionStrategy::AudioWorker { .. } => {
                            self.offscreen_converter
                                .convert_audio_to_worker(&analysis, &usage)?
                        }
                        ConversionStrategy::BackgroundIntegration { .. } => {
                            self.offscreen_converter
                                .convert_network_to_background(&analysis, &usage)?
                        }
                        ConversionStrategy::ContentScript { .. } => {
                            self.offscreen_converter
                                .convert_dom_to_content_script(&analysis, &usage)?
                        }
                        ConversionStrategy::ManualGuidance { reason, suggestions } => {
                            // Create a guidance document
                            ChromeOnlyConversionResult {
                                new_files: vec![],
                                modified_files: vec![],
                                manifest_changes: vec![],
                                removed_files: vec![],
                                instructions: vec![
                                    format!("Manual conversion needed: {}", reason),
                                    "Suggestions:".to_string(),
                                ]
                                .into_iter()
                                .chain(suggestions)
                                .collect(),
                            }
                        }
                        _ => {
                            // Other strategies not yet implemented
                            ChromeOnlyConversionResult::default()
                        }
                    };

                    results.push(result);
                }
            }
        }

        Ok(results)
    }

    fn convert_declarative_content(
        &self,
        context: &ConversionContext,
    ) -> Result<Vec<ChromeOnlyConversionResult>> {
        let mut all_rules = Vec::new();

        // Scan all JavaScript files for declarativeContent usage
        for js_path in context.source.get_javascript_files() {
            if let Some(content) = context.source.get_file_content(&js_path) {
                let rules = self
                    .declarative_content_analyzer
                    .analyze_usage(&content, &js_path)?;
                all_rules.extend(rules);
            }
        }

        if all_rules.is_empty() {
            return Ok(Vec::new());
        }

        // Convert all rules together
        let result = if all_rules.len() > 5 || self.has_complex_rules(&all_rules) {
            self.declarative_content_converter
                .convert_complex_conditions(&all_rules)?
        } else {
            self.declarative_content_converter.convert(&all_rules)?
        };

        Ok(vec![result])
    }

    fn convert_tab_groups(&self, context: &ConversionContext) -> Result<Option<ChromeOnlyConversionResult>> {
        // Check if any file uses chrome.tabGroups
        for js_path in context.source.get_javascript_files() {
            if let Some(content) = context.source.get_file_content(&js_path) {
                if content.contains("chrome.tabGroups") || content.contains("browser.tabGroups") {
                    // Generate stub
                    return Ok(Some(self.tab_groups_converter.generate_stub()?));
                }
            }
        }

        Ok(None)
    }

    fn has_complex_rules(&self, rules: &[DeclarativeContentRule]) -> bool {
        // Consider complex if:
        // - More than 3 conditions per rule
        // - Uses advanced URL patterns
        // - Has multiple actions
        rules.iter().any(|r| {
            r.conditions.len() > 3
                || r.actions.len() > 2
                || r.conditions.iter().any(|c| match c {
                    PageCondition::PageStateMatcher { css, .. } => {
                        css.as_ref().map_or(false, |s| s.len() > 3)
                    }
                })
        })
    }

    /// Merge multiple conversion results into one
    pub fn merge_results(
        results: Vec<ChromeOnlyConversionResult>,
    ) -> ChromeOnlyConversionResult {
        let mut merged = ChromeOnlyConversionResult::default();

        for result in results {
            merged.new_files.extend(result.new_files);
            merged.modified_files.extend(result.modified_files);
            merged.manifest_changes.extend(result.manifest_changes);
            merged.removed_files.extend(result.removed_files);
            merged.instructions.extend(result.instructions);
        }

        merged
    }
}