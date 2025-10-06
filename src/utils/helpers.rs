//! Helper utility functions

/// Check if a string is a URL match pattern
pub fn is_match_pattern(s: &str) -> bool {
    s.contains("://") || s.starts_with('<') || s.starts_with('*')
}

/// Sanitize extension name for use in filenames
pub fn sanitize_name(name: &str) -> String {
    name.to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

/// Generate a Firefox extension ID from a name
pub fn generate_extension_id(name: &str) -> String {
    format!("{}@converted.extension", sanitize_name(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_match_pattern() {
        assert!(is_match_pattern("https://example.com/*"));
        assert!(is_match_pattern("<all_urls>"));
        assert!(is_match_pattern("*://*.example.com/*"));
        assert!(!is_match_pattern("storage"));
        assert!(!is_match_pattern("tabs"));
    }
    
    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("My Extension"), "my-extension");
        assert_eq!(sanitize_name("Test@123"), "test123");
    }
    
    #[test]
    fn test_generate_extension_id() {
        assert_eq!(
            generate_extension_id("My Extension"),
            "my-extension@converted.extension"
        );
    }
}