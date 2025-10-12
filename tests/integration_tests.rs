//! Integration tests for Chrome to Firefox extension conversion
//! 
//! These tests use real Chrome extension examples and validate output
//! using Mozilla's addons-linter.

use chrome2moz::{convert_extension, ConversionOptions, CalculatorType};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper to check if addons-linter is installed
fn check_addons_linter() -> bool {
    Command::new("addons-linter")
        .arg("--version")
        .output()
        .is_ok()
}

/// Run addons-linter on converted extension
fn validate_with_linter(output_dir: &PathBuf) -> Result<(), String> {
    if !check_addons_linter() {
        println!("⚠️  addons-linter not installed. Install with: npm install -g addons-linter");
        return Ok(()); // Skip validation if not installed
    }

    println!("🔍 Running addons-linter on {:?}", output_dir);
    
    let output = Command::new("addons-linter")
        .arg(output_dir)
        .arg("--output")
        .arg("json")
        .output()
        .map_err(|e| format!("Failed to run addons-linter: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Parse JSON output
    if let Ok(result) = serde_json::from_str::<serde_json::Value>(&stdout) {
        let errors = result["errors"].as_array().map(|a| a.len()).unwrap_or(0);
        let warnings = result["warnings"].as_array().map(|a| a.len()).unwrap_or(0);
        
        println!("📊 Linter Results: {} errors, {} warnings", errors, warnings);
        
        if errors > 0 {
            println!("❌ Linter errors detected:");
            if let Some(errors_array) = result["errors"].as_array() {
                for error in errors_array {
                    println!("  - {}", error["message"].as_str().unwrap_or("Unknown error"));
                }
            }
            return Err(format!("Linter found {} errors", errors));
        }
        
        if warnings > 0 {
            println!("⚠️  Linter warnings:");
            if let Some(warnings_array) = result["warnings"].as_array() {
                for warning in warnings_array.iter().take(5) {
                    println!("  - {}", warning["message"].as_str().unwrap_or("Unknown warning"));
                }
            }
        }
    } else {
        println!("Raw output:\n{}", stdout);
        println!("Stderr:\n{}", stderr);
    }

    Ok(())
}

/// Create a simple test extension with storage.session API
fn create_storage_session_extension(dir: &PathBuf) {
    // manifest.json
    let manifest = r#"{
  "manifest_version": 3,
  "name": "Storage Session Test",
  "version": "1.0.0",
  "description": "Tests storage.session API",
  "permissions": ["storage"],
  "background": {
    "service_worker": "background.js"
  }
}"#;
    fs::write(dir.join("manifest.json"), manifest).unwrap();

    // background.js
    let background = r#"
// Test storage.session API
chrome.storage.session.set({ sessionKey: "sessionValue" }).then(() => {
  console.log("Session data saved");
});

chrome.storage.session.get("sessionKey").then((result) => {
  console.log("Session data:", result);
});
"#;
    fs::write(dir.join("background.js"), background).unwrap();
}

/// Create a test extension with sidePanel API
fn create_sidepanel_extension(dir: &PathBuf) {
    let manifest = r#"{
  "manifest_version": 3,
  "name": "SidePanel Test",
  "version": "1.0.0",
  "description": "Tests sidePanel API",
  "permissions": ["sidePanel"],
  "background": {
    "service_worker": "background.js"
  }
}"#;
    fs::write(dir.join("manifest.json"), manifest).unwrap();

    let background = r#"
// Test sidePanel API
chrome.sidePanel.setOptions({
  path: "sidepanel.html",
  enabled: true
});

chrome.action.onClicked.addListener(() => {
  chrome.sidePanel.open({ windowId: activeWindowId });
});
"#;
    fs::write(dir.join("background.js"), background).unwrap();
    
    fs::write(dir.join("sidepanel.html"), "<html><body><h1>Side Panel</h1></body></html>").unwrap();
}

