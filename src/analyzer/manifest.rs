//! Manifest analysis for incompatibilities

use crate::models::{
    Manifest, Incompatibility, Severity, IncompatibilityCategory, Location,
    WebAccessibleResources, ContentSecurityPolicy,
};

pub fn analyze_manifest(manifest: &Manifest) -> Vec<Incompatibility> {
    let mut issues = Vec::new();
    
    // Check manifest version
    if manifest.manifest_version != 3 {
        issues.push(
            Incompatibility::new(
                Severity::Blocker,
                IncompatibilityCategory::ManifestStructure,
                Location::ManifestField("manifest_version".to_string()),
                format!("Only Manifest V3 is supported. Found version {}", manifest.manifest_version)
            )
        );
        return issues;
    }
    
    // Check for browser_specific_settings
    if manifest.browser_specific_settings.is_none() {
        issues.push(
            Incompatibility::new(
                Severity::Major,
                IncompatibilityCategory::MissingFirefoxId,
                Location::ManifestField("browser_specific_settings".to_string()),
                "Firefox requires browser_specific_settings.gecko.id for submission"
            )
            .with_suggestion("Add a unique extension ID in email format")
            .auto_fixable()
        );
    }
    
    // Check background configuration
    if let Some(background) = &manifest.background {
        if background.service_worker.is_some() && background.scripts.is_none() {
            issues.push(
                Incompatibility::new(
                    Severity::Major,
                    IncompatibilityCategory::BackgroundWorker,
                    Location::ManifestField("background".to_string()),
                    "Service worker detected. Firefox MV3 uses event pages (background.scripts)"
                )
                .with_suggestion("Add background.scripts with same file for Firefox compatibility")
                .auto_fixable()
            );
        }
    }
    
    // Check host_permissions
    let has_host_patterns_in_permissions = manifest.permissions.iter()
        .any(|p| is_match_pattern(p));
    
    if has_host_patterns_in_permissions {
        issues.push(
            Incompatibility::new(
                Severity::Minor,
                IncompatibilityCategory::HostPermissions,
                Location::ManifestField("permissions".to_string()),
                "Match patterns found in permissions should be in host_permissions for MV3"
            )
            .with_suggestion("Move match patterns from permissions to host_permissions")
            .auto_fixable()
        );
    }
    
    // Check web_accessible_resources
    if let Some(WebAccessibleResources::V3(resources)) = &manifest.web_accessible_resources {
        for resource in resources {
            if resource.use_dynamic_url == Some(true) {
                issues.push(
                    Incompatibility::new(
                        Severity::Minor,
                        IncompatibilityCategory::WebAccessibleResources,
                        Location::ManifestField("web_accessible_resources".to_string()),
                        "use_dynamic_url is not supported in Firefox"
                    )
                    .with_suggestion("Remove use_dynamic_url and ensure matches or extension_ids are specified")
                    .auto_fixable()
                );
            }
        }
    }
    
    // Check CSP format
    if let Some(ContentSecurityPolicy::V2(_)) = &manifest.content_security_policy {
        issues.push(
            Incompatibility::new(
                Severity::Minor,
                IncompatibilityCategory::ContentSecurityPolicy,
                Location::ManifestField("content_security_policy".to_string()),
                "CSP must use object format in MV3"
            )
            .with_suggestion("Convert to { extension_pages: '...' } format")
            .auto_fixable()
        );
    }
    
    // Check for browser_style
    if let Some(action) = &manifest.action {
        if action.browser_style == Some(true) {
            issues.push(
                Incompatibility::new(
                    Severity::Minor,
                    IncompatibilityCategory::BrowserStyle,
                    Location::ManifestField("action.browser_style".to_string()),
                    "browser_style is not supported in MV3"
                )
                .with_suggestion("Remove browser_style property")
                .auto_fixable()
            );
        }
    }
    
    // Check browser_action (MV2 legacy)
    if manifest.browser_action.is_some() {
        issues.push(
            Incompatibility::new(
                Severity::Minor,
                IncompatibilityCategory::ManifestStructure,
                Location::ManifestField("browser_action".to_string()),
                "browser_action should be renamed to action in MV3"
            )
            .with_suggestion("Rename browser_action to action")
            .auto_fixable()
        );
    }
    
    issues
}

fn is_match_pattern(s: &str) -> bool {
    s.contains("://") || s.starts_with('<') || s.starts_with('*')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Background;
    
    #[test]
    fn test_detect_service_worker() {
        let manifest = Manifest {
            manifest_version: 3,
            name: "Test".to_string(),
            version: "1.0".to_string(),
            description: None,
            background: Some(Background {
                service_worker: Some("background.js".to_string()),
                scripts: None,
                persistent: None,
                type_: None,
            }),
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
        
        let issues = analyze_manifest(&manifest);
        assert!(issues.iter().any(|i| matches!(i.category, IncompatibilityCategory::BackgroundWorker)));
    }
}