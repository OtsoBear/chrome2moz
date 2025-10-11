//! Analyzer for chrome.declarativeContent API usage

use crate::models::chrome_only::*;
use anyhow::Result;
use std::path::Path;

pub struct DeclarativeContentAnalyzer;

impl DeclarativeContentAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze JavaScript code for declarativeContent usage
    pub fn analyze_usage(&self, code: &str, file_path: &Path) -> Result<Vec<DeclarativeContentRule>> {
        let mut rules = Vec::new();
        
        // Look for chrome.declarativeContent.onPageChanged.addRules calls
        if code.contains("declarativeContent") && code.contains("addRules") {
            let lines: Vec<&str> = code.lines().collect();
            for (line_num, line) in lines.iter().enumerate() {
                if line.contains("declarativeContent") && line.contains("addRules") {
                    // Extract rule details
                    let rule = DeclarativeContentRule {
                        conditions: self.extract_conditions(code),
                        actions: self.extract_actions(code),
                        location: FileLocation::new(
                            file_path.to_path_buf(),
                            line_num + 1,
                            0,
                        ),
                    };
                    rules.push(rule);
                }
            }
        }
        
        Ok(rules)
    }

    fn extract_conditions(&self, code: &str) -> Vec<PageCondition> {
        let mut conditions = Vec::new();
        
        // Look for PageStateMatcher
        if code.contains("PageStateMatcher") {
            let url_filter = UrlFilter {
                host_equals: self.extract_string_value(code, "hostEquals"),
                host_contains: self.extract_string_value(code, "hostContains"),
                host_prefix: None,
                host_suffix: None,
                path_equals: None,
                path_contains: None,
                path_prefix: None,
                path_suffix: None,
                query_equals: None,
                query_contains: None,
                query_prefix: None,
                query_suffix: None,
                url_matches: self.extract_string_value(code, "urlMatches"),
                schemes: None,
            };
            
            let css = self.extract_css_selectors(code);
            
            conditions.push(PageCondition::PageStateMatcher {
                page_url: url_filter,
                css,
                is_bookmarked: None,
            });
        }
        
        conditions
    }

    fn extract_actions(&self, code: &str) -> Vec<PageAction> {
        let mut actions = Vec::new();
        
        if code.contains("ShowPageAction") {
            actions.push(PageAction::ShowPageAction);
        }
        
        if code.contains("SetIcon") {
            if let Some(icon_path) = self.extract_string_value(code, "path") {
                actions.push(PageAction::SetIcon { icon_path });
            }
        }
        
        actions
    }

    fn extract_string_value(&self, code: &str, key: &str) -> Option<String> {
        if let Some(start) = code.find(&format!("{key}:")) {
            let after_key = &code[start + key.len() + 1..];
            if let Some(quote_start) = after_key.find(|c| c == '\'' || c == '"') {
                let quote_char = after_key.chars().nth(quote_start).unwrap();
                let content = &after_key[quote_start + 1..];
                if let Some(quote_end) = content.find(quote_char) {
                    return Some(content[..quote_end].to_string());
                }
            }
        }
        None
    }

    fn extract_css_selectors(&self, code: &str) -> Option<Vec<String>> {
        if let Some(start) = code.find("css:") {
            let after_css = &code[start + 4..];
            if let Some(bracket_start) = after_css.find('[') {
                if let Some(bracket_end) = after_css.find(']') {
                    let array_content = &after_css[bracket_start + 1..bracket_end];
                    let selectors: Vec<String> = array_content
                        .split(',')
                        .map(|s| s.trim().trim_matches(|c| c == '\'' || c == '"').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if !selectors.is_empty() {
                        return Some(selectors);
                    }
                }
            }
        }
        None
    }
}

impl Default for DeclarativeContentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}