//! Injects the runtime.onMessage compat shim (issue #8) as the first <script> of each
//! packaged extension HTML page, so listeners in offscreen documents / popups / options
//! pages are fixed too (background.scripts injection alone cannot reach them).

use std::path::Path;

const SHIM_REL: &str = "shims/runtime-onmessage-compat.js";

/// Returns the modified HTML with the compat shim inserted before the first `<script`, or
/// None when the page has no script tag (nothing to fix) or the shim is already present.
pub fn inject_onmessage_shim(html: &str, html_path: &Path) -> Option<String> {
    if html.contains(SHIM_REL) {
        return None; // already injected (idempotent)
    }
    let idx = find_first_script(html)?;
    let depth = html_path.parent().map(|p| p.components().count()).unwrap_or(0);
    let rel: String = "../".repeat(depth);
    let tag = format!("<script src=\"{}{}\"></script>", rel, SHIM_REL);
    let mut out = String::with_capacity(html.len() + tag.len());
    out.push_str(&html[..idx]);
    out.push_str(&tag);
    out.push_str(&html[idx..]);
    Some(out)
}

/// Case-insensitive search for the first `<script` opening tag.
fn find_first_script(html: &str) -> Option<usize> {
    let lower = html.to_ascii_lowercase();
    lower.find("<script")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_before_first_script_at_root() {
        let html = "<!DOCTYPE html><html><body><script src=\"offscreen.js\"></script></body></html>";
        let out = inject_onmessage_shim(html, std::path::Path::new("offscreen.html"));
        assert_eq!(
            out.as_deref(),
            Some("<!DOCTYPE html><html><body><script src=\"shims/runtime-onmessage-compat.js\"></script><script src=\"offscreen.js\"></script></body></html>")
        );
    }

    #[test]
    fn computes_relative_prefix_for_nested_html() {
        let html = "<html><head><script src=\"x.js\"></script></head></html>";
        let out = inject_onmessage_shim(html, std::path::Path::new("pages/popup.html")).unwrap();
        assert!(out.contains("<script src=\"../shims/runtime-onmessage-compat.js\"></script><script src=\"x.js\">"));
    }

    #[test]
    fn skips_html_without_scripts() {
        let html = "<html><body><p>no scripts here</p></body></html>";
        assert_eq!(inject_onmessage_shim(html, std::path::Path::new("about.html")), None);
    }

    #[test]
    fn is_idempotent() {
        let html = "<html><body><script src=\"shims/runtime-onmessage-compat.js\"></script><script src=\"a.js\"></script></body></html>";
        assert_eq!(inject_onmessage_shim(html, std::path::Path::new("a.html")), None);
    }
}
