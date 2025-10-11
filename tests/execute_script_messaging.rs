//! Test for executeScript → messaging system transformation
//! 
//! Tests the transformation of chrome.scripting.executeScript calls
//! into proper message passing between background and content scripts.

use chrome_to_firefox::transformer::javascript::JavaScriptTransformer;
use std::path::PathBuf;

#[test]
fn test_execute_script_to_messaging() {
    let mut transformer = JavaScriptTransformer::new(&[]);
    
    // Simplified version of the LatexToCalc pattern
    let background_code = r#"
chrome.commands.onCommand.addListener(async (command) => {
    if (command === 'translate-clipboard') {
        const activeTabs = await chrome.tabs.query({ active: true, currentWindow: true });
        const activeTab = activeTabs[0];
        if (activeTab) {
            chrome.scripting.executeScript({
                target: { tabId: activeTab.id },
                function: async (reqId) => {
                    const latexContent = await extractLatexContent();
                    if (latexContent && latexContent.trim() !== "") {
                        chrome.runtime.sendMessage({ type: "LATEX_EXTRACTED", latexContent: latexContent, requestId: reqId });
                    }
                },
                args: [requestId]
            });
        }
    }
});
"#;

    let result = transformer.transform(background_code, &PathBuf::from("background.js")).unwrap();
    
    // Should be transformed to browser API
    assert!(result.new_content.contains("browser.commands.onCommand"));
    assert!(result.new_content.contains("browser.tabs.query"));
    
    // executeScript should be auto-converted to sendMessage (message passing)
    assert!(result.new_content.contains("browser.tabs.sendMessage"),
        "executeScript should be converted to message passing with sendMessage");
    
    // Should NOT contain executeScript anymore
    assert!(!result.new_content.contains("executeScript"),
        "executeScript should be completely replaced with message passing");
    
    // Verify all chrome references are transformed
    assert!(!result.new_content.contains("chrome.commands"));
    assert!(!result.new_content.contains("chrome.tabs.query"));
}

#[test]
fn test_execute_script_with_captured_variables() {
    let mut transformer = JavaScriptTransformer::new(&[]);
    
    let code = r#"
const config = { enabled: true };
chrome.tabs.query({ active: true }, (tabs) => {
    chrome.scripting.executeScript({
        target: { tabId: tabs[0].id },
        function: (settings) => {
            console.log('Settings:', settings);
        },
        args: [config]
    });
});
"#;

    let result = transformer.transform(code, &PathBuf::from("background.js")).unwrap();
    
    // Should transform to browser API
    assert!(result.new_content.contains("browser.tabs.query"));
    
    // executeScript should be converted to sendMessage (message passing)
    assert!(result.new_content.contains("browser.tabs.sendMessage"),
        "executeScript should be converted to message passing");
    
    // Should NOT contain executeScript
    assert!(!result.new_content.contains("executeScript"),
        "executeScript should be replaced with message passing");
}

#[test]
fn test_content_script_message_listener() {
    let mut transformer = JavaScriptTransformer::new(&[]);
    
    // Content script with message listener
    let content_code = r#"
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
    if (request.type === 'TRANSLATION_COMPLETED') {
        const totalTimeMs = request.totalTime;
        console.log(`Translation completed in ${totalTimeMs} ms`);
        showPopup('green', 'Translated and copied to clipboard.');
    } else if (request.type === 'SHOW_ERROR_POPUP') {
        showPopup('red', request.message);
    }
});
"#;

    let result = transformer.transform(content_code, &PathBuf::from("content.js")).unwrap();
    
    // Should transform to browser API
    assert!(result.new_content.contains("browser.runtime.onMessage"));
    assert!(!result.new_content.contains("chrome.runtime"));
}