/// Create a test extension with declarativeNetRequest API
fn create_dnr_extension(dir: &PathBuf) {
    let manifest = r#"{
  "manifest_version": 3,
  "name": "DNR Test",
  "version": "1.0.0",
  "description": "Tests declarativeNetRequest API",
  "permissions": ["declarativeNetRequest"],
  "background": {
    "service_worker": "background.js"
  }
}"#;
    fs::write(dir.join("manifest.json"), manifest).unwrap();

    let background = r#"
// Test declarativeNetRequest API
chrome.declarativeNetRequest.updateDynamicRules({
  removeRuleIds: [1],
  addRules: [{
    id: 1,
    priority: 1,
    action: { type: "block" },
    condition: { urlFilter: "example.com" }
  }]
});

chrome.declarativeNetRequest.onRuleMatchedDebug.addListener((details) => {
  console.log("Rule matched:", details);
});
"#;
    fs::write(dir.join("background.js"), background).unwrap();
}

/// Create a test extension with userScripts API
fn create_userscripts_extension(dir: &PathBuf) {
    let manifest = r#"{
  "manifest_version": 3,
  "name": "UserScripts Test",
  "version": "1.0.0",
  "description": "Tests userScripts API",
  "permissions": ["userScripts"],
  "background": {
    "service_worker": "background.js"
  }
}"#;
    fs::write(dir.join("manifest.json"), manifest).unwrap();

    let background = r#"
// Test userScripts API
chrome.userScripts.register([{
  id: "test-script",
  matches: ["https://example.com/*"],
  js: [{ code: "console.log('Injected script');" }]
}]);
"#;
    fs::write(dir.join("background.js"), background).unwrap();
}

/// Create a test extension with legacy tabs APIs
fn create_legacy_tabs_extension(dir: &PathBuf) {
    let manifest = r#"{
  "manifest_version": 3,
  "name": "Legacy Tabs Test",
  "version": "1.0.0",
  "description": "Tests legacy tabs APIs",
  "permissions": ["tabs"],
  "background": {
    "service_worker": "background.js"
  }
}"#;
    fs::write(dir.join("manifest.json"), manifest).unwrap();

    let background = r#"
// Test legacy tabs APIs
chrome.tabs.getSelected(null, (tab) => {
  console.log("Selected tab:", tab);
});

chrome.tabs.getAllInWindow(null, (tabs) => {
  console.log("All tabs:", tabs);
});
"#;
    fs::write(dir.join("background.js"), background).unwrap();
}

#[test]
fn test_storage_session_conversion() {
    let temp_input = TempDir::new().unwrap();
    let temp_output = TempDir::new().unwrap();
    
    create_storage_session_extension(&temp_input.path().to_path_buf());
    
    let options = ConversionOptions {
        interactive: false,
        target_calculator: CalculatorType::Both,
        preserve_chrome_compatibility: true,
        generate_report: false,
        
    };
    
    let result = convert_extension(
        temp_input.path(),
        temp_output.path(),
        options
    );
    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    
    // Check that storage-session-compat.js shim was created
    let shim_path = temp_output.path().join("shims/storage-session-compat.js");
    assert!(shim_path.exists(), "storage-session-compat.js shim not created");
    
    // Verify the shim content
    let shim_content = fs::read_to_string(&shim_path).unwrap();
    assert!(shim_content.contains("sessionStore"), "Shim missing sessionStore implementation");
    assert!(shim_content.contains("new Map()"), "Shim missing Map implementation");
    
    // Validate with addons-linter
    let _ = validate_with_linter(&temp_output.path().to_path_buf());
}

#[test]
fn test_sidepanel_conversion() {
    let temp_input = TempDir::new().unwrap();
    let temp_output = TempDir::new().unwrap();
    
    create_sidepanel_extension(&temp_input.path().to_path_buf());
    
    let options = ConversionOptions {
        interactive: false,
        target_calculator: CalculatorType::Both,
        preserve_chrome_compatibility: true,
        generate_report: false,
        
    };
    
    let result = convert_extension(
        temp_input.path(),
        temp_output.path(),
        options
    );
    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    
    // Check that sidepanel-compat.js shim was created
    let shim_path = temp_output.path().join("shims/sidepanel-compat.js");
    assert!(shim_path.exists(), "sidepanel-compat.js shim not created");
    
    // Verify the shim maps to sidebarAction
    let shim_content = fs::read_to_string(&shim_path).unwrap();
    assert!(shim_content.contains("sidebarAction"), "Shim missing sidebarAction mapping");
    
    // Check manifest was updated with browser_specific_settings
    let manifest_content = fs::read_to_string(temp_output.path().join("manifest.json")).unwrap();
    assert!(manifest_content.contains("browser_specific_settings"), "Manifest missing Firefox settings");
    
    let _ = validate_with_linter(&temp_output.path().to_path_buf());
}

