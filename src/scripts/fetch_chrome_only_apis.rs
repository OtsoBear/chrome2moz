use std::collections::HashSet;
use std::time::Duration;

use anyhow::{Context, Result};
use futures::{stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::parser::javascript::CHROME_ONLY_APIS;

const REPO_OWNER: &str = "mdn";
const REPO_NAME: &str = "browser-compat-data";
const BRANCH: &str = "main";
const API_PATH: &str = "webextensions/api";

const GITHUB_API_BASE: &str = "https://api.github.com/repos";
const GITHUB_RAW_BASE: &str = "https://raw.githubusercontent.com";

#[derive(Debug)]
struct ChromeOnlyApi {
    feature_path: String,
    source_file: String,
    chrome_info: Value,
    firefox_info: Value,
}

#[derive(Debug, Deserialize)]
struct ContentItem {
    name: String,
}

pub async fn run() -> Result<()> {
    let client = Client::builder()
        .user_agent("chrome-to-firefox (https://github.com/OtsoBear/chrome-to-firefox)")
        .timeout(Duration::from_secs(30))
        .build()
        .context("failed to build HTTP client")?;

    eprintln!("Fetching API file list from GitHub...");
    let api_files = list_api_files(&client).await?;

    if api_files.is_empty() {
        eprintln!("No API files found.");
        return Ok(());
    }

    eprintln!("Found {} API files. Processing...", api_files.len());
    let results = process_api_files(&client, &api_files).await?;

    if results.is_empty() {
        println!("\nNo APIs found that are supported in Chrome but not in Firefox.");
        return Ok(());
    }

    println!("\n{}", "=".repeat(80));
    println!("WebExtension APIs supported in Chrome but not Firefox:");
    println!("{}\n", "=".repeat(80));

    let mut sorted_results = results;
    sorted_results.sort_by(|a, b| a.feature_path.to_lowercase().cmp(&b.feature_path.to_lowercase()));

    let mut implemented_count = 0usize;
    let mut not_implemented_count = 0usize;
    let mut matched_prefixes: HashSet<&str> = HashSet::new();

    for entry in &sorted_results {
        let chrome_path = format!("chrome.{}", entry.feature_path);
        let status = if let Some(prefix) = matches_known_chrome_only(&chrome_path) {
            matched_prefixes.insert(prefix);
            implemented_count += 1;
            "[implemented]"
        } else {
            not_implemented_count += 1;
            "[not implemented]"
        };

        println!("- {} {}", entry.feature_path, status);
        println!("    Source: {}", entry.source_file);
        println!("    Chrome: {}", format_version(&entry.chrome_info));
        println!("    Firefox: {}\n", format_version(&entry.firefox_info));
    }

    println!("Summary:");
    println!("  Total Chrome-only APIs found: {}", sorted_results.len());
    println!("  Implemented (matches parser/javascript.rs): {}", implemented_count);
    println!("  Not yet implemented: {}", not_implemented_count);

    let missing_prefixes: Vec<&str> = CHROME_ONLY_APIS
        .iter()
        .copied()
        .filter(|prefix| !matched_prefixes.contains(prefix))
        .collect();

    println!(
        "  Known chrome-only prefixes tracked: {}",
        CHROME_ONLY_APIS.len()
    );
    println!(
        "  Known prefixes missing from MDN dataset: {}",
        missing_prefixes.len()
    );

    if !missing_prefixes.is_empty() {
        println!("    Missing prefixes:");
        for prefix in missing_prefixes {
            println!("      - {}", prefix);
        }
    }

    println!("");
    println!("Use this summary to prioritize new compatibility shims.");

    Ok(())
}

async fn list_api_files(client: &Client) -> Result<Vec<String>> {
    let url = format!(
        "{}/{}/{}/contents/{}?ref={}",
        GITHUB_API_BASE, REPO_OWNER, REPO_NAME, API_PATH, BRANCH
    );

    let response = client
        .get(url)
        .send()
        .await
        .context("failed to request API file list")?
        .error_for_status()
        .context("GitHub API returned an error for contents list")?;

    let items: Vec<ContentItem> = response
        .json()
        .await
        .context("failed to parse contents list")?;

    let files = items
        .into_iter()
        .filter(|item| item.name.ends_with(".json"))
        .map(|item| item.name)
        .collect();

    Ok(files)
}

async fn fetch_api_file(client: &Client, filename: &str) -> Result<Value> {
    let url = format!(
        "{}/{}/{}/{}/{}/{}",
        GITHUB_RAW_BASE, REPO_OWNER, REPO_NAME, BRANCH, API_PATH, filename
    );

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to download {filename}"))?
        .error_for_status()
        .with_context(|| format!("GitHub returned an error for {filename}"))?;

    response
        .json()
        .await
        .with_context(|| format!("failed to parse JSON for {filename}"))
}

async fn process_api_files(client: &Client, api_files: &[String]) -> Result<Vec<ChromeOnlyApi>> {
    let total = api_files.len();
    eprintln!("Fetching {} files concurrently...", total);

    let mut results = Vec::new();
    let mut processed = 0usize;

    let mut stream = stream::iter(api_files.iter().cloned())
        .map(|filename| {
            let client = client.clone();
            async move {
                let data = fetch_api_file(&client, &filename).await;
                (filename, data)
            }
        })
        .buffer_unordered(32);

    while let Some((filename, data)) = stream.next().await {
        match data {
            Ok(value) => {
                processed += 1;
                if processed % 10 == 0 || processed == total {
                    eprintln!("Processed {processed}/{total} files...");
                }
                collect_chrome_only_apis(&filename, &value, &mut results);
            }
            Err(err) => {
                eprintln!("Error fetching {filename}: {err:?}");
            }
        }
    }

    eprintln!("Completed processing all {processed} files");
    Ok(results)
}

fn collect_chrome_only_apis(filename: &str, data: &Value, results: &mut Vec<ChromeOnlyApi>) {
    let api_section = data
        .get("webextensions")
        .and_then(|v| v.get("api"))
        .and_then(Value::as_object);

    let Some(api_section) = api_section else {
        return;
    };

    for (api_name, api_data) in api_section {
        if !api_data.is_object() {
            continue;
        }

        let mut path = vec![api_name.clone()];
        walk_support_entries(&mut path, api_data, filename, results);
    }
}

fn walk_support_entries(
    path: &mut Vec<String>,
    node: &Value,
    filename: &str,
    results: &mut Vec<ChromeOnlyApi>,
) {
    if let Value::Object(map) = node {
        if let Some(Value::Object(compat)) = map.get("__compat") {
            if let Some(Value::Object(support)) = compat.get("support") {
                handle_support_entry(path, filename, support, results);
            }
        }

        for (key, child) in map {
            if key.starts_with("__") {
                continue;
            }

            match child {
                Value::Object(_) => {
                    path.push(key.clone());
                    walk_support_entries(path, child, filename, results);
                    path.pop();
                }
                Value::Array(items) => {
                    for (index, item) in items.iter().enumerate() {
                        if item.is_object() {
                            path.push(format!("{}[{}]", key, index));
                            walk_support_entries(path, item, filename, results);
                            path.pop();
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn handle_support_entry(
    path: &[String],
    filename: &str,
    support: &Map<String, Value>,
    results: &mut Vec<ChromeOnlyApi>,
) {
    let chrome_info = support.get("chrome");
    let firefox_info = support.get("firefox");

    if let Some(chrome_info) = chrome_info {
        if is_supported(chrome_info) && !is_supported(firefox_info.unwrap_or(&Value::Null)) {
            let feature_path = path.join(".");
            results.push(ChromeOnlyApi {
                feature_path,
                source_file: filename.to_string(),
                chrome_info: chrome_info.clone(),
                firefox_info: firefox_info.cloned().unwrap_or(Value::Null),
            });
        }
    }
}

fn matches_known_chrome_only(chrome_path: &str) -> Option<&'static str> {
    CHROME_ONLY_APIS
        .iter()
        .copied()
        .find(|prefix| chrome_path.starts_with(prefix))
}

fn is_supported(entry: &Value) -> bool {
    match entry {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(_) => true,
        Value::String(text) => {
            let normalized = text.trim().to_lowercase();
            !normalized.is_empty() && normalized != "false" && normalized != "no"
        }
        Value::Array(items) => items.iter().any(is_supported),
        Value::Object(map) => {
            match map.get("version_added") {
                Some(Value::Bool(value)) => *value,
                Some(Value::Number(_)) => true,
                Some(Value::String(text)) => {
                    let normalized = text.trim().to_lowercase();
                    !matches!(normalized.as_str(), "" | "false" | "mirrored")
                }
                Some(Value::Array(items)) => items.iter().any(is_supported),
                Some(Value::Object(obj)) => is_supported(&Value::Object(obj.clone())),
                Some(Value::Null) | None => false,
            }
        }
    }
}

fn format_version(entry: &Value) -> String {
    match entry {
        Value::Null => "not supported".into(),
        Value::Bool(true) => "supported".into(),
        Value::Bool(false) => "not supported".into(),
        Value::Number(num) => num.to_string(),
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                "not supported".into()
            } else {
                trimmed.to_string()
            }
        }
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(format_version).collect();
            parts.join("; ")
        }
        Value::Object(map) => {
            match map.get("version_added") {
                Some(Value::String(text)) => {
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        "not supported".into()
                    } else {
                        trimmed.to_string()
                    }
                }
                Some(Value::Bool(true)) => "supported".into(),
                Some(Value::Bool(false)) | Some(Value::Null) | None => "not supported".into(),
                Some(Value::Number(num)) => num.to_string(),
                Some(other) => format_version(other),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn supported_detection_handles_various_forms() {
        assert!(is_supported(&json!(true)));
        assert!(is_supported(&json!("1.0")));
        assert!(!is_supported(&json!("")));
        assert!(!is_supported(&json!("false")));
        assert!(is_supported(&json!("Note")));
        assert!(is_supported(&json!({"version_added": "50"})));
        assert!(!is_supported(&json!({"version_added": ""})));
        assert!(is_supported(&json!({"version_added": true})));
        assert!(!is_supported(&json!({"version_added": false})));
    }

    #[test]
    fn format_version_matches_expectations() {
        assert_eq!(format_version(&json!(null)), "not supported");
        assert_eq!(format_version(&json!(true)), "supported");
        assert_eq!(format_version(&json!("")), "not supported");
        assert_eq!(format_version(&json!(" 72 " )), "72");
        assert_eq!(
            format_version(&json!({"version_added": "42"})),
            "42"
        );
        assert_eq!(
            format_version(&json!([{ "version_added": "1" }, null])),
            "1; not supported"
        );
    }

    #[test]
    fn matches_known_prefixes() {
        assert!(matches_known_chrome_only("chrome.offscreen.createDocument").is_some());
        assert!(matches_known_chrome_only("chrome.action.openPopup").is_some());
        assert!(matches_known_chrome_only("chrome.tabs.getSelected").is_some());
        assert!(matches_known_chrome_only("chrome.runtime.getPackageDirectoryEntry").is_some());
    }

    #[test]
    fn detects_unknown_prefixes() {
        assert!(matches_known_chrome_only("chrome.tabs.query").is_none());
        assert!(matches_known_chrome_only("chrome.cookies.getAll").is_none());
    }
}
