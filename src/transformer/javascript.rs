//! JavaScript code transformation for Chrome to Firefox conversion

use crate::models::{ModifiedFile, FileChange, ChangeType, SelectedDecision};
use anyhow::Result;
use regex::Regex;
use lazy_static::lazy_static;
use std::path::PathBuf;

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

pub struct JavaScriptTransformer {
    decisions: Vec<SelectedDecision>,
    execute_script_calls: Vec<ExecuteScriptCall>,
}

impl JavaScriptTransformer {
    pub fn new(decisions: &[SelectedDecision]) -> Self {
        Self {
            decisions: decisions.to_vec(),
            execute_script_calls: Vec::new(),
        }
    }
    
    /// Transform JavaScript code from Chrome to Firefox compatibility
    pub fn transform(&mut self, content: &str, path: &PathBuf) -> Result<ModifiedFile> {
        let mut new_content = content.to_string();
        let mut changes = Vec::new();
        
        // 1. Add browser polyfill at the top if chrome APIs are used
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
        
        // 2. Convert chrome.* to browser.*
        let (transformed, chrome_changes) = self.convert_chrome_to_browser(&new_content);
        new_content = transformed;
        changes.extend(chrome_changes);
        
        // 3. Detect and extract executeScript calls AFTER chrome->browser conversion
        if path.to_string_lossy().contains("background") {
            self.execute_script_calls = self.extract_execute_script_calls(&new_content);
        }
        
        // 4. Transform executeScript to message passing (background.js)
        if path.to_string_lossy().contains("background") && !self.execute_script_calls.is_empty() {
            let (transformed, exec_changes) = self.transform_execute_script_to_messages(&new_content);
            new_content = transformed;
            changes.extend(exec_changes);
        }
        
        // 5. Convert callback-style to promise-style
        let (transformed, callback_changes) = self.convert_callbacks_to_promises(&new_content);
        new_content = transformed;
        changes.extend(callback_changes);
        
        // 6. Handle importScripts in service workers
        if path.to_string_lossy().contains("background") {
            let (transformed, import_changes) = self.handle_import_scripts(&new_content);
            new_content = transformed;
            changes.extend(import_changes);
        }
        
        // 7. Convert chrome.runtime.lastError checks
        let (transformed, error_changes) = self.convert_last_error_checks(&new_content);
        new_content = transformed;
        changes.extend(error_changes);
        
        // 8. Replace Chrome settings URLs with Firefox equivalents
        let (transformed, url_changes) = self.replace_chrome_urls(&new_content);
        new_content = transformed;
        changes.extend(url_changes);
        
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
        let result = content.to_string();
        
        // This is a simplified version - full implementation would use AST transformation
        // For now, we'll add comments suggesting manual conversion
        
        let lines: Vec<&str> = content.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            if line.contains("function(") && (line.contains("browser.") || line.contains("chrome.")) {
                // Detect potential callback pattern
                if line.contains("browser.storage") || line.contains("browser.tabs") {
                    changes.push(FileChange {
                        line_number: line_num + 1,
                        change_type: ChangeType::Modification,
                        description: "Callback detected - consider converting to promise".to_string(),
                        old_code: Some(line.to_string()),
                        new_code: None,
                    });
                }
            }
        }
        
        (result, changes)
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