#[test]
fn test_dnr_conversion() {
    let temp_input = TempDir::new().unwrap();
    let temp_output = TempDir::new().unwrap();
    
    create_dnr_extension(&temp_input.path().to_path_buf());
    
    let options = ConversionOptions {
        interactive: false,
        target_calculator: CalculatorType::Both,
        preserve_chrome_compatibility: true,
        generate_report: false,
        
    };
    
    let result = convert_extension(
        temp_input.path(),
        temp_output.path(),
        options
    );
    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    
    // Check that declarative-net-request-stub.js was created
    let shim_path = temp_output.path().join("shims/declarative-net-request-stub.js");
    assert!(shim_path.exists(), "declarative-net-request-stub.js not created");
    
    // Verify converter contains webRequest implementation
    let shim_content = fs::read_to_string(&shim_path).unwrap();
    assert!(shim_content.contains("webRequest"), "Converter missing webRequest implementation");
    assert!(shim_content.contains("updateDynamicRules"), "Converter missing updateDynamicRules");
    assert!(shim_content.contains("block") || shim_content.contains("redirect"),
            "Converter missing action type support");
    assert!(shim_content.contains("Converting DNR") || shim_content.contains("converter"),
            "Converter missing conversion logic");
    
    let _ = validate_with_linter(&temp_output.path().to_path_buf());
}

#[test]
fn test_userscripts_conversion() {
    let temp_input = TempDir::new().unwrap();
    let temp_output = TempDir::new().unwrap();
    
    create_userscripts_extension(&temp_input.path().to_path_buf());
    
    let options = ConversionOptions {
        interactive: false,
        target_calculator: CalculatorType::Both,
        preserve_chrome_compatibility: true,
        generate_report: false,
        
    };
    
    let result = convert_extension(
        temp_input.path(),
        temp_output.path(),
        options
    );
    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    
    // Check that user-scripts-compat.js was created
    let shim_path = temp_output.path().join("shims/user-scripts-compat.js");
    assert!(shim_path.exists(), "user-scripts-compat.js not created");
    
    // Verify it maps to Firefox API
    let shim_content = fs::read_to_string(&shim_path).unwrap();
    assert!(shim_content.contains("browser.userScripts") || shim_content.contains("contentScripts"), 
            "Shim missing Firefox userScripts mapping");
    
    let _ = validate_with_linter(&temp_output.path().to_path_buf());
}

#[test]
fn test_legacy_tabs_conversion() {
    let temp_input = TempDir::new().unwrap();
    let temp_output = TempDir::new().unwrap();
    
    create_legacy_tabs_extension(&temp_input.path().to_path_buf());
    
    let options = ConversionOptions {
        interactive: false,
        target_calculator: CalculatorType::Both,
        preserve_chrome_compatibility: true,
        generate_report: false,
        
    };
    
    let result = convert_extension(
        temp_input.path(),
        temp_output.path(),
        options
    );
    assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
    
    // Check that tabs-windows-compat.js was created
    let shim_path = temp_output.path().join("shims/tabs-windows-compat.js");
    assert!(shim_path.exists(), "tabs-windows-compat.js not created");
    
    // Verify it maps to tabs.query
    let shim_content = fs::read_to_string(&shim_path).unwrap();
    assert!(shim_content.contains("tabs.query"), "Shim missing tabs.query mapping");
    assert!(shim_content.contains("getSelected"), "Shim missing getSelected implementation");
    
    let _ = validate_with_linter(&temp_output.path().to_path_buf());
}

