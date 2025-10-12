//! Manifest transformation for Firefox compatibility

use crate::models::{
    Manifest, BrowserSpecificSettings, GeckoSettings,
    ContentSecurityPolicy, ContentSecurityPolicyV3, WebAccessibleResources,
    SelectedDecision, Extension,
};
use anyhow::Result;
use regex::Regex;

pub struct ManifestTransformer {
    _decisions: Vec<SelectedDecision>,
}

impl ManifestTransformer {
    pub fn new(decisions: &[SelectedDecision]) -> Self {
        Self {
            _decisions: decisions.to_vec(),
        }
    }
    
    pub fn transform(&self, manifest: &Manifest, source: Option<&Extension>) -> Result<Manifest> {
        let mut result = manifest.clone();
        
        // 1. Add Firefox-specific settings
        self.add_firefox_settings(&mut result);
        
        // 2. Transform background configuration
        self.transform_background(&mut result, source);
        
        // 3. Fix permissions structure
        self.transform_permissions(&mut result);
        
        // 4. Fix web_accessible_resources
        self.transform_web_accessible_resources(&mut result);
        
        // 5. Fix CSP format
        self.transform_csp(&mut result);
        
        // 6. Fix action/browser_action
        self.transform_action(&mut result);
        
        // 7. Fix content scripts for iframe support
        self.fix_content_scripts(&mut result);
        
        // 8. Remove Chrome-specific fields
        self.remove_chrome_specific_fields(&mut result);
        
        Ok(result)
    }
    
    fn add_firefox_settings(&self, manifest: &mut Manifest) {
        if manifest.browser_specific_settings.is_none() {
            // Generate Firefox-compliant email-style ID
            // Pattern: [a-zA-Z0-9-._]*@[a-zA-Z0-9-._]+
            let sanitized_name = Self::sanitize_extension_name(&manifest.name);
            let extension_id = format!("{}@converted-extension.org", sanitized_name);
            
            manifest.browser_specific_settings = Some(BrowserSpecificSettings {
                gecko: Some(GeckoSettings {
                    id: extension_id,
                    strict_min_version: Some("121.0".to_string()),
                    strict_max_version: None,
                }),
            });
        }
    }
    
