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
                "Missing Firefox extension ID (required for AMO submission)"
            )
            .with_suggestion("Will auto-generate email-style ID: {name}@converted-extension.org")
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
                    "Service worker detected (Chrome). Firefox uses event pages with background.scripts"
                )
                .with_suggestion("Will convert to event page + detect importScripts() calls + include 10 runtime shims")
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
                "Host patterns in 'permissions' (should be separate in Firefox MV3)"
            )
            .with_suggestion("Will move host patterns to 'host_permissions' array automatically")
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
                        "use_dynamic_url not supported in Firefox (Chrome-only feature)"
                    )
                    .with_suggestion("Will remove use_dynamic_url and ensure matches/extension_ids are specified")
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
                "CSP using MV2 string format (MV3 requires object format)"
            )
            .with_suggestion("Will convert to MV3 format: { extension_pages: '...' }")
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
                    "browser_style property (deprecated in MV3)"
                )
                .with_suggestion("Will remove browser_style property automatically")
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
                "browser_action (MV2 API, renamed in MV3)"
            )
            .with_suggestion("Will rename to 'action' for MV3 compatibility")
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