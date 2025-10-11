//! JavaScript code transformation for Chrome to Firefox conversion

use crate::models::{ModifiedFile, FileChange, ChangeType, SelectedDecision};
use anyhow::Result;
use regex::Regex;
use lazy_static::lazy_static;
use std::path::PathBuf;

/// Helper function to determine if a variable should be persisted
fn should_persist_variable(name: &str) -> bool {
    // Skip common patterns that shouldn't be persisted
    let skip_patterns = vec![
        // Browser APIs and built-ins
        "chrome", "browser", "window", "document", "console",
        // Common constants
        "API_KEY", "VERSION", "CONFIG",
        // Event listeners
        "listener", "handler",
        // Regex patterns
        "REGEX", "PATTERN",
        // Imports
        "import", "require",
    ];
    
    // Skip if name matches common skip patterns
    for pattern in skip_patterns {
        if name.to_uppercase().contains(&pattern.to_uppercase()) {
            return false;
        }
    }
    
    // Skip if variable name is all caps (likely a constant)
    if name.chars().all(|c| c.is_uppercase() || c == '_') {
        return false;
    }
    
    // Skip if it looks like a class or constructor
    if name.chars().next().map_or(false, |c| c.is_uppercase()) {
        return false;
    }
    
    true
}

lazy_static! {
    // Regex patterns for Chrome API transformations
    static ref CHROME_NAMESPACE: Regex = Regex::new(r"\bchrome\.").unwrap();
    static ref CHROME_RUNTIME_LASTERROR: Regex = Regex::new(r"chrome\.runtime\.lastError").unwrap();
    static ref IMPORT_SCRIPTS: Regex = Regex::new(r"importScripts\s*\((.*?)\)").unwrap();
    
    // Callback pattern detection
    static ref CALLBACK_PATTERN: Regex = Regex::new(
        r"chrome\.(\w+)\.(\w+)\s*\((.*?),\s*(?:function\s*\(|(?:\w+\s*=>|\(\w+\)\s*=>))"
    ).unwrap();
    
    // Pattern to detect scripting.executeScript calls
    static ref EXECUTE_SCRIPT_PATTERN: Regex = Regex::new(
        r"(?:chrome|browser)\.scripting\.executeScript\s*\("
    ).unwrap();
    
    // Pattern to detect chrome:// URLs
    static ref CHROME_URL_PATTERN: Regex = Regex::new(
        r#"['"]chrome://([a-zA-Z0-9\-_]+)/?([^'"]*?)['"]"#
    ).unwrap();
    
    // Pattern to detect setTimeout/setInterval with long delays
    static ref SETTIMEOUT_PATTERN: Regex = Regex::new(
        r"setTimeout\s*\(\s*([^,]+),\s*(\d+)\s*\)"
    ).unwrap();
    
    static ref SETINTERVAL_PATTERN: Regex = Regex::new(
        r"setInterval\s*\(\s*([^,]+),\s*(\d+)\s*\)"
    ).unwrap();
}

/// Structure to hold extracted executeScript information
#[derive(Debug, Clone)]
pub(crate) struct ExecuteScriptCall {
    start_line: usize,
    end_line: usize,
    tab_id_expr: String,
    function_body: String,
    function_name: Option<String>, // For function references
    function_params: Vec<String>, // Original function parameter names
    args: Vec<String>,
    background_vars: Vec<String>, // Variables from background.js used in function
    has_callback: bool,
    callback_body: String,
    full_text: String,
}

/// Structure to hold information about a callback to be transformed
#[derive(Debug, Clone)]
struct CallbackInfo {
    line_number: usize,
    start_pos: usize,
    end_pos: usize,
    start_line: usize,
    end_line: usize,
    api_namespace: String,
    api_call: String,
    api_method: String,
    args: String,
    callback_param: String,
    callback_body: String,
    has_error_check: bool,
    nesting_depth: usize,
    has_control_flow: bool,
    is_named_function: bool,
    original_text: String,
}

pub struct JavaScriptTransformer {
    decisions: Vec<SelectedDecision>,
    execute_script_calls: Vec<ExecuteScriptCall>,
    global_variables: Vec<GlobalVariable>,
    converted_timers: Vec<TimerConversion>,
}

/// Structure to track timer conversions
#[derive(Debug, Clone)]
struct TimerConversion {
    alarm_name: String,
    original_delay_ms: u64,
    is_interval: bool,
    callback_code: String,
}

/// Structure to track global variables for persistence
#[derive(Debug, Clone)]
struct GlobalVariable {
    name: String,
    line: usize,
    is_const: bool,
}

impl JavaScriptTransformer {
    pub fn new(decisions: &[SelectedDecision]) -> Self {
        Self {
            decisions: decisions.to_vec(),
            execute_script_calls: Vec::new(),
            global_variables: Vec::new(),
            converted_timers: Vec::new(),
        }
    }
    
    /// Transform JavaScript code from Chrome to Firefox compatibility
    pub fn transform(&mut self, content: &str, path: &PathBuf) -> Result<ModifiedFile> {
        let mut new_content = content.to_string();
        let mut changes = Vec::new();
        
        // 1. Detect global variables in background scripts for persistence
        if path.to_string_lossy().contains("background") {
            self.global_variables = self.detect_global_variables(&new_content);
        }
        
        // 2. Add browser polyfill at the top if chrome APIs are used
        if CHROME_NAMESPACE.is_match(&new_content) {
            new_content = self.add_browser_polyfill(&new_content);
            changes.push(FileChange {
                line_number: 1,
                change_type: ChangeType::Addition,
                description: "Added browser namespace polyfill".to_string(),
                old_code: None,
                new_code: Some("if (typeof browser === 'undefined') { var browser = chrome; }".to_string()),
            });
        }
        
        // 3. Add global variable persistence wrapper (background scripts only)
        if path.to_string_lossy().contains("background") && !self.global_variables.is_empty() {
            let (wrapped, persist_changes) = self.add_variable_persistence_wrapper(&new_content);
            new_content = wrapped;
            changes.extend(persist_changes);
        }
        
        // 4. Convert chrome.* to browser.*
        let (transformed, chrome_changes) = self.convert_chrome_to_browser(&new_content);
        new_content = transformed;
        changes.extend(chrome_changes);
        
        // 5. Detect and extract executeScript calls AFTER chrome->browser conversion
        if path.to_string_lossy().contains("background") {
            self.execute_script_calls = self.extract_execute_script_calls(&new_content);
        }
        
        // 6. Transform executeScript to message passing (background.js)
        if path.to_string_lossy().contains("background") && !self.execute_script_calls.is_empty() {
            let (transformed, exec_changes) = self.transform_execute_script_to_messages(&new_content);
            new_content = transformed;
            changes.extend(exec_changes);
        }
        
        // 7. Convert callback-style to promise-style
        let (transformed, callback_changes) = self.convert_callbacks_to_promises(&new_content);
        new_content = transformed;
        changes.extend(callback_changes);
        
        // 8. Handle importScripts in service workers
        if path.to_string_lossy().contains("background") {
            let (transformed, import_changes) = self.handle_import_scripts(&new_content);
            new_content = transformed;
            changes.extend(import_changes);
        }
        
        // 9. Convert chrome.runtime.lastError checks
        let (transformed, error_changes) = self.convert_last_error_checks(&new_content);
        new_content = transformed;
        changes.extend(error_changes);
        
        // 10. Replace Chrome settings URLs with Firefox equivalents
        let (transformed, url_changes) = self.replace_chrome_urls(&new_content);
        new_content = transformed;
        changes.extend(url_changes);
        
        // 11. Convert long setTimeout/setInterval to chrome.alarms (background scripts only)
        if path.to_string_lossy().contains("background") {
            let (transformed, timer_changes) = self.convert_long_timers_to_alarms(&new_content);
            new_content = transformed;
            changes.extend(timer_changes);
        }
        
        Ok(ModifiedFile {
            path: path.clone(),
            original_content: content.to_string(),
            new_content,
            changes,
        })
    }
    