    /// Sanitize extension name to be valid in Firefox email-style IDs
    /// Only allows: a-z, A-Z, 0-9, hyphen, dot, underscore
    fn sanitize_extension_name(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '.' || c == '_' {
                    c.to_lowercase().to_string()
                } else if c.is_whitespace() {
                    "-".to_string()
                } else {
                    // Remove other special characters
                    String::new()
                }
            })
            .collect::<String>()
            .trim_matches('-') // Remove leading/trailing hyphens
            .trim_matches('.') // Remove leading/trailing dots
            .trim_matches('_') // Remove leading/trailing underscores
            .to_string()
    }
    
    fn transform_background(&self, manifest: &mut Manifest, source: Option<&Extension>) {
        if let Some(background) = &mut manifest.background {
            // Build the scripts array with shims FIRST, then original scripts
            let mut scripts = vec![];
            
            // CRITICAL: Add all shims BEFORE the background scripts (no importScripts polyfill needed!)
            scripts.push("shims/storage-session-compat.js".to_string());
            scripts.push("shims/execute-script-compat.js".to_string());
            scripts.push("shims/sidepanel-compat.js".to_string());
            scripts.push("shims/declarative-net-request-stub.js".to_string());
            scripts.push("shims/user-scripts-compat.js".to_string());
            scripts.push("shims/tabs-windows-compat.js".to_string());
            scripts.push("shims/runtime-compat.js".to_string());
            scripts.push("shims/downloads-compat.js".to_string());
            scripts.push("shims/privacy-stub.js".to_string());
            scripts.push("shims/notifications-compat.js".to_string());
            
            // Add original background scripts (and extract importScripts)
            if let Some(existing_scripts) = &background.scripts {
                for script in existing_scripts {
                    // Try to extract importScripts() calls from this script
                    if let Some(imported) = Self::extract_imported_scripts(script, source) {
                        // Add imported scripts before the main script
                        scripts.extend(imported);
                    }
                    scripts.push(script.clone());
                }
            } else if let Some(sw) = &background.service_worker {
                // Try to extract importScripts() calls from service worker
                if let Some(imported) = Self::extract_imported_scripts(sw, source) {
                    scripts.extend(imported);
                }
                scripts.push(sw.clone());
            }
            
            background.scripts = Some(scripts);
            
            // IMPORTANT: Remove service_worker for Firefox (not supported)
            background.service_worker = None;
            
            // IMPORTANT: Remove persistent property for Firefox MV3 (not supported)
            background.persistent = None;
            
            // IMPORTANT: Remove type field (not supported in Firefox yet)
            background.type_ = None;
        }
    }
    
    /// Extract script names from importScripts() calls using regex
    /// This is SAFE - no eval() needed! We parse the calls and add scripts to manifest.
    /// Handles both commented and uncommented importScripts() calls.
    fn extract_imported_scripts(script_path: &str, source: Option<&Extension>) -> Option<Vec<String>> {
        // Read the script file content
        let content = source?.get_file_content(&std::path::PathBuf::from(script_path))?;
        
        // Match importScripts() calls, including commented out ones
        // Pattern: optional // comment, then importScripts(...)
        let re = Regex::new(r#"(?://\s*)?importScripts\s*\([^)]*\)"#).ok()?;
        
        let mut imported = Vec::new();
        
        // Find all importScripts() calls (commented or not)
        for call_match in re.find_iter(&content) {
            let call = call_match.as_str();
            
            // Extract each quoted string (file name) from the call
            let file_re = Regex::new(r#"['"]([^'"]+)['"]"#).ok()?;
            for file_cap in file_re.captures_iter(call) {
                if let Some(file) = file_cap.get(1) {
                    let filename = file.as_str().to_string();
                    if !imported.contains(&filename) {
                        imported.push(filename);
                    }
                }
            }
        }
        
        if !imported.is_empty() {
            Some(imported)
        } else {
            None
        }
    }
    
    fn transform_permissions(&self, manifest: &mut Manifest) {
        // Remove invalid permissions for Firefox
        let invalid_permissions = vec![
            "commands",   // "commands" is not a permission, it's a manifest key
            "offscreen",  // Chrome-only permission for offscreen documents
        ];
        
        // Separate API permissions from host permissions
        let permissions = manifest.permissions.clone();
        let (api_perms, host_perms): (Vec<_>, Vec<_>) = permissions
            .iter()
            .filter(|p| !invalid_permissions.contains(&p.as_str()))
            .partition(|p| !is_match_pattern(p));
        
        manifest.permissions = api_perms.into_iter().cloned().collect();
        
        // Merge with existing host_permissions
        let mut all_host_perms = host_perms.into_iter().cloned().collect::<Vec<_>>();
        all_host_perms.extend(manifest.host_permissions.iter().cloned());
        manifest.host_permissions = all_host_perms;
    }
    
    fn transform_web_accessible_resources(&self, manifest: &mut Manifest) {
        if let Some(WebAccessibleResources::V3(resources)) = &mut manifest.web_accessible_resources {
            for resource in resources {
                // Remove use_dynamic_url (not supported in Firefox)
                resource.use_dynamic_url = None;
                
                // Ensure matches or extension_ids are present
                if resource.matches.is_none() && resource.extension_ids.is_none() {
                    resource.matches = Some(vec!["<all_urls>".to_string()]);
                }
            }
        }
    }
    
    fn transform_csp(&self, manifest: &mut Manifest) {
        // Convert V2 CSP to V3 format
        if let Some(ContentSecurityPolicy::V2(csp_string)) = &manifest.content_security_policy {
            manifest.content_security_policy = Some(ContentSecurityPolicy::V3(
                ContentSecurityPolicyV3 {
                    extension_pages: Some(csp_string.clone()),
                    sandbox: None,
                }
            ));
        }
        
        // Add wasm-unsafe-eval if needed (check if extension uses WebAssembly)
        if let Some(ContentSecurityPolicy::V3(csp)) = &mut manifest.content_security_policy {
            if let Some(pages) = &mut csp.extension_pages {
                if !pages.contains("'wasm-unsafe-eval'") {
                    // Add wasm-unsafe-eval to script-src
                    if pages.contains("script-src") {
                        *pages = pages.replace("script-src", "script-src 'wasm-unsafe-eval'");
                    }
                }
            }
        }
        
        // NOTE: We don't add 'unsafe-eval' - it's not needed and reduces security
        // Instead, we detect importScripts() calls and add those scripts to the manifest
    }
    
    fn transform_action(&self, manifest: &mut Manifest) {
        // Rename browser_action to action
        if manifest.browser_action.is_some() && manifest.action.is_none() {
            manifest.action = manifest.browser_action.clone();
            manifest.browser_action = None;
        }
        
        // Remove browser_style (not supported in MV3)
        if let Some(action) = &mut manifest.action {
            action.browser_style = None;
        }
    }
    
    fn fix_content_scripts(&self, manifest: &mut Manifest) {
        // Enable all_frames for content scripts to work in iframes
        for content_script in &mut manifest.content_scripts {
            // If all_frames is false, enable it to support iframe content
            if !content_script.all_frames {
                content_script.all_frames = true;
            }
        }
    }
    
    fn remove_chrome_specific_fields(&self, manifest: &mut Manifest) {
        // Remove Chrome-specific fields that Firefox doesn't support
        let chrome_only_fields = vec![
            "key",           // Chrome's public key for extension signing
            "update_url",    // Chrome's auto-update URL
            "minimum_chrome_version",  // Chrome version requirement
            "oauth2",        // Chrome's OAuth2 configuration
            "export",        // Chrome's export configuration
        ];
        
        for field in chrome_only_fields {
            manifest.extra.remove(field);
        }
    }
    
    fn _get_decision_value(&self, decision_id: &str) -> Option<String> {
        self._decisions
            .iter()
            .find(|d| d.decision_id == decision_id)
            .map(|d| format!("option_{}", d.selected_index))
    }
}

fn is_match_pattern(s: &str) -> bool {
    s.contains("://") || s.starts_with('<') || s.starts_with('*')
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add_firefox_settings() {
        let mut manifest = Manifest {
            manifest_version: 3,
            name: "Test Extension".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            background: None,
            action: None,
            browser_action: None,
            permissions: vec![],
            host_permissions: vec![],
            content_scripts: vec![],
            web_accessible_resources: None,
            content_security_policy: None,
            browser_specific_settings: None,
            icons: None,
            commands: None,
            extra: Default::default(),
        };
        
        let transformer = ManifestTransformer::new(&[]);
        transformer.add_firefox_settings(&mut manifest);
        
        assert!(manifest.browser_specific_settings.is_some());
        let gecko = manifest.browser_specific_settings.unwrap().gecko.unwrap();
        assert!(gecko.id.contains("test-extension"));
        assert!(gecko.id.contains("@converted-extension.org"));
        // Verify it matches Firefox's email-style pattern
        assert!(gecko.id.ends_with("@converted-extension.org"));
    }
    
    #[test]
    fn test_sanitize_extension_name() {
        // Test simple case
        assert_eq!(
            ManifestTransformer::sanitize_extension_name("My Extension"),
            "my-extension"
        );
        
        // Test with special characters
        assert_eq!(
            ManifestTransformer::sanitize_extension_name("My@Extension#2024!"),
            "myextension2024"
        );
        
        // Test with dots and underscores
        assert_eq!(
            ManifestTransformer::sanitize_extension_name("my.extension_v2"),
            "my.extension_v2"
        );
        
        // Test with leading/trailing invalid chars
        assert_eq!(
            ManifestTransformer::sanitize_extension_name("-test-"),
            "test"
        );
    }
}