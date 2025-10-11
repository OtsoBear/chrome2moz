//! Simple regex-based URL replacer for chrome:// → Firefox equivalents
//! Works on any text file (JS, HTML, CSS, JSON, etc.)

use regex::Regex;
use std::sync::OnceLock;

/// Chrome URL → Firefox URL mappings
/// Note: Most about: pages cannot be opened programmatically in Firefox due to security restrictions.
/// These mappings are primarily for HTML links (href) and user-visible text, not browser.tabs.create() calls.
static URL_MAPPINGS: &[(&str, &str)] = &[
    // Extensions management
    ("chrome://extensions/shortcuts", "about:addons"),
    ("chrome://extensions/", "about:addons"),
    ("chrome://extensions", "about:addons"),
    
    // Settings and preferences
    ("chrome://settings/", "about:preferences"),
    ("chrome://settings", "about:preferences"),
    
    // Other common pages
    ("chrome://history", "about:history"),
    ("chrome://downloads", "about:downloads"),
    ("chrome://bookmarks", "about:bookmarks"),
    ("chrome://newtab", "about:newtab"),
    ("chrome://flags", "about:config"),
];

/// Cached regex for finding chrome:// URLs
static CHROME_URL_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_chrome_url_regex() -> &'static Regex {
    CHROME_URL_REGEX.get_or_init(|| {
        Regex::new(r#"chrome://[a-z0-9/_-]+"#).unwrap()
    })
}

/// Replace all chrome:// URLs with their Firefox equivalents in the given text
pub fn replace_chrome_urls(content: &str) -> String {
    let regex = get_chrome_url_regex();
    
    regex.replace_all(content, |caps: &regex::Captures| {
        let url = caps.get(0).unwrap().as_str();
        
        // Find the best matching replacement
        // Sort by longest match first to handle "chrome://extensions/shortcuts" before "chrome://extensions"
        for (chrome_url, firefox_url) in URL_MAPPINGS {
            if url.starts_with(chrome_url) {
                return firefox_url.to_string();
            }
        }
        
        // No mapping found, return original
        url.to_string()
    }).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_replace_chrome_extensions() {
        let input = r#"window.open("chrome://extensions")"#;
        let output = replace_chrome_urls(input);
        assert!(output.contains("about:addons"));
        assert!(!output.contains("chrome://extensions"));
    }
    
    #[test]
    fn test_replace_chrome_extensions_shortcuts() {
        let input = r#"const url = "chrome://extensions/shortcuts";"#;
        let output = replace_chrome_urls(input);
        assert!(output.contains("about:addons"));
        assert!(!output.contains("chrome://extensions"));
    }
    
    #[test]
    fn test_replace_chrome_settings() {
        let input = r#"browser.tabs.create({ url: "chrome://settings" });"#;
        let output = replace_chrome_urls(input);
        assert!(output.contains("about:preferences"));
        assert!(!output.contains("chrome://settings"));
    }
    
    #[test]
    fn test_replace_multiple_urls() {
        let input = r#"
            <a href="chrome://extensions">Extensions</a>
            <a href="chrome://settings">Settings</a>
            <a href="chrome://history">History</a>
        "#;
        let output = replace_chrome_urls(input);
        assert!(output.contains("about:addons"));
        assert!(output.contains("about:preferences"));
        assert!(output.contains("about:history"));
        assert!(!output.contains("chrome://"));
    }
    
    #[test]
    fn test_no_replacement_for_unknown_urls() {
        let input = r#"chrome://unknown-page"#;
        let output = replace_chrome_urls(input);
        // Unknown URLs are kept as-is
        assert_eq!(output, input);
    }
    
    #[test]
    fn test_handles_html() {
        let input = r#"<a href="chrome://extensions/shortcuts">Configure shortcuts</a>"#;
        let output = replace_chrome_urls(input);
        assert!(output.contains("about:addons"));
    }
    
    #[test]
    fn test_handles_javascript() {
        let input = r#"
            const url1 = "chrome://extensions/shortcuts";
            const url2 = 'chrome://settings';
            browser.tabs.create({ url: `chrome://history` });
        "#;
        let output = replace_chrome_urls(input);
        assert!(output.contains("about:addons"));
        assert!(output.contains("about:preferences"));
        assert!(output.contains("about:history"));
    }
}