    /// Get the collected executeScript calls for generating content.js listeners
    pub fn get_execute_script_calls(&self) -> &[ExecuteScriptCall] {
        &self.execute_script_calls
    }
    
    /// Get detected global variables
    pub fn get_global_variables(&self) -> &[GlobalVariable] {
        &self.global_variables
    }
    
    /// Detect global variables in background script
    fn detect_global_variables(&self, content: &str) -> Vec<GlobalVariable> {
        let mut variables = Vec::new();
        
        // Patterns for variable declarations at global scope
        let var_patterns = vec![
            (Regex::new(r"^\s*let\s+(\w+)\s*=").unwrap(), false),
            (Regex::new(r"^\s*var\s+(\w+)\s*=").unwrap(), false),
            (Regex::new(r"^\s*const\s+(\w+)\s*=").unwrap(), true),
        ];
        
        // Track scope depth to only get globals
        let mut brace_depth = 0;
        
        for (line_num, line) in content.lines().enumerate() {
            // Check for variables BEFORE updating brace count for this line
            // This ensures we capture variables declared at the start of a line
            // even if the line contains opening braces
            if brace_depth == 0 {
                for (pattern, is_const) in &var_patterns {
                    if let Some(cap) = pattern.captures(line) {
                        if let Some(var_name) = cap.get(1) {
                            let name = var_name.as_str().to_string();
                            
                            // Skip common patterns that shouldn't be persisted
                            if should_persist_variable(&name) {
                                variables.push(GlobalVariable {
                                    name,
                                    line: line_num + 1,
                                    is_const: *is_const,
                                });
                            }
                        }
                    }
                }
            }
            
            // Track scope by counting braces
            for ch in line.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => brace_depth -= 1,
                    _ => {}
                }
            }
        }
        
        variables
    }
    
    /// Add variable persistence wrapper to background script
    fn add_variable_persistence_wrapper(&self, content: &str) -> (String, Vec<FileChange>) {
        let mut changes = Vec::new();
        
        // Include ALL global variables (const objects/arrays can have mutable contents)
        let persist_vars: Vec<&GlobalVariable> = self.global_variables.iter().collect();
        
        if persist_vars.is_empty() {
            return (content.to_string(), changes);
        }
        
        let var_names: Vec<String> = persist_vars.iter()
            .map(|v| format!("'{}'", v.name))
            .collect();
        
        let var_list = persist_vars.iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        
        // Generate persistence code
        let persistence_code = format!(r#"
// === AUTO-GENERATED: Global Variable Persistence ===
// Firefox event pages can be terminated, losing global variables.
// This code automatically saves/restores them using browser.storage.local.

const PERSIST_VARS = [{}];
const PERSIST_KEY = '__bg_vars_persist__';

// Restore variables on startup
(async function restoreGlobalVars() {{
    try {{
        const stored = await browser.storage.local.get(PERSIST_KEY);
        if (stored[PERSIST_KEY]) {{
            const saved = stored[PERSIST_KEY];
            console.log('🔄 Restoring global variables:', Object.keys(saved));
            {}
        }}
    }} catch (error) {{
        console.error('❌ Failed to restore global variables:', error);
    }}
}})();

// Save variables periodically and on changes
let saveTimeout;
const SAVE_DEBOUNCE_MS = 1000; // Configurable debounce delay

function saveGlobalVars(immediate = false) {{
    if (saveTimeout) clearTimeout(saveTimeout);
    
    const doSave = async () => {{
        try {{
            const toSave = {{{}}};
            await browser.storage.local.set({{ [PERSIST_KEY]: toSave }});
            console.log('💾 Saved global variables');
        }} catch (error) {{
            console.error('❌ Failed to save global variables:', error);
        }}
    }};
    
    if (immediate) {{
        doSave(); // Save immediately without debounce
    }} else {{
        saveTimeout = setTimeout(doSave, SAVE_DEBOUNCE_MS);
    }}
}}

// Auto-save on window unload (event page termination)
if (typeof window !== 'undefined') {{
    window.addEventListener('unload', () => {{
        // Synchronous save on unload
        const toSave = {{{}}};
        browser.storage.local.set({{ [PERSIST_KEY]: toSave }});
    }});
}}

// Proxy wrapper to auto-save on modifications
function createPersistentVar(varName, initialValue) {{
    let value = initialValue;
    return {{
        get: () => value,
        set: (newValue) => {{
            value = newValue;
            saveGlobalVars();
            return value;
        }}
    }};
}}

// === END AUTO-GENERATED ===

"#,
            var_names.join(", "),
            persist_vars.iter()
                .map(|v| {
                    if v.is_const {
                        // For const, we deep-merge to preserve object/array contents
                        format!("if (saved['{}'] !== undefined) {{ if (typeof {} === 'object') Object.assign({}, saved['{}']); }}",
                            v.name, v.name, v.name, v.name)
                    } else {
                        // For let/var, direct assignment is fine
                        format!("if (saved['{}'] !== undefined) {} = saved['{}'];", v.name, v.name, v.name)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n            "),
            persist_vars.iter()
                .map(|v| format!("{}: {}", v.name, v.name))
                .collect::<Vec<_>>()
                .join(", "),
            persist_vars.iter()
                .map(|v| format!("{}: {}", v.name, v.name))
                .collect::<Vec<_>>()
                .join(", ")
        );
        
        // Add at the beginning of the file (after any imports/polyfills)
        let new_content = format!("{}\n{}", persistence_code, content);
        
        changes.push(FileChange {
            line_number: 1,
            change_type: ChangeType::Addition,
            description: format!("Added automatic persistence for {} global variables: {}",
                persist_vars.len(), var_list),
            old_code: None,
            new_code: Some("Global variable persistence wrapper".to_string()),
        });
        
        (new_content, changes)
    }
    
    fn add_browser_polyfill(&self, content: &str) -> String {
        let polyfill = r#"// Browser namespace polyfill for Firefox compatibility
if (typeof browser === 'undefined') {
  var browser = chrome;
}

"#;
        format!("{}{}", polyfill, content)
    }
    
    fn convert_chrome_to_browser(&self, content: &str) -> (String, Vec<FileChange>) {
        let mut changes = Vec::new();
        let mut result = content.to_string();
        
        // Replace chrome. with browser. but preserve chrome.runtime.lastError for now
        let lines: Vec<&str> = content.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            if CHROME_NAMESPACE.is_match(line) && !line.contains("chrome.runtime.lastError") {
                let new_line = CHROME_NAMESPACE.replace_all(line, "browser.");
                if new_line != *line {
                    changes.push(FileChange {
                        line_number: line_num + 1,
                        change_type: ChangeType::Modification,
                        description: "Converted chrome.* to browser.*".to_string(),
                        old_code: Some(line.to_string()),
                        new_code: Some(new_line.to_string()),
                    });
                }
            }
        }
        
        result = CHROME_NAMESPACE.replace_all(&result, "browser.").to_string();
        
        (result, changes)
    }
    
    fn convert_callbacks_to_promises(&self, content: &str) -> (String, Vec<FileChange>) {
        let mut changes = Vec::new();
        let mut result = content.to_string();
        
        // Parse and transform callbacks
        let callbacks = self.parse_all_callbacks(&result);
        
        // Sort by position (reverse order) to maintain positions during replacement
        let mut transformable: Vec<_> = callbacks.into_iter()
            .filter(|cb| self.should_transform_callback(cb))
            .collect();
        transformable.sort_by(|a, b| b.start_pos.cmp(&a.start_pos));
        
        // Transform each callback
        for callback in transformable {
            let transformed = self.transform_callback_to_promise(&callback);
            
            // Replace in content
            result.replace_range(callback.start_pos..callback.end_pos, &transformed);
            
            changes.push(FileChange {
                line_number: callback.line_number,
                change_type: ChangeType::Modification,
                description: format!("Converted callback to promise: {}", callback.api_call),
                old_code: Some(callback.original_text.clone()),
                new_code: Some(transformed),
            });
        }
        
        (result, changes)
    }
    
    /// Parse all callback patterns in the code
    fn parse_all_callbacks(&self, content: &str) -> Vec<CallbackInfo> {
        let mut callbacks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        let mut line_idx = 0;
        while line_idx < lines.len() {
            // Check for callback patterns on this line
            if let Some(callback) = self.try_parse_callback_at_line(content, &lines, line_idx) {
                let end_line = callback.end_line;
                callbacks.push(callback);
                // Skip lines we've already processed
                line_idx = end_line;
            } else {
                line_idx += 1;
            }
        }
        
        callbacks
    }
    
    /// Try to parse a callback starting at the given line
    fn try_parse_callback_at_line(&self, full_content: &str, lines: &[&str], start_line: usize) -> Option<CallbackInfo> {
        let line = lines[start_line];
        
        // Pattern: browser/chrome.api.method(args, callback)
        // Look for patterns like:
        // - browser.storage.get('key', (result) => { ... })
        // - browser.storage.get('key', function(result) { ... })
        // - browser.tabs.query({}, tabs => { ... })
        
        let api_pattern = regex::Regex::new(
            r"(browser|chrome)\.([\w.]+)\s*\(([^,\)]*(?:,\s*[^,\)]*)*),\s*(function\s*\(|(?:\w+|\([^)]*\))\s*=>|\(\s*\w+\s*\)\s*=>)"
        ).unwrap();
        
        if let Some(cap) = api_pattern.captures(line) {
            let api_namespace = cap.get(1).unwrap().as_str();
            let api_method = cap.get(2).unwrap().as_str();
            let args = cap.get(3).unwrap().as_str();
            let callback_start = cap.get(4).unwrap();
            
            // Calculate start position in full content
            let start_pos = self.line_to_byte_pos(full_content, start_line) + cap.get(0).unwrap().start();
            
            // Extract callback parameter name
            let param_name = self.extract_callback_param(&callback_start.as_str());
            
            // Find the callback body by matching braces
            let callback_start_in_line = callback_start.end();
            let (callback_body, end_line, end_pos) = self.extract_callback_body_with_braces(
                full_content,
                lines,
                start_line,
                callback_start_in_line
            )?;
            
            // Check for error handling pattern
            let has_error_check = callback_body.contains("chrome.runtime.lastError")
                || callback_body.contains("browser.runtime.lastError");
            
            // Calculate nesting depth
            let nesting_depth = self.calculate_nesting_depth(&callback_body);
            
            // Check for control flow
            let has_control_flow = self.has_control_flow(&callback_body);
            
            // Reconstruct original text
            let original_text = full_content[start_pos..end_pos].to_string();
            
            return Some(CallbackInfo {
                line_number: start_line + 1,
                start_pos,
                end_pos,
                start_line,
                end_line,
                api_namespace: api_namespace.to_string(),
                api_call: format!("{}.{}", api_namespace, api_method),
                api_method: api_method.to_string(),
                args: args.trim().to_string(),
                callback_param: param_name,
                callback_body,
                has_error_check,
                nesting_depth,
                has_control_flow,
                is_named_function: false, // We don't match named function references
                original_text,
            });
        }
        
        None
    }
    
    /// Extract callback body by matching braces
    fn extract_callback_body_with_braces(
        &self,
        full_content: &str,
        lines: &[&str],
        start_line: usize,
        start_offset_in_line: usize
    ) -> Option<(String, usize, usize)> {
        let mut body = String::new();
        let mut brace_count = 0;
        let mut found_opening_brace = false;
        let mut current_line = start_line;
        let mut char_offset = start_offset_in_line;
        
        // Calculate starting byte position in full content
        let line_start_pos = self.line_to_byte_pos(full_content, start_line);
        let mut current_pos = line_start_pos + start_offset_in_line;
        
        while current_line < lines.len() {
            let line = lines[current_line];
            let chars: Vec<char> = line.chars().collect();
            
            while char_offset < chars.len() {
                let ch = chars[char_offset];
                
                if ch == '{' {
                    brace_count += 1;
                    if !found_opening_brace {
                        found_opening_brace = true;
                        char_offset += 1;
                        current_pos += 1;
                        continue; // Don't include opening brace
                    }
                } else if ch == '}' {
                    brace_count -= 1;
                    if brace_count == 0 && found_opening_brace {
                        // Found matching closing brace
                        // Skip any trailing ); or );
                        let mut end_pos = current_pos + 1;
                        let mut end_offset = char_offset + 1;
                        
                        // Skip closing parens and semicolons
                        while end_offset < chars.len() {
                            let next_ch = chars[end_offset];
                            if next_ch == ')' || next_ch == ';' || next_ch.is_whitespace() {
                                end_pos += 1;
                                end_offset += 1;
                                if next_ch == ';' {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        
                        return Some((body.trim().to_string(), current_line, end_pos));
                    }
                }
                
                if found_opening_brace && brace_count > 0 {
                    body.push(ch);
                }
                
                char_offset += 1;
                current_pos += 1;
            }
            
            // Move to next line
            if found_opening_brace && brace_count > 0 {
                body.push('\n');
            }
            current_line += 1;
            char_offset = 0;
            current_pos = self.line_to_byte_pos(full_content, current_line);
        }
        
        None
    }
    
    /// Convert line number to byte position in content
    fn line_to_byte_pos(&self, content: &str, line_num: usize) -> usize {
        content.lines()
            .take(line_num)
            .map(|line| line.len() + 1) // +1 for newline
            .sum()
    }
    
    /// Extract callback parameter name from callback declaration
    fn extract_callback_param(&self, callback_start: &str) -> String {
        // Handle different patterns:
        // - function(result)
        // - (result) =>
        // - result =>
        
        if callback_start.starts_with("function") {
            // Extract between ( and )
            if let Some(start) = callback_start.find('(') {
                if let Some(end) = callback_start.find(')') {
                    return callback_start[start + 1..end].trim().to_string();
                }
            }
        } else if callback_start.contains("=>") {
            // Arrow function
            let before_arrow = callback_start.split("=>").next().unwrap_or("").trim();
            if before_arrow.starts_with('(') && before_arrow.ends_with(')') {
                return before_arrow[1..before_arrow.len() - 1].trim().to_string();
            }
            return before_arrow.to_string();
        }
        
        "result".to_string() // Default
    }
    
    /// Calculate nesting depth of callbacks
    fn calculate_nesting_depth(&self, callback_body: &str) -> usize {
        let nested_pattern = regex::Regex::new(
            r"(browser|chrome)\.\w+\.\w+\s*\([^)]*,\s*(?:function\s*\(|\w+\s*=>|\([^)]*\)\s*=>)"
        ).unwrap();
        
        let matches: Vec<_> = nested_pattern.find_iter(callback_body).collect();
        matches.len() + 1 // +1 for the current callback
    }
    
    /// Check if callback body contains control flow keywords
    fn has_control_flow(&self, callback_body: &str) -> bool {
        let control_keywords = ["return ", "break ", "continue "];
        control_keywords.iter().any(|kw| callback_body.contains(kw))
    }
    
    /// Determine if a callback should be transformed
    fn should_transform_callback(&self, callback: &CallbackInfo) -> bool {
        // Skip if nesting is too deep (>3 levels)
        if callback.nesting_depth > 3 {
            return false;
        }
        
        // Skip if it's a named function reference
        if callback.is_named_function {
            return false;
        }
        
        // Skip if it has complex control flow
        if callback.has_control_flow {
            return false;
        }
        
        // Skip if callback has multiple parameters (unusual pattern)
        if callback.callback_param.contains(',') {
            return false;
        }
        
        true
    }
    
    /// Transform a callback to a promise-based pattern
    fn transform_callback_to_promise(&self, callback: &CallbackInfo) -> String {
        if callback.has_error_check {
            self.transform_error_checking_callback(callback)
        } else if callback.nesting_depth > 1 {
            self.transform_nested_callback(callback)
        } else {
            self.transform_simple_callback(callback)
        }
    }
    
    /// Transform a simple callback to .then()
    fn transform_simple_callback(&self, callback: &CallbackInfo) -> String {
        format!(
            "{}.{}({})\n    .then(({}) => {{\n{}\n    }})",
            callback.api_namespace,
            callback.api_method,
            callback.args,
            callback.callback_param,
            self.indent_code(&callback.callback_body, 8)
        )
    }
    
    /// Transform error-checking callback to .then().catch()
    fn transform_error_checking_callback(&self, callback: &CallbackInfo) -> String {
        // Parse the callback body to separate error handling from success code
        let (success_code, error_code) = self.split_error_handling(&callback.callback_body);
        
        let mut result = format!(
            "{}.{}({})\n    .then(({}) => {{",
            callback.api_namespace,
            callback.api_method,
            callback.args,
            callback.callback_param
        );
        
        if !success_code.trim().is_empty() {
            result.push_str(&format!("\n{}", self.indent_code(&success_code, 8)));
        }
        
        result.push_str("\n    })");
        
        if !error_code.trim().is_empty() {
            result.push_str(&format!(
                "\n    .catch((error) => {{\n{}\n    }})",
                self.indent_code(&self.convert_error_handling(&error_code), 8)
            ));
        }
        
        result
    }
    
    /// Split callback body into success and error handling code
    fn split_error_handling(&self, callback_body: &str) -> (String, String) {
        // Look for if (browser.runtime.lastError) or if (chrome.runtime.lastError)
        let error_pattern = regex::Regex::new(
            r"if\s*\(\s*(?:browser|chrome)\.runtime\.lastError\s*\)\s*\{([^}]*)\}\s*(?:else\s*\{([^}]*)\})?"
        ).unwrap();
        
        if let Some(cap) = error_pattern.captures(callback_body) {
            let error_code = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            let success_code = cap.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
            
            // If there's no else block, success code is everything after the if block
            let final_success = if success_code.is_empty() {
                // Remove the error check from the original body
                error_pattern.replace(callback_body, "").to_string()
            } else {
                success_code
            };
            
            return (final_success, error_code);
        }
        
        // No error handling found, return all as success code
        (callback_body.to_string(), String::new())
    }
    
    /// Convert error handling code to use error parameter instead of lastError
    fn convert_error_handling(&self, error_code: &str) -> String {
        let mut result = error_code.to_string();
        
        // Replace browser.runtime.lastError with error
        result = result.replace("browser.runtime.lastError.message", "error.message");
        result = result.replace("browser.runtime.lastError", "error");
        result = result.replace("chrome.runtime.lastError.message", "error.message");
        result = result.replace("chrome.runtime.lastError", "error");
        
        result
    }
    
    /// Transform nested callbacks by flattening them
    fn transform_nested_callback(&self, callback: &CallbackInfo) -> String {
        // For nested callbacks, try to flatten them into a promise chain
        // This is a simplified version - handles 2-3 levels
        
        // For now, just use simple transformation
        // A more sophisticated version would recursively parse and flatten
        self.transform_simple_callback(callback)
    }
    
    fn handle_import_scripts(&self, content: &str) -> (String, Vec<FileChange>) {
        let mut changes = Vec::new();
        let mut result = content.to_string();
        
        // Comment out importScripts() calls since scripts will be loaded via manifest
        let lines: Vec<&str> = content.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            if IMPORT_SCRIPTS.is_match(line) {
                let new_line = format!("// {} // Scripts loaded via manifest.json for Firefox compatibility", line.trim());
                result = result.replace(line, &new_line);
                
                changes.push(FileChange {
                    line_number: line_num + 1,
                    change_type: ChangeType::Modification,
                    description: "Commented out importScripts() - scripts loaded via manifest.json".to_string(),
                    old_code: Some(line.to_string()),
                    new_code: Some(new_line),
                });
            }
        }
        
        (result, changes)
    }
    
    fn convert_last_error_checks(&self, content: &str) -> (String, Vec<FileChange>) {
        let mut changes = Vec::new();
        let mut result = content.to_string();
        
        // Convert chrome.runtime.lastError to promise catch blocks
        let lines: Vec<&str> = content.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            if CHROME_RUNTIME_LASTERROR.is_match(line) {
                changes.push(FileChange {
                    line_number: line_num + 1,
                    change_type: ChangeType::Modification,
                    description: "chrome.runtime.lastError check - convert to promise .catch()".to_string(),
                    old_code: Some(line.to_string()),
                    new_code: None,
                });
            }
        }
        
        // Replace chrome.runtime.lastError with browser.runtime.lastError
        result = CHROME_RUNTIME_LASTERROR.replace_all(&result, "browser.runtime.lastError").to_string();
        
        (result, changes)
    }
    
    /// Replace Chrome settings URLs with Firefox equivalents
    fn replace_chrome_urls(&self, content: &str) -> (String, Vec<FileChange>) {
        let mut changes = Vec::new();
        let mut result = content.to_string();
        
        // Mapping of Chrome URLs to Firefox URLs
        let url_mappings = vec![
            ("extensions", "about:addons"),
            ("settings", "about:preferences"),
            ("history", "about:history"),
            ("downloads", "about:downloads"),
            ("bookmarks", "about:bookmarks"),
            ("flags", "about:config"),
            ("version", "about:support"),
            ("apps", "about:addons"),
            ("plugins", "about:addons"),
            ("privacy", "about:preferences#privacy"),
            ("passwords", "about:preferences#privacy"),
            ("autofill", "about:preferences#privacy"),
            ("appearance", "about:preferences#general"),
            ("search", "about:preferences#search"),
            ("sync", "about:preferences#sync"),
        ];
        
        let lines: Vec<&str> = content.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            for cap in CHROME_URL_PATTERN.captures_iter(line) {
                let full_match = cap.get(0).unwrap().as_str();
                let page = cap.get(1).unwrap().as_str();
                
                // Find matching Firefox URL
                if let Some((_, firefox_url)) = url_mappings.iter().find(|(chrome_page, _)| *chrome_page == page) {
                    let replacement = format!("\"{}\"", firefox_url);
                    result = result.replace(full_match, &replacement);
                    
                    changes.push(FileChange {
                        line_number: line_num + 1,
                        change_type: ChangeType::Modification,
                        description: format!("Replaced chrome://{} with {}", page, firefox_url),
                        old_code: Some(line.to_string()),
                        new_code: Some(line.replace(full_match, &replacement)),
                    });
                }
            }
        }
        
        (result, changes)
    }
    
    /// Extract executeScript calls from background.js for transformation
    fn extract_execute_script_calls(&self, content: &str) -> Vec<ExecuteScriptCall> {
        let mut calls = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            
            // Check if this line starts an executeScript call
            if EXECUTE_SCRIPT_PATTERN.is_match(line) {
                // Try to extract the full call including the function body
                let (call_info, end_line) = self.parse_execute_script_call(&lines, i);
                if let Some(info) = call_info {
                    calls.push(info);
                    i = end_line;
                    continue;
                }
            }
            
            i += 1;
        }
        
        calls
    }
    
    /// Parse a single executeScript call starting at the given line
    fn parse_execute_script_call(&self, lines: &[&str], start: usize) -> (Option<ExecuteScriptCall>, usize) {
        let mut full_text = String::new();
        let mut brace_count = 0;
        let mut in_function = false;
        let mut function_body = String::new();
        let mut tab_id_expr = String::new();
        let mut function_name: Option<String> = None;
        let mut function_params: Vec<String> = Vec::new(); // Track function parameters
        let mut has_callback = false;
        let mut callback_body = String::new();
        let mut callback_start_line = 0;
        let mut in_callback = false;
        let mut callback_brace_count = 0;
        
        let mut i = start;
        while i < lines.len() {
            let line = lines[i];
            full_text.push_str(line);
            full_text.push('\n');
            
            // Extract tabId from target: { tabId: ... }
            if line.contains("tabId:") && tab_id_expr.is_empty() {
                if let Some(start_pos) = line.find("tabId:") {
                    let after_colon = &line[start_pos + 6..];
                    // Extract until comma or closing brace
                    let tab_id = after_colon.trim()
                        .split(&[',', '}'][..])
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    tab_id_expr = tab_id;
                }
            }
            
            // Look for function: keyword
            if line.contains("function:") {
                in_function = true;
                // Check if it's a function reference
                let after_func = line.split("function:").nth(1).unwrap_or("").trim();
                if !after_func.starts_with("async") && !after_func.starts_with("(")
                    && !after_func.contains("=>") && !after_func.contains("function") {
                    // It's a function reference
                    let func_ref = after_func.split(',').next().unwrap_or("").trim();
                    if !func_ref.is_empty() {
                        function_name = Some(func_ref.to_string());
                    }
                } else {
                    // It's an inline function - extract parameters
                    // Look for (param1, param2) or async (param) patterns
                    if let Some(paren_start) = after_func.find('(') {
                        if let Some(paren_end) = after_func.find(')') {
                            let params_str = &after_func[paren_start + 1..paren_end];
                            function_params = params_str
                                .split(',')
                                .map(|p| p.trim().to_string())
                                .filter(|p| !p.is_empty())
                                .collect();
                        }
                    }
                }
            }
            
            // Collect function body
            if in_function && !in_callback {
                function_body.push_str(line);
                function_body.push('\n');
            }
            
            // Detect callback starting with }, (result) => {
            if line.contains("}, (") || line.contains("}, function") {
                has_callback = true;
                in_callback = true;
                callback_start_line = i;
                callback_brace_count = 0;
            }
            
            // Collect callback body
            if in_callback {
                callback_body.push_str(line);
                callback_body.push('\n');
                
                for ch in line.chars() {
                    match ch {
                        '{' => callback_brace_count += 1,
                        '}' => callback_brace_count -= 1,
                        _ => {}
                    }
                }
            }
            
            // Count braces to find the end
            for ch in line.chars() {
                match ch {
                    '{' => brace_count += 1,
                    '}' => {
                        brace_count -= 1;
                        // Don't exit if we just detected a callback on this line - need to parse callback body
                        if brace_count == 0 && i > start && !(in_callback && i == callback_start_line) {
                            // Found the end of executeScript call
                            let all_content = lines.join("\n"); // Search entire file, not just up to this point
                            let (extracted_body, looked_up_params) = if let Some(ref fname) = function_name {
                                // Function reference - look it up in the file
                                self.lookup_function_body(&all_content, fname)
                            } else {
                                // Inline function - extract from function_body
                                (self.extract_function_body(&function_body), function_params.clone())
                            };
                            
                            // If we looked up a function reference, use those params
                            let final_params = if function_name.is_some() {
                                looked_up_params
                            } else {
                                function_params.clone()
                            };
                            
                            // Extract args from the executeScript call
                            let args = self.extract_args(&full_text);
                            
                            // Extract background variables used in the function (excluding parameters and args)
                            let background_vars = self.find_background_variables_excluding_args(
                                &all_content,
                                &extracted_body,
                                &final_params,
                                &args
                            );
                            
                            // Calculate end line - if has callback, include callback end
                            let actual_end_line = if has_callback && callback_start_line > 0 {
                                // The callback ends at the current line i
                                i + 1
                            } else {
                                i + 1
                            };
                            
                            return (Some(ExecuteScriptCall {
                                start_line: start + 1,
                                end_line: actual_end_line,
                                tab_id_expr,
                                function_body: extracted_body,
                                function_name,
                                function_params: final_params, // Use the final parameters (either from inline or lookup)
                                args,
                                background_vars,
                                has_callback,
                                callback_body: if has_callback {
                                    self.extract_callback_body(&callback_body)
                                } else {
                                    String::new()
                                },
                                full_text: lines[start..=i].join("\n"), // Full text including callback
                            }), i + 1);
                        }
                    }
                    _ => {}
                }
            }
            
            i += 1;
        }
        
        (None, i)
    }
    
    /// Look up a function definition by name in the background.js content
    /// Returns (function_body, function_params)
    fn lookup_function_body(&self, content: &str, function_name: &str) -> (String, Vec<String>) {
        // Try different function definition patterns
        let patterns = vec![
            format!("function {}(", function_name),  // function name(
            format!("function  {}(", function_name), // function  name( (extra space)
        ];
        
        for pattern in patterns {
            if let Some(start) = content.find(&pattern) {
                let after_start = &content[start..];
                
                // Extract parameters first (between ( and ))
                let mut params = Vec::new();
                if let Some(paren_start) = after_start.find('(') {
                    if let Some(paren_end) = after_start.find(')') {
                        let params_str = &after_start[paren_start + 1..paren_end];
                        params = params_str
                            .split(',')
                            .map(|p| p.trim().to_string())
                            .filter(|p| !p.is_empty())
                            .collect();
                    }
                }
                
                // Find the opening brace
                if let Some(brace_pos) = after_start.find('{') {
                    let after_brace = &after_start[brace_pos + 1..];
                    // Extract until we find the matching closing brace
                    let mut brace_count = 1;
                    let mut result = String::new();
                    
                    for ch in after_brace.chars() {
                        if ch == '{' {
                            brace_count += 1;
                        } else if ch == '}' {
                            brace_count -= 1;
                            if brace_count == 0 {
                                return (result.trim().to_string(), params);
                            }
                        }
                        result.push(ch);
                    }
                }
            }
        }
        
        // If not found, return empty string and empty params
        // Log a warning that the function wasn't found
        eprintln!("Warning: Function '{}' not found in background.js", function_name);
        (String::new(), Vec::new())
    }
    
    /// Extract the actual function body from the function declaration
    fn extract_function_body(&self, func_text: &str) -> String {
        // Find the function body between the outermost { }
        let mut body = String::new();
        let mut brace_count = 0;
        let mut started = false;
        let mut chars = func_text.chars().peekable();
        
        while let Some(ch) = chars.next() {
            match ch {
                '{' => {
                    brace_count += 1;
                    if !started {
                        started = true;
                        continue; // Skip the opening brace
                    }
                    body.push(ch);
                }
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        break; // Skip the closing brace
                    }
                    body.push(ch);
                }
                _ => {
                    if started {
                        body.push(ch);
                    }
                }
            }
        }
        
        body.trim().to_string()
    }
    
    /// Extract callback body from }, (result) => { ... })
    fn extract_callback_body(&self, callback_text: &str) -> String {
        let mut body = String::new();
        let mut brace_count = 0;
        let mut started = false;
        
        for ch in callback_text.chars() {
            if ch == '{' {
                brace_count += 1;
                if !started {
                    started = true;
                    continue;
                }
            }
            
            if started {
                if ch == '}' {
                    brace_count -= 1;
                    if brace_count == 0 {
                        break;
                    }
                }
                body.push(ch);
            }
        }
        
        body.trim().to_string()
    }
    
    /// Find variables from background.js scope that are used in the function
    /// Excludes function parameters, function declarations, and args already passed
    fn find_background_variables_excluding_args(
        &self,
        background_content: &str,
        function_body: &str,
        function_params: &[String],
        args: &[String]
    ) -> Vec<String> {
        let mut vars = Vec::new();
        
        // Extract all variable declarations from background.js (NOT functions)
        let var_patterns = vec![
            Regex::new(r"\blet\s+(\w+)\s*=").unwrap(),
            Regex::new(r"\bconst\s+(\w+)\s*=").unwrap(),
            Regex::new(r"\bvar\s+(\w+)\s*=").unwrap(),
        ];
        
        // Get all declared variables in background.js
        let mut declared_vars: Vec<String> = Vec::new();
        for pattern in &var_patterns {
            for cap in pattern.captures_iter(background_content) {
                if let Some(var_name) = cap.get(1) {
                    declared_vars.push(var_name.as_str().to_string());
                }
            }
        }
        
        // Get variables declared INSIDE the function (to exclude)
        let mut local_vars: Vec<String> = Vec::new();
        for pattern in &var_patterns {
            for cap in pattern.captures_iter(function_body) {
                if let Some(var_name) = cap.get(1) {
                    local_vars.push(var_name.as_str().to_string());
                }
            }
        }
        
        // List of global objects to exclude
        let globals = vec![
            "browser", "chrome", "window", "document", "navigator",
            "console", "setTimeout", "setInterval", "clearTimeout", "clearInterval",
            "Promise", "Array", "Object", "String", "Number", "Boolean",
            "Date", "Math", "JSON", "Error", "TypeError", "SyntaxError",
            "fetch", "performance", "XMLHttpRequest", "FormData",
            "localStorage", "sessionStorage", "location", "history"
        ];
        
        // Check which background variables are used in the function body
        // but NOT declared locally, NOT global objects, NOT function parameters, NOT already in args
        for var in declared_vars {
            // Skip if it's a global object
            if globals.contains(&var.as_str()) {
                continue;
            }
            
            // Skip if it's declared locally in the function
            if local_vars.contains(&var) {
                continue;
            }
            
            // Skip if it's a function parameter
            if function_params.contains(&var) {
                continue;
            }
            
            // Skip if it's already in the args list
            if args.contains(&var) {
                continue;
            }
            
            // Look for variable usage (not just as substring of other identifiers)
            let usage_pattern = format!(r"\b{}\b", regex::escape(&var));
            if let Ok(re) = Regex::new(&usage_pattern) {
                if re.is_match(function_body) {
                    vars.push(var);
                }
            }
        }
        
        vars
    }
    
    /// Extract args array from executeScript call
    fn extract_args(&self, full_text: &str) -> Vec<String> {
        // Look for args: [...]
        if let Some(args_start) = full_text.find("args:") {
            let after_args = &full_text[args_start + 5..];
            if let Some(bracket_start) = after_args.find('[') {
                if let Some(bracket_end) = after_args.find(']') {
                    let args_str = &after_args[bracket_start + 1..bracket_end];
                    return args_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        }
        Vec::new()
    }
    
    /// Transform executeScript calls to message passing pattern
    fn transform_execute_script_to_messages(&self, content: &str) -> (String, Vec<FileChange>) {
        let mut changes = Vec::new();
        let mut result = content.to_string();
        
        // Process each executeScript call in reverse order (to maintain line numbers)
        for call in self.execute_script_calls.iter().rev() {
            // Generate a unique message type based on the function content
            let message_type = format!("EXECUTE_SCRIPT_REQUEST_{}", call.start_line);
            
            // Create the replacement code using sendMessage
            let replacement = self.generate_send_message_replacement(&call, &message_type);
            
            // Replace in the content
            result = result.replace(&call.full_text, &replacement);
            
            changes.push(FileChange {
                line_number: call.start_line,
                change_type: ChangeType::Modification,
                description: format!(
                    "Transformed executeScript to message passing (type: {})",
                    message_type
                ),
                old_code: Some(call.full_text.clone()),
                new_code: Some(replacement),
            });
        }
        
        (result, changes)
    }
    
    /// Generate sendMessage replacement code
    fn generate_send_message_replacement(&self, call: &ExecuteScriptCall, message_type: &str) -> String {
        // Combine args and background vars (args are already the executeScript args, bg vars are additional)
        let mut all_params = Vec::new();
        all_params.extend(call.args.clone());
        all_params.extend(call.background_vars.clone());
        
        let args_obj = if all_params.is_empty() {
            String::new()
        } else {
            format!(", args: [{}]", all_params.join(", "))
        };
        
        // Generate the sendMessage call
        let mut result = format!(
            r#"browser.tabs.sendMessage({}, {{
                type: '{}'{}
            }})"#,
            call.tab_id_expr,
            message_type,
            args_obj
        );
        
        // Don't include the original callback - it contains Chrome-specific patterns
        // like browser.runtime.lastError that don't work with promise-based message passing
        // Add .catch() for error handling
        result.push_str(r#".catch(error => {
                console.error('%c LatexToCalc [BG] › %cFailed to communicate with content script:',
                              'color:#2196F3;font-weight:bold', 'color:#F44336', error);
            });"#);
        
        result
    }
    
    /// Generate message listener code for content.js
    pub fn generate_content_script_listeners(&self) -> String {
        if self.execute_script_calls.is_empty() {
            return String::new();
        }
        
        let mut listeners = String::new();
        listeners.push_str("\n// Auto-generated message listeners for Firefox compatibility\n");
        listeners.push_str("// These replace executeScript injected functions\n\n");
        
        for call in &self.execute_script_calls {
            let message_type = format!("EXECUTE_SCRIPT_REQUEST_{}", call.start_line);
            
            // Use original function parameter names for extraction
            // This ensures variable names in the function body match the extraction
            let mut extraction_names = call.function_params.clone();
            
            // Add background variables after the function parameters
            extraction_names.extend(call.background_vars.clone());
            
            let args_extraction = if !extraction_names.is_empty() {
                format!("const [{}] = request.args;", extraction_names.join(", "))
            } else {
                String::new()
            };
            
            // Determine if function returns a value (for sendResponse)
            let needs_response = call.function_body.contains("return ")
                || call.function_body.contains("clipboard.writeText")
                || call.function_body.contains("sendMessage");
            
            let listener = if needs_response {
                // For functions that return values, use sendResponse
                format!(
                    r#"// Listener for executeScript from line {}
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {{
    if (request.type === '{}') {{
        (async () => {{
            {}
            try {{
                {}
                const result = await (async () => {{
                    {}
                }})();
                sendResponse(result);
            }} catch (error) {{
                console.error('Error in listener:', error);
                sendResponse({{ error: error.message }});
            }}
        }})();
        return true; // Keep message channel open for async response
    }}
}});

"#,
                    call.start_line,
                    message_type,
                    args_extraction,
                    if !args_extraction.is_empty() { "\n            " } else { "" },
                    self.indent_code(&call.function_body, 20)
                )
            } else {
                // For functions that don't return values
                format!(
                    r#"// Listener for executeScript from line {}
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {{
    if (request.type === '{}') {{
        (async () => {{
            {}
            {}
        }})();
        return true; // Keep message channel open for async response
    }}
}});

"#,
                    call.start_line,
                    message_type,
                    args_extraction,
                    if !args_extraction.is_empty() {
                        format!("\n            {}", self.indent_code(&call.function_body, 12))
                    } else {
                        self.indent_code(&call.function_body, 12)
                    }
                )
            };
            
            listeners.push_str(&listener);
        }
        
        listeners
    }
    
    /// Helper to indent code
    fn indent_code(&self, code: &str, spaces: usize) -> String {
        let indent = " ".repeat(spaces);
        code.lines()
            .map(|line| {
                if line.trim().is_empty() {
                    String::new()
                } else {
                    format!("{}{}", indent, line)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    /// Convert long setTimeout/setInterval to chrome.alarms
    fn convert_long_timers_to_alarms(&mut self, content: &str) -> (String, Vec<FileChange>) {
        let mut changes = Vec::new();
        let mut result = content.to_string();
        const LONG_DELAY_MS: u64 = 30000; // 30 seconds
        
        let mut timer_count = 0;
        
        // Convert setTimeout with delays > 30 seconds
        for cap in SETTIMEOUT_PATTERN.captures_iter(content) {
            if let (Some(callback), Some(delay_str)) = (cap.get(1), cap.get(2)) {
                if let Ok(delay) = delay_str.as_str().parse::<u64>() {
                    if delay > LONG_DELAY_MS {
                        timer_count += 1;
                        let alarm_name = format!("converted_timeout_{}", timer_count);
                        let callback_code = callback.as_str().trim().to_string();
                        
                        // Create alarm setup code
                        let alarm_code = format!(
                            "browser.alarms.create('{}', {{ delayInMinutes: {} }})",
                            alarm_name,
                            delay as f64 / 60000.0
                        );
                        
                        // Replace setTimeout with alarm creation
                        result = result.replace(cap.get(0).unwrap().as_str(), &alarm_code);
                        
                        self.converted_timers.push(TimerConversion {
                            alarm_name: alarm_name.clone(),
                            original_delay_ms: delay,
                            is_interval: false,
                            callback_code,
                        });
                        
                        changes.push(FileChange {
                            line_number: 1, // Approximate
                            change_type: ChangeType::Modification,
                            description: format!(
                                "Converted setTimeout({}ms) to browser.alarms (survives event page termination)",
                                delay
                            ),
                            old_code: Some(cap.get(0).unwrap().as_str().to_string()),
                            new_code: Some(alarm_code),
                        });
                    }
                }
            }
        }
        
        // Convert setInterval with delays > 30 seconds
        for cap in SETINTERVAL_PATTERN.captures_iter(content) {
            if let (Some(callback), Some(delay_str)) = (cap.get(1), cap.get(2)) {
                if let Ok(delay) = delay_str.as_str().parse::<u64>() {
                    if delay > LONG_DELAY_MS {
                        timer_count += 1;
                        let alarm_name = format!("converted_interval_{}", timer_count);
                        let callback_code = callback.as_str().trim().to_string();
                        
                        // Create recurring alarm setup code
                        let alarm_code = format!(
                            "browser.alarms.create('{}', {{ periodInMinutes: {} }})",
                            alarm_name,
                            delay as f64 / 60000.0
                        );
                        
                        // Replace setInterval with alarm creation
                        result = result.replace(cap.get(0).unwrap().as_str(), &alarm_code);
                        
                        self.converted_timers.push(TimerConversion {
                            alarm_name: alarm_name.clone(),
                            original_delay_ms: delay,
                            is_interval: true,
                            callback_code,
                        });
                        
                        changes.push(FileChange {
                            line_number: 1, // Approximate
                            change_type: ChangeType::Modification,
                            description: format!(
                                "Converted setInterval({}ms) to browser.alarms (survives event page termination)",
                                delay
                            ),
                            old_code: Some(cap.get(0).unwrap().as_str().to_string()),
                            new_code: Some(alarm_code),
                        });
                    }
                }
            }
        }
        
        // Add alarm listeners if any timers were converted
        if !self.converted_timers.is_empty() {
            let listener_code = self.generate_alarm_listeners();
            result = format!("{}\n\n{}", result, listener_code);
            
            changes.push(FileChange {
                line_number: 1,
                change_type: ChangeType::Addition,
                description: format!("Added browser.alarms listeners for {} converted timers", self.converted_timers.len()),
                old_code: None,
                new_code: Some("Alarm listeners for converted timers".to_string()),
            });
        }
        
        (result, changes)
    }
    
    /// Generate alarm listener code for converted timers
    fn generate_alarm_listeners(&self) -> String {
        let mut code = String::new();
        
        code.push_str("// === AUTO-GENERATED: Alarm Listeners for Converted Timers ===\n");
        code.push_str("// Long setTimeout/setInterval calls were converted to browser.alarms\n");
        code.push_str("// to survive event page termination.\n\n");
        
        code.push_str("browser.alarms.onAlarm.addListener((alarm) => {\n");
        
        for timer in &self.converted_timers {
            code.push_str(&format!("    if (alarm.name === '{}') {{\n", timer.alarm_name));
            code.push_str(&format!("        // Original: {}(callback, {})\n",
                if timer.is_interval { "setInterval" } else { "setTimeout" },
                timer.original_delay_ms
            ));
            code.push_str(&format!("        const callback = {};\n", timer.callback_code));
            code.push_str("        callback();\n");
            
            if !timer.is_interval {
                code.push_str(&format!("        browser.alarms.clear('{}'); // One-time alarm\n", timer.alarm_name));
            }
            
            code.push_str("    }\n");
        }
        
        code.push_str("});\n\n");
        code.push_str("// === END AUTO-GENERATED ===\n");
        
        code
    }
}

/// Advanced transformation using AST manipulation
/// Note: Full AST transformation can be added later with proper SWC version compatibility
pub struct AstTransformer {
    source: String,
}

impl AstTransformer {
    pub fn new(source: String) -> Self {
        Self { source }
    }
    
    /// Transform chrome.* calls to browser.* with proper promise handling
    pub fn transform_to_firefox(&self) -> Result<String> {
        // For now, use regex-based transformation
        // TODO: Implement full AST transformation when SWC versions are stable
        let mut result = self.source.clone();
        result = CHROME_NAMESPACE.replace_all(&result, "browser.").to_string();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chrome_to_browser_conversion() {
        let transformer = JavaScriptTransformer::new(&[]);
        let code = "chrome.storage.local.get('key');";
        let (result, _) = transformer.convert_chrome_to_browser(code);
        assert!(result.contains("browser.storage"));
    }
    
    #[test]
    fn test_polyfill_addition() {
        let transformer = JavaScriptTransformer::new(&[]);
        let code = "chrome.runtime.sendMessage({});";
        let result = transformer.add_browser_polyfill(code);
        assert!(result.contains("typeof browser === 'undefined'"));
    }
    
    #[test]
    fn test_chrome_url_replacement() {
        let transformer = JavaScriptTransformer::new(&[]);
        let code = r#"
            window.open('chrome://extensions/');
            const url = "chrome://settings/";
            fetch('chrome://history/');
        "#;
        let (result, _) = transformer.replace_chrome_urls(code);
        assert!(result.contains("about:addons"));
        assert!(result.contains("about:preferences"));
        assert!(result.contains("about:history"));
        assert!(!result.contains("chrome://"));
    }
}