#[test]
fn test_background_sends_message_to_content() {
    let mut transformer = JavaScriptTransformer::new(&[]);
    
    let code = r#"
async function notifyContentScript(message) {
    const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
    if (tabs[0]) {
        chrome.tabs.sendMessage(tabs[0].id, {
            type: 'TRANSLATION_COMPLETED',
            data: message
        });
    }
}
"#;

    let result = transformer.transform(code, &PathBuf::from("background.js")).unwrap();
    
    // Should transform all chrome APIs to browser
    assert!(result.new_content.contains("browser.tabs.query"));
    assert!(result.new_content.contains("browser.tabs.sendMessage"));
    assert!(!result.new_content.contains("chrome.tabs"));
}

#[test]
fn test_complex_execute_script_pattern() {
    let mut transformer = JavaScriptTransformer::new(&[]);
    
    // Pattern from real extension with nested callbacks
    let code = r#"
chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
    if (tabs && tabs[0]) {
        chrome.scripting.executeScript({
            target: { tabId: tabs[0].id },
            function: performClipboardCopy,
            args: [text, trackTiming]
        }, (result) => {
            if (chrome.runtime.lastError) {
                console.error('Error:', chrome.runtime.lastError.message);
            } else {
                console.log('Script executed successfully');
            }
        });
    }
});
"#;

    let result = transformer.transform(code, &PathBuf::from("background.js")).unwrap();
    
    // All chrome APIs should be transformed
    assert!(result.new_content.contains("browser.tabs.query"));
    
    // Note: This test has a function reference (performClipboardCopy) which is NOT an inline function
    // so it won't be auto-transformed to message passing. It should remain as executeScript.
    assert!(result.new_content.contains("browser.scripting.executeScript") ||
            result.new_content.contains("browser.tabs.executeScript"),
        "Function references (not inline) should keep executeScript");
    
    assert!(result.new_content.contains("browser.runtime"));
    
    // Verify no chrome references remain
    assert!(!result.new_content.contains("chrome.tabs"));
    assert!(!result.new_content.contains("chrome.scripting"));
}

#[test]
fn test_execute_script_basic_transformation() {
    let mut transformer = JavaScriptTransformer::new(&[]);
    
    let code = "chrome.scripting.executeScript({ target: { tabId: 123 }, func: () => {} });";
    
    let result = transformer.transform(code, &PathBuf::from("background.js")).unwrap();
    
    // Should transform to browser API
    assert!(result.new_content.contains("browser.scripting.executeScript") ||
            result.new_content.contains("browser.tabs.executeScript"));
    
    // Verify chrome is replaced
    assert!(!result.new_content.contains("chrome.scripting"));
}

#[test]
fn test_full_background_to_content_flow() {
    let mut transformer = JavaScriptTransformer::new(&[]);
    
    // Background script that communicates with content
    let background_code = "
// Background script
chrome.runtime.onInstalled.addListener(() => {
    console.log('Extension installed');
});

chrome.commands.onCommand.addListener(async (command) => {
    const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
    if (tabs[0]) {
        // Execute function in content script
        chrome.scripting.executeScript({
            target: { tabId: tabs[0].id },
            function: () => {
                return document.title;
            }
        });
        
        // Also send a direct message
        chrome.tabs.sendMessage(tabs[0].id, { type: 'PING' });
    }
});

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
    if (msg.type === 'PONG') {
        console.log('Received pong from content script');
        sendResponse({ received: true });
    }
});
";

    let result = transformer.transform(background_code, &PathBuf::from("background.js")).unwrap();
    
    // All chrome APIs should be converted to browser
    assert!(result.new_content.contains("browser.runtime.onInstalled"));
    assert!(result.new_content.contains("browser.commands.onCommand"));
    assert!(result.new_content.contains("browser.tabs.query"));
    assert!(result.new_content.contains("browser.tabs.sendMessage"));
    assert!(result.new_content.contains("browser.runtime.onMessage"));
    
    // executeScript should be converted to message passing (sendMessage)
    // It should NOT still contain executeScript calls
    assert!(!result.new_content.contains("executeScript"),
        "executeScript should be converted to message passing");
    
    // No chrome references should remain
    let chrome_count = result.new_content.matches("chrome.").count();
    assert_eq!(chrome_count, 0, "Found {} chrome. references, expected 0", chrome_count);
}