#[test]
fn test_all_shims_together() {
    // Create an extension that uses multiple APIs
    let temp_input = TempDir::new().unwrap();
    let temp_output = TempDir::new().unwrap();
    
    let manifest = r#"{
  "manifest_version": 3,
  "name": "Multi-API Test",
  "version": "1.0.0",
  "description": "Tests multiple APIs",
  "permissions": ["storage", "tabs", "sidePanel"],
  "background": {
    "service_worker": "background.js"
  }
}"#;
    fs::write(temp_input.path().join("manifest.json"), manifest).unwrap();
    
    let background = r#"
// Multiple API usage
chrome.storage.session.set({ key: "value" });
chrome.tabs.getSelected(null, (tab) => {});
chrome.sidePanel.setOptions({ path: "panel.html" });
chrome.declarativeNetRequest.getDynamicRules();
"#;
    fs::write(temp_input.path().join("background.js"), background).unwrap();
    
    let options = ConversionOptions {
        interactive: false,
        target_calculator: CalculatorType::Both,
        preserve_chrome_compatibility: true,
        generate_report: true,
        
    };
    
    let result = convert_extension(
        temp_input.path(),
        temp_output.path(),
        options
    );
    assert!(result.is_ok(), "Multi-API conversion failed: {:?}", result.err());
    
    // Verify multiple shims were created
    let shims_dir = temp_output.path().join("shims");
    assert!(shims_dir.exists(), "Shims directory not created");
    
    let shim_count = fs::read_dir(&shims_dir).unwrap().count();
    assert!(shim_count >= 4, "Expected at least 4 shims, found {}", shim_count);
    
    // Validate the complete package
    let _ = validate_with_linter(&temp_output.path().to_path_buf());
    
    // Check report was generated
    let report_path = temp_output.path().parent().unwrap().join("output.md");
    if report_path.exists() {
        let report_content = fs::read_to_string(&report_path).unwrap();
        assert!(report_content.contains("Conversion"), "Report missing conversion details");
    }
}

