//! Report generation

use crate::models::ConversionResult;
use anyhow::Result;

pub fn generate_markdown_report(result: &ConversionResult) -> Result<String> {
    let mut report = String::new();
    
    report.push_str("# Chrome to Firefox Extension Conversion Report\n\n");
    
    // Summary
    report.push_str("## Summary\n\n");
    report.push_str(&format!("- **Extension**: {} v{}\n", 
        result.report.summary.extension_name,
        result.report.summary.extension_version));
    report.push_str(&format!("- **Conversion Status**: {}\n", 
        if result.report.summary.conversion_successful { "✅ Success" } else { "❌ Failed" }));
    report.push_str(&format!("- **Files Modified**: {}\n", result.report.summary.files_modified));
    report.push_str(&format!("- **Files Added**: {}\n", result.report.summary.files_added));
    report.push_str(&format!("- **Total Changes**: {}\n", result.report.summary.total_changes));
    report.push_str(&format!("- **Chrome API Calls Converted**: {}\n", 
        result.report.summary.chrome_api_calls_converted));
    report.push_str(&format!("- **Callback→Promise Conversions**: {}\n\n", 
        result.report.summary.callback_to_promise_conversions));
    
    // Manifest Changes
    if !result.report.manifest_changes.is_empty() {
        report.push_str("## Manifest Changes\n\n");
        for change in &result.report.manifest_changes {
            report.push_str(&format!("- {}\n", change));
        }
        report.push_str("\n");
    }
    
    // JavaScript Changes
    if !result.report.javascript_changes.is_empty() {
        report.push_str("## JavaScript Transformations\n\n");
        for change in &result.report.javascript_changes {
            report.push_str(&format!("- {}\n", change));
        }
        report.push_str("\n");
    }
    
    // Blockers
    if !result.report.blockers.is_empty() {
        report.push_str("## ⛔ Blockers\n\n");
        for blocker in &result.report.blockers {
            report.push_str(&format!("- {}\n", blocker));
        }
        report.push_str("\n");
    }
    
    // Manual Actions
    if !result.report.manual_actions.is_empty() {
        report.push_str("## ⚠️ Manual Actions Required\n\n");
        for action in &result.report.manual_actions {
            report.push_str(&format!("- {}\n", action));
        }
        report.push_str("\n");
    }
    
    // Warnings
    if !result.report.warnings.is_empty() {
        report.push_str("## ℹ️ Warnings\n\n");
        for warning in &result.report.warnings {
            report.push_str(&format!("- {}\n", warning));
        }
        report.push_str("\n");
    }
    
    // Next Steps
    report.push_str("## Next Steps\n\n");
    report.push_str("1. Review the converted extension files\n");
    report.push_str("2. Test the extension in Firefox\n");
    report.push_str("3. Address any manual action items listed above\n");
    report.push_str("4. Submit to Firefox Add-ons (AMO) when ready\n\n");
    
    Ok(report)
}