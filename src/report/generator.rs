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
    
    // Detailed File Changes
    if !result.modified_files.is_empty() {
        report.push_str("## Modified Files - Detailed Changes\n\n");
        for modified in &result.modified_files {
            report.push_str(&format!("### {}\n\n", modified.path.display()));
            report.push_str(&format!("**{} changes made**\n\n", modified.changes.len()));
            
            for change in &modified.changes {
                report.push_str(&format!("- **Line {}**: {}\n", change.line_number, change.description));
            }
            report.push_str("\n");
        }
    }
    
    // Added Files
    if !result.new_files.is_empty() {
        report.push_str("## Added Compatibility Shims\n\n");
        for new_file in &result.new_files {
            report.push_str(&format!("- **{}**\n", new_file.path.display()));
            report.push_str(&format!("  - Purpose: {}\n", new_file.purpose));
        }
        report.push_str("\n");
    }
    
    // Manifest Changes
    if !result.report.manifest_changes.is_empty() {
        report.push_str("## Manifest Changes\n\n");
        for change in &result.report.manifest_changes {
            report.push_str(&format!("- {}\n", change));
        }
        report.push_str("\n");
    }
    
    // JavaScript Changes Summary
    if !result.report.javascript_changes.is_empty() {
        report.push_str("## JavaScript Transformations Summary\n\n");
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
    
    // Warnings with detailed explanations
    if !result.report.warnings.is_empty() {
        report.push_str("## ℹ️ Warnings & What They Mean\n\n");
        for warning in &result.report.warnings {
            report.push_str(&format!("- {}\n", warning));
            
            // Add detailed explanations for common warnings
            if warning.contains("service worker") {
                report.push_str("\n### Service Worker → Event Page Conversion\n\n");
                report.push_str("**CHROME (Service Worker):**\n");
                report.push_str("- Can be terminated at ANY time by browser\n");
                report.push_str("- NO access to DOM or localStorage\n");
                report.push_str("- Must use chrome.storage API for persistence\n");
                report.push_str("- Restarts on API events (messages, alarms, etc.)\n\n");
                
                report.push_str("**FIREFOX (Event Page):**\n");
                report.push_str("- Stays loaded longer, terminated after ~30 seconds of inactivity\n");
                report.push_str("- CAN access DOM and use limited localStorage\n");
                report.push_str("- Better suited for persistent listeners\n");
                report.push_str("- Reloads on extension startup or API events\n\n");
                
                report.push_str("**WHAT COULD BREAK:**\n");
                report.push_str("- ❌ Assumptions about always-running background script\n");
                report.push_str("- ❌ Complex in-memory state not using global variables\n\n");
                
                report.push_str("**WHAT WORKS AUTOMATICALLY:**\n");
                report.push_str("- ✅ Global variables (auto-persisted with browser.storage.local)\n");
                report.push_str("- ✅ Long timers (setTimeout/setInterval >30s converted to browser.alarms)\n");
                report.push_str("- ✅ Event listeners (runtime.onMessage, tabs.onUpdated, etc.)\n");
                report.push_str("- ✅ chrome.alarms for scheduled/recurring tasks\n");
                report.push_str("- ✅ chrome.storage for persisting data\n");
                report.push_str("- ✅ Message passing between scripts\n");
                report.push_str("- ✅ Short-lived operations and immediate responses\n\n");
                
                report.push_str("**AUTO-GENERATED FEATURES:**\n\n");
                
                report.push_str("📦 **Global Variable Persistence:**\n");
                report.push_str("- Automatically detects global variables in background scripts\n");
                report.push_str("- Generates code to save/restore them using browser.storage.local\n");
                report.push_str("- Variables are restored on event page startup\n");
                report.push_str("- Auto-saves on changes (1-second debounce)\n");
                report.push_str("- Saves immediately on page termination\n\n");
                
                report.push_str("⏰ **Long Timer Conversion:**\n");
                report.push_str("- setTimeout/setInterval with delays >30 seconds automatically converted\n");
                report.push_str("- Converted to browser.alarms API (survives termination)\n");
                report.push_str("- Generates alarm listeners to execute original callback code\n");
                report.push_str("- Short timers (<30s) remain unchanged\n\n");
                
                report.push_str("**ACTION REQUIRED:**\n");
                report.push_str("- Test timer-based features thoroughly (use chrome.alarms for long delays)\n");
                report.push_str("- Verify data persistence works across browser restarts\n");
                report.push_str("- Check that message listeners respond correctly\n");
                report.push_str("- Review console logs for global variable save/restore operations\n\n");
            } else if warning.contains("extension ID") || warning.contains("default extension ID") {
                report.push_str("\n**Extension ID Information:**\n");
                report.push_str("- Firefox requires a unique extension ID for AMO submission\n");
                report.push_str("- The generated ID uses email format: `name@domain`\n");
                report.push_str("- For development/testing, the default ID works fine\n");
                report.push_str("- For publishing to AMO, customize in: `manifest.json → browser_specific_settings.gecko.id`\n\n");
            }
        }
        report.push_str("\n");
    }
    
    // Important Notes
    report.push_str("## 📝 Important Notes\n\n");
    report.push_str("### What Was Automatically Converted\n\n");
    report.push_str("✅ **These changes were made automatically:**\n");
    report.push_str("- All `chrome.*` API calls converted to `browser.*`\n");
    report.push_str("- Browser namespace polyfills added to all JavaScript files\n");
    report.push_str("- Service worker converted to event page (if applicable)\n");
    report.push_str("- Manifest updated for Firefox compatibility\n");
    report.push_str("- `executeScript` calls transformed to message passing\n");
    report.push_str("- Compatibility shims added for missing APIs\n\n");
    
    report.push_str("### What Needs Manual Review\n\n");
    report.push_str("⚠️ **Messages like \"Callback detected - consider converting to promise\" mean:**\n");
    report.push_str("- The tool **detected** a callback-based API call\n");
    report.push_str("- The call was converted from `chrome.*` to `browser.*`\n");
    report.push_str("- The callback-based code **will work** via compatibility layer\n");
    report.push_str("- **However**, promises are preferred for Firefox\n\n");
    
    report.push_str("**Callback vs Promise Support:**\n");
    report.push_str("- **Chrome**: Supports BOTH callbacks AND promises natively\n");
    report.push_str("- **Firefox**: `browser.*` API returns promises natively\n");
    report.push_str("- Callbacks work via webextension-polyfill compatibility layer\n");
    report.push_str("- Promises are MORE reliable and the preferred Firefox style\n\n");
    
    report.push_str("**Why convert to promises:**\n");
    report.push_str("- Better error handling with try/catch\n");
    report.push_str("- Cleaner code with async/await syntax\n");
    report.push_str("- Native Firefox API behavior (no polyfill needed)\n");
    report.push_str("- Avoids potential polyfill overhead\n");
    report.push_str("- More maintainable and modern JavaScript\n\n");
    
    report.push_str("**Example:**\n");
    report.push_str("```javascript\n");
    report.push_str("// Current (works but not ideal):\n");
    report.push_str("browser.storage.local.get('key', (result) => {\n");
    report.push_str("  console.log(result);\n");
    report.push_str("});\n\n");
    report.push_str("// Better (Firefox-native style):\n");
    report.push_str("browser.storage.local.get('key').then((result) => {\n");
    report.push_str("  console.log(result);\n");
    report.push_str("});\n\n");
    report.push_str("// Or with async/await:\n");
    report.push_str("const result = await browser.storage.local.get('key');\n");
    report.push_str("console.log(result);\n");
    report.push_str("```\n\n");
    
    // Next Steps
    report.push_str("## Next Steps\n\n");
    report.push_str("1. **Test in Firefox**: Load the extension in Firefox (`about:debugging#/runtime/this-firefox`)\n");
    report.push_str("2. **Check Console**: Open Browser Console (Ctrl+Shift+J) and look for any errors\n");
    report.push_str("3. **Review Callbacks**: Consider converting callback-based calls to promises for better Firefox compatibility\n");
    report.push_str("4. **Address Warnings**: Review and fix any warnings listed above\n");
    report.push_str("5. **Test Features**: Verify all extension features work as expected\n");
    report.push_str("6. **Customize ID**: If publishing to AMO, set a proper extension ID\n");
    report.push_str("7. **Submit to AMO**: When ready, submit to Firefox Add-ons\n\n");
    
    Ok(report)
}