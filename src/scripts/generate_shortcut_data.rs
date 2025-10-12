//! Script to generate keyboard shortcut data from live Firefox documentation
//! Run with: cargo run --features cli --bin generate-shortcuts

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::Result;

mod check_keyboard_shortcuts;
use check_keyboard_shortcuts::{fetch_firefox_devtools_shortcuts, fetch_firefox_support_shortcuts, FIREFOX_DEVTOOLS_SHORTCUTS_URL, FIREFOX_SUPPORT_SHORTCUTS_URLS};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Fetching latest Firefox keyboard shortcuts...");
    
    let client = reqwest::Client::builder()
        .user_agent("chrome-to-firefox (https://github.com/OtsoBear/chrome-to-firefox)")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    
    // Collect all shortcuts
    let mut all_shortcuts = Vec::new();
    
    // Fetch DevTools shortcuts
    println!("Fetching DevTools shortcuts...");
    let devtools = fetch_firefox_devtools_shortcuts(&client).await?;
    println!("  Found {} DevTools shortcuts", devtools.len());
    all_shortcuts.extend(devtools);
    
    // Fetch support page shortcuts
    for (platform, url) in FIREFOX_SUPPORT_SHORTCUTS_URLS {
        println!("Fetching {} shortcuts...", platform);
        let support = fetch_firefox_support_shortcuts(&client, url, platform).await?;
        println!("  Found {} {} shortcuts", support.len(), platform);
        all_shortcuts.extend(support);
    }
    
    // Deduplicate and organize
    let mut shortcuts_map: HashMap<String, String> = HashMap::new();
    for shortcut in all_shortcuts {
        shortcuts_map
            .entry(shortcut.normalized.clone())
            .or_insert_with(|| shortcut.description.clone());
    }
    
    println!("\nTotal unique shortcuts: {}", shortcuts_map.len());
    
    // Generate Rust code
    let mut code = String::from(r#"//! Auto-generated Firefox keyboard shortcuts database
//! Generated from live Firefox documentation
//! DO NOT EDIT MANUALLY - regenerate with: cargo run --bin generate-shortcuts

use std::collections::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref FIREFOX_SHORTCUTS: HashMap<String, String> = {
        let mut m = HashMap::new();
"#);
    
    // Sort keys for consistent output
    let mut keys: Vec<_> = shortcuts_map.keys().collect();
    keys.sort();
    
    for key in keys {
        let description = &shortcuts_map[key];
        // Escape quotes in description
        let escaped_desc = description.replace('\\', "\\\\").replace('"', "\\\"");
        code.push_str(&format!(
            "        m.insert(\"{}\".to_string(), \"{}\".to_string());\n",
            key, escaped_desc
        ));
    }
    
    code.push_str(r#"        m
    };
}
"#);
    
    // Write to file
    let output_path = Path::new("src/analyzer/firefox_shortcuts_data.rs");
    fs::write(output_path, code)?;
    
    println!("\n✅ Generated {} with {} shortcuts", 
        output_path.display(), shortcuts_map.len());
    println!("Remember to rebuild WASM after updating this file!");
    
    Ok(())
}