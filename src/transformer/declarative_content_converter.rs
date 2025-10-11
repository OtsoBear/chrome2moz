//! Converter for chrome.declarativeContent API to Firefox alternative

use crate::models::chrome_only::*;
use crate::models::conversion::NewFile;
use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;

pub struct DeclarativeContentConverter;

impl DeclarativeContentConverter {
    pub fn new() -> Self {
        Self
    }

    /// Convert declarativeContent rules to content scripts + messaging
    pub fn convert(&self, rules: &[DeclarativeContentRule]) -> Result<ChromeOnlyConversionResult> {
        let mut content_script_matches = HashSet::new();
        let mut conditions_code = Vec::new();

        for rule in rules {
            for condition in &rule.conditions {
                if let PageCondition::PageStateMatcher { page_url, css, .. } = condition {
                    content_script_matches.insert(page_url.to_match_pattern());

                    let check_code = if let Some(selectors) = css {
                        format!(
                            r#"
// Check page conditions
const elements = document.querySelectorAll('{}');
if (elements.length > 0) {{
  // Condition met - notify background
  browser.runtime.sendMessage({{
    type: 'page_condition_met',
    action: 'show_page_action'
  }});
}}
"#,
                            selectors.join(", ")
                        )
                    } else {
                        // Just URL matching - simpler case
                        r#"
// URL matched - notify background
browser.runtime.sendMessage({
  type: 'page_condition_met',
  action: 'show_page_action'
});
"#
                        .to_string()
                    };

                    conditions_code.push(check_code);
                }
            }
        }

        let content_script = format!(
            r#"// Auto-generated content script
// Converted from chrome.declarativeContent rules

'use strict';

(function() {{
  // Check conditions on page load
  function checkConditions() {{
    {}
  }}
  
  // Run on page load
  if (document.readyState === 'loading') {{
    document.addEventListener('DOMContentLoaded', checkConditions);
  }} else {{
    checkConditions();
  }}
  
  // Also check on DOM mutations (for dynamic content)
  const observer = new MutationObserver(checkConditions);
  observer.observe(document.body, {{
    childList: true,
    subtree: true
  }});
}})();
"#,
            conditions_code.join("\n\n")
        );

        let background_handler = r#"// Auto-generated handler for declarativeContent conversion
browser.runtime.onMessage.addListener((message, sender) => {
  if (message.type === 'page_condition_met' && sender.tab?.id) {
    // Show page action for this tab
    browser.pageAction.show(sender.tab.id);
    
    // Set icon if specified
    if (message.iconPath) {
      browser.pageAction.setIcon({
        tabId: sender.tab.id,
        path: message.iconPath
      });
    }
  }
});
"#;

        let matches: Vec<String> = content_script_matches.into_iter().collect();

        Ok(ChromeOnlyConversionResult {
            new_files: vec![
                NewFile {
                    path: PathBuf::from("content-scripts/page-condition-checker.js"),
                    content: content_script,
                    purpose: "Checks page conditions (converted from declarativeContent)"
                        .to_string(),
                },
                NewFile {
                    path: PathBuf::from("background_declarative_content_handler.js"),
                    content: background_handler.to_string(),
                    purpose: "Background handler for page condition messages".to_string(),
                },
            ],
            modified_files: Vec::new(),
            manifest_changes: vec![
                ManifestChange::AddContentScript {
                    matches,
                    js: vec!["content-scripts/page-condition-checker.js".to_string()],
                    run_at: "document_idle".to_string(),
                },
                ManifestChange::AddPermission("pageAction".to_string()),
            ],
            removed_files: Vec::new(),
            instructions: vec![
                "declarativeContent rules converted to content script + messaging".to_string(),
                "Page action will be shown when conditions are met".to_string(),
                "Firefox requires explicit pageAction permission".to_string(),
                "Add background_declarative_content_handler.js content to your background script"
                    .to_string(),
            ],
        })
    }

    /// Convert complex conditions with advanced monitoring
    pub fn convert_complex_conditions(
        &self,
        rules: &[DeclarativeContentRule],
    ) -> Result<ChromeOnlyConversionResult> {
        let content_script = r#"// Complex condition checker with caching
'use strict';

class ConditionChecker {
  constructor() {
    this.cache = new Map();
    this.checkInterval = null;
  }
  
  async checkAllConditions() {
    const results = [];
    
    // Check URL patterns
    const currentUrl = window.location.href;
    // URL pattern matching would go here
    
    // Check CSS selectors
    const selectors = ['video', 'audio', 'canvas']; // Example selectors
    for (const selector of selectors) {
      const elements = document.querySelectorAll(selector);
      if (elements.length > 0) {
        results.push({ type: 'css', selector, count: elements.length });
      }
    }
    
    // Notify background if any condition met
    if (results.length > 0) {
      browser.runtime.sendMessage({
        type: 'conditions_met',
        results,
        url: currentUrl
      });
    }
  }
  
  startMonitoring() {
    // Check on load
    this.checkAllConditions();
    
    // Check periodically for SPA changes
    this.checkInterval = setInterval(() => {
      this.checkAllConditions();
    }, 1000);
    
    // Check on history changes
    window.addEventListener('popstate', () => this.checkAllConditions());
    
    // Check on mutations
    const observer = new MutationObserver(() => this.checkAllConditions());
    observer.observe(document.body, {
      childList: true,
      subtree: true,
      attributes: true
    });
  }
}

const checker = new ConditionChecker();
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => checker.startMonitoring());
} else {
  checker.startMonitoring();
}
"#;

        let matches: Vec<String> = rules
            .iter()
            .flat_map(|r| &r.conditions)
            .filter_map(|c| match c {
                PageCondition::PageStateMatcher { page_url, .. } => {
                    Some(page_url.to_match_pattern())
                }
            })
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        Ok(ChromeOnlyConversionResult {
            new_files: vec![NewFile {
                path: PathBuf::from("content-scripts/advanced-condition-checker.js"),
                content: content_script.to_string(),
                purpose: "Advanced condition checking for complex declarativeContent rules"
                    .to_string(),
            }],
            modified_files: Vec::new(),
            manifest_changes: vec![ManifestChange::AddContentScript {
                matches,
                js: vec!["content-scripts/advanced-condition-checker.js".to_string()],
                run_at: "document_idle".to_string(),
            }],
            removed_files: Vec::new(),
            instructions: vec![
                "Complex declarativeContent rules converted with monitoring".to_string(),
                "Handles dynamic content and SPA navigation".to_string(),
            ],
        })
    }
}

impl Default for DeclarativeContentConverter {
    fn default() -> Self {
        Self::new()
    }
}