//! Compatibility shim generation

use crate::models::{ConversionContext, NewFile};
use anyhow::Result;
use std::path::PathBuf;

/// Generate compatibility shims for Firefox
pub fn generate_shims(context: &ConversionContext) -> Result<Vec<NewFile>> {
    let mut shims = Vec::new();
    
    // Check if we need browser namespace polyfill
    let needs_browser_polyfill = context.source
        .get_javascript_files()
        .iter()
        .any(|path| {
            context.source.get_file_content(path)
                .map(|content| content.contains("chrome."))
                .unwrap_or(false)
        });
    
    if needs_browser_polyfill {
        shims.push(create_browser_polyfill());
    }
    
    // Check if we need promise wrapper
    let needs_promise_wrapper = context.incompatibilities
        .iter()
        .any(|i| matches!(i.category, crate::models::incompatibility::IncompatibilityCategory::CallbackVsPromise));
    
    if needs_promise_wrapper {
        shims.push(create_promise_wrapper());
    }
    
    // Check if we need action compatibility
    if context.source.manifest.action.is_some() || context.source.manifest.browser_action.is_some() {
        shims.push(create_action_compat());
    }
    
    Ok(shims)
}

fn create_browser_polyfill() -> NewFile {
    let content = r#"// Browser namespace polyfill for Chrome compatibility
// This allows the extension to work in both Chrome and Firefox

if (typeof browser === 'undefined') {
  // Chrome doesn't have 'browser' namespace, so we create it
  window.browser = window.chrome;
}

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
  module.exports = browser;
}
"#;
    
    NewFile {
        path: PathBuf::from("shims/browser-polyfill.js"),
        content: content.to_string(),
        purpose: "Provides browser namespace compatibility between Chrome and Firefox".to_string(),
    }
}

fn create_promise_wrapper() -> NewFile {
    let content = r#"// Promise wrapper for callback-based Chrome APIs
// Converts Chrome's callback-style APIs to promise-based for Firefox compatibility

/**
 * Wraps a Chrome API function to return a Promise instead of using callbacks
 * @param {Function} fn - The Chrome API function to wrap
 * @returns {Function} A function that returns a Promise
 */
function promisify(fn) {
  return function(...args) {
    return new Promise((resolve, reject) => {
      fn(...args, (...results) => {
        if (chrome.runtime.lastError) {
          reject(new Error(chrome.runtime.lastError.message));
        } else {
          resolve(results.length === 1 ? results[0] : results);
        }
      });
    });
  };
}

/**
 * Promisified versions of common Chrome APIs
 */
const promisifiedAPIs = {
  storage: {
    local: {
      get: promisify(chrome.storage.local.get.bind(chrome.storage.local)),
      set: promisify(chrome.storage.local.set.bind(chrome.storage.local)),
      remove: promisify(chrome.storage.local.remove.bind(chrome.storage.local)),
      clear: promisify(chrome.storage.local.clear.bind(chrome.storage.local)),
    },
    sync: {
      get: promisify(chrome.storage.sync.get.bind(chrome.storage.sync)),
      set: promisify(chrome.storage.sync.set.bind(chrome.storage.sync)),
      remove: promisify(chrome.storage.sync.remove.bind(chrome.storage.sync)),
      clear: promisify(chrome.storage.sync.clear.bind(chrome.storage.sync)),
    }
  },
  tabs: {
    query: promisify(chrome.tabs.query.bind(chrome.tabs)),
    get: promisify(chrome.tabs.get.bind(chrome.tabs)),
    create: promisify(chrome.tabs.create.bind(chrome.tabs)),
    update: promisify(chrome.tabs.update.bind(chrome.tabs)),
    remove: promisify(chrome.tabs.remove.bind(chrome.tabs)),
  },
  runtime: {
    sendMessage: promisify(chrome.runtime.sendMessage.bind(chrome.runtime)),
  }
};

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
  module.exports = { promisify, promisifiedAPIs };
}
"#;
    
    NewFile {
        path: PathBuf::from("shims/promise-wrapper.js"),
        content: content.to_string(),
        purpose: "Converts callback-based Chrome APIs to promise-based for Firefox".to_string(),
    }
}

fn create_action_compat() -> NewFile {
    let content = r#"// Action API compatibility shim
// Provides compatibility between MV2 browser_action and MV3 action APIs

const browserAction = chrome.action || chrome.browserAction;

// Unified API that works with both
const actionAPI = {
  setIcon: (details) => browserAction.setIcon(details),
  setTitle: (details) => browserAction.setTitle(details),
  setBadgeText: (details) => browserAction.setBadgeText(details),
  setBadgeBackgroundColor: (details) => browserAction.setBadgeBackgroundColor(details),
  setPopup: (details) => browserAction.setPopup(details),
  getTitle: (details) => browserAction.getTitle(details),
  getPopup: (details) => browserAction.getPopup(details),
  getBadgeText: (details) => browserAction.getBadgeText(details),
  getBadgeBackgroundColor: (details) => browserAction.getBadgeBackgroundColor(details),
};

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
  module.exports = actionAPI;
}
"#;
    
    NewFile {
        path: PathBuf::from("shims/action-compat.js"),
        content: content.to_string(),
        purpose: "Provides compatibility between browser_action and action APIs".to_string(),
    }
}

fn create_storage_session_compat() -> NewFile {
    let content = r#"// Storage session compatibility shim
// Provides fallback for chrome.storage.session (Chrome 102+) to local storage

const storageSession = chrome.storage.session || {
  get: (keys) => chrome.storage.local.get(keys),
  set: (items) => chrome.storage.local.set(items),
  remove: (keys) => chrome.storage.local.remove(keys),
  clear: () => chrome.storage.local.clear(),
};

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
  module.exports = storageSession;
}
"#;
    
    NewFile {
        path: PathBuf::from("shims/storage-session-compat.js"),
        content: content.to_string(),
        purpose: "Provides fallback for chrome.storage.session API".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_browser_polyfill_generation() {
        let polyfill = create_browser_polyfill();
        assert!(polyfill.content.contains("typeof browser === 'undefined'"));
        assert_eq!(polyfill.path, PathBuf::from("shims/browser-polyfill.js"));
    }
    
    #[test]
    fn test_promise_wrapper_generation() {
        let wrapper = create_promise_wrapper();
        assert!(wrapper.content.contains("promisify"));
        assert!(wrapper.content.contains("chrome.runtime.lastError"));
    }
}