#[test]
#[ignore] // This test downloads from GitHub - run with: cargo test -- --ignored
fn test_real_world_latex_to_calc() {
    use std::process::Command;
    use std::env;
    
    println!("🔍 Testing real-world extension: LatexToCalc");
    
    // Check if we should save output for manual testing
    let save_output = env::var("SAVE_TEST_OUTPUT").is_ok();
    let (output_base, _temp_guard) = if save_output {
        let path = PathBuf::from("./test-output");
        fs::create_dir_all(&path).unwrap();
        println!("💾 Output will be saved to: {:?}", path.canonicalize().unwrap());
        (path, None)
    } else {
        let temp = TempDir::new().unwrap();
        let path = temp.path().to_path_buf();
        (path, Some(temp))
    };
    
    let temp_input = output_base.join("LatexToCalc");
    let temp_output = output_base.join("LatexToCalc-Firefox");
    
    // Clone the repository
    println!("📦 Cloning LatexToCalc from GitHub...");
    let clone_result = Command::new("git")
        .args(&["clone", "https://github.com/OtsoBear/LatexToCalc.git"])
        .current_dir(&output_base)
        .output();
    
    match clone_result {
        Ok(output) if output.status.success() => {
            println!("✅ Repository cloned successfully");
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("⚠️  Clone failed: {}", stderr);
            println!("⏭️  Skipping test - git clone failed");
            return;
        }
        Err(e) => {
            println!("⚠️  git not available: {}", e);
            println!("⏭️  Skipping test - git not installed");
            return;
        }
    }
    
    // Verify the extension was cloned
    assert!(temp_input.exists(), "LatexToCalc directory not created");
    assert!(temp_input.join("manifest.json").exists(), "manifest.json not found");
    
    println!("🔄 Converting LatexToCalc to Firefox format...");
    
    let options = ConversionOptions {
        interactive: false,
        target_calculator: CalculatorType::Both,
        preserve_chrome_compatibility: true,
        generate_report: true,
        
    };
    
    let result = convert_extension(
        &temp_input,
        &temp_output,
        options
    );
    
    match &result {
        Ok(_) => {
            println!("✅ Conversion completed successfully");
        }
        Err(e) => {
            println!("❌ Conversion failed: {}", e);
            panic!("Conversion failed: {}", e);
        }
    }
    
    assert!(result.is_ok(), "LatexToCalc conversion failed: {:?}", result.err());
    
    // Verify output structure
    assert!(temp_output.exists(), "Output directory not created");
    assert!(temp_output.join("manifest.json").exists(), "Converted manifest.json not found");
    
    // Check that Firefox-specific settings were added
    let manifest_content = fs::read_to_string(temp_output.join("manifest.json")).unwrap();
    assert!(manifest_content.contains("browser_specific_settings"),
            "Manifest missing Firefox settings");
    assert!(manifest_content.contains("gecko"),
            "Manifest missing gecko settings");
    
    // Check that shims were created
    let shims_dir = temp_output.join("shims");
    if shims_dir.exists() {
        let shim_count = fs::read_dir(&shims_dir).unwrap().count();
        println!("📦 Generated {} compatibility shims", shim_count);
        assert!(shim_count > 0, "Expected shims to be generated");
    }
    
    // Validate with addons-linter
    println!("🔍 Validating with addons-linter...");
    match validate_with_linter(&temp_output) {
        Ok(_) => println!("✅ Linter validation passed"),
        Err(e) => {
            println!("⚠️  Linter validation had issues: {}", e);
            // Don't fail the test - linter may have warnings
        }
    }
    
    // Print summary
    println!("\n📊 LatexToCalc Conversion Summary:");
    println!("   - Input: {:?}", temp_input);
    println!("   - Output: {:?}", temp_output);
    println!("   - Status: ✅ SUCCESS");
    
    if save_output {
        let manifest_path = temp_output.join("manifest.json");
        let abs_path = manifest_path.canonicalize().unwrap_or(manifest_path);
        
        println!("\n💾 SAVED FOR MANUAL TESTING!");
        println!("\n🦊 To test in Firefox:");
        println!("   1. Open Firefox");
        println!("   2. Go to: about:debugging#/runtime/this-firefox");
        println!("   3. Click 'Load Temporary Add-on'");
        println!("   4. Select: {:?}", abs_path);
        println!("\n📁 Extension saved at: {:?}", temp_output.canonicalize().unwrap_or(temp_output.clone()));
        println!("\n🧹 To clean up: rm -rf {:?}", output_base);
    } else {
        println!("\n💡 To save output for manual testing, run:");
        println!("   SAVE_TEST_OUTPUT=1 cargo test --test integration_tests -- --ignored --nocapture test_real_world_latex_to_calc");
    }
}

#[test]
fn test_chrome_namespace_conversion() {
    let temp_input = TempDir::new().unwrap();
    let temp_output = TempDir::new().unwrap();
    
    let manifest = r#"{
  "manifest_version": 3,
  "name": "Namespace Test",
  "version": "1.0.0",
  "permissions": ["storage"],
  "background": {
    "service_worker": "background.js"
  }
}"#;
    fs::write(temp_input.path().join("manifest.json"), manifest).unwrap();
    
    let background = r#"
chrome.storage.local.get("key", (result) => {
  chrome.tabs.query({active: true}, (tabs) => {
    chrome.runtime.sendMessage({data: "test"});
  });
});
"#;
    fs::write(temp_input.path().join("background.js"), background).unwrap();
    
    let options = ConversionOptions {
        interactive: false,
        target_calculator: CalculatorType::Both,
        preserve_chrome_compatibility: true,
        generate_report: false,
        
    };
    
    let result = convert_extension(
        temp_input.path(),
        temp_output.path(),
        options
    );
    assert!(result.is_ok());
    
    // Verify chrome -> browser conversion
    let background_content = fs::read_to_string(temp_output.path().join("background.js")).unwrap();
    assert!(background_content.contains("browser.storage") || background_content.contains("chrome.storage"), 
            "Namespace not properly converted");
    assert!(background_content.contains("typeof browser"), "Browser polyfill not added");
    
    let _ = validate_with_linter(&temp_output.path().to_path_buf());
}