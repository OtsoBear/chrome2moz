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
    
    // Check if we need storage.session polyfill
    let needs_storage_session = context.source
        .get_javascript_files()
        .iter()
        .any(|path| {
            context.source.get_file_content(path)
                .map(|content| content.contains("storage.session"))
                .unwrap_or(false)
        });
    
    if needs_storage_session {
        shims.push(create_storage_session_compat());
    }
    
    // Check if we need sidePanel compatibility
    let needs_sidepanel = context.source
        .get_javascript_files()
        .iter()
        .any(|path| {
            context.source.get_file_content(path)
                .map(|content| content.contains("sidePanel"))
                .unwrap_or(false)
        });
    
    if needs_sidepanel {
        shims.push(create_sidepanel_compat());
    }
    
    // Check if we need declarativeNetRequest stub
    let needs_dnr = context.source
        .get_javascript_files()
        .iter()
        .any(|path| {
            context.source.get_file_content(path)
                .map(|content| content.contains("declarativeNetRequest"))
                .unwrap_or(false)
        });
    
    if needs_dnr {
        shims.push(create_declarative_net_request_stub());
    }
    
    // Check if we need userScripts compatibility
    let needs_user_scripts = context.source
        .get_javascript_files()
        .iter()
        .any(|path| {
            context.source.get_file_content(path)
                .map(|content| content.contains("userScripts"))
                .unwrap_or(false)
        });
    
    if needs_user_scripts {
        shims.push(create_user_scripts_compat());
    }
    
    // Check if we need tabs/windows legacy API shims
    let needs_legacy_tabs = context.source
        .get_javascript_files()
        .iter()
        .any(|path| {
            context.source.get_file_content(path)
                .map(|content| {
                    content.contains("tabs.getSelected") ||
                    content.contains("tabs.getAllInWindow") ||
                    content.contains("windows.create")
                })
                .unwrap_or(false)
        });
    
    if needs_legacy_tabs {
        shims.push(create_tabs_windows_compat());
    }
    
    // Check if we need runtime compatibility stubs
    let needs_runtime_stubs = context.source
        .get_javascript_files()
        .iter()
        .any(|path| {
            context.source.get_file_content(path)
                .map(|content| content.contains("runtime.getPackageDirectoryEntry"))
                .unwrap_or(false)
        });
    
    if needs_runtime_stubs {
        shims.push(create_runtime_compat());
    }
    
    // Optional: Check if we need downloads compatibility
    let needs_downloads = context.source
        .get_javascript_files()
        .iter()
        .any(|path| {
            context.source.get_file_content(path)
                .map(|content| {
                    content.contains("downloads.acceptDanger") ||
                    content.contains("downloads.setShelfEnabled")
                })
                .unwrap_or(false)
        });
    
    if needs_downloads {
        shims.push(create_downloads_compat());
    }
    
    // Optional: Check if we need privacy API stubs
    let needs_privacy = context.source
        .get_javascript_files()
        .iter()
        .any(|path| {
            context.source.get_file_content(path)
                .map(|content| content.contains("chrome.privacy"))
                .unwrap_or(false)
        });
    
    if needs_privacy {
        shims.push(create_privacy_stub());
    }
    
    // Optional: Check if we need notifications compatibility
    let needs_notifications = context.source
        .get_javascript_files()
        .iter()
        .any(|path| {
            context.source.get_file_content(path)
                .map(|content| {
                    content.contains("notifications.create") &&
                    (content.contains("buttons:") || content.contains("imageUrl:"))
                })
                .unwrap_or(false)
        });
    
    if needs_notifications {
        shims.push(create_notifications_compat());
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
// Provides in-memory fallback for chrome.storage.session (Chrome 102+)
// Firefox doesn't support storage.session, so we use an in-memory Map()

(function() {
  'use strict';
  
  // In-memory storage for session data
  const sessionStore = new Map();
  
  const storageSessionCompat = {
    get: async function(keys) {
      if (keys === null || keys === undefined) {
        // Return all items
        const result = {};
        sessionStore.forEach((value, key) => {
          result[key] = value;
        });
        return result;
      }
      
      const keysArray = Array.isArray(keys) ? keys : [keys];
      const result = {};
      
      if (typeof keys === 'string') {
        const value = sessionStore.get(keys);
        if (value !== undefined) {
          result[keys] = value;
        }
        return result;
      }
      
      if (Array.isArray(keys)) {
        keysArray.forEach(key => {
          const value = sessionStore.get(key);
          if (value !== undefined) {
            result[key] = value;
          }
        });
        return result;
      }
      
      if (typeof keys === 'object') {
        // Keys is an object with default values
        Object.keys(keys).forEach(key => {
          const value = sessionStore.get(key);
          result[key] = value !== undefined ? value : keys[key];
        });
        return result;
      }
      
      return result;
    },
    
    set: async function(items) {
      Object.keys(items).forEach(key => {
        sessionStore.set(key, items[key]);
      });
      return;
    },
    
    remove: async function(keys) {
      const keysArray = Array.isArray(keys) ? keys : [keys];
      keysArray.forEach(key => {
        sessionStore.delete(key);
      });
      return;
    },
    
    clear: async function() {
      sessionStore.clear();
      return;
    },
    
    onChanged: {
      addListener: function(callback) {
        console.warn('storage.session.onChanged is not fully supported in this polyfill');
      },
      removeListener: function() {},
      hasListener: function() { return false; }
    }
  };
  
  // Attach to chrome/browser objects
  if (typeof chrome !== 'undefined' && chrome.storage && !chrome.storage.session) {
    chrome.storage.session = storageSessionCompat;
  }
  if (typeof browser !== 'undefined' && browser.storage && !browser.storage.session) {
    browser.storage.session = storageSessionCompat;
  }
  
  console.info('✅ storage.session polyfill loaded (in-memory)');
})();
"#;
    
    NewFile {
        path: PathBuf::from("shims/storage-session-compat.js"),
        content: content.to_string(),
        purpose: "Provides in-memory fallback for chrome.storage.session API".to_string(),
    }
}

fn create_sidepanel_compat() -> NewFile {
    let content = r#"// sidePanel API compatibility shim for Firefox
// Chrome's sidePanel API is not available in Firefox - provides sidebar fallback

(function() {
  'use strict';
  
  if (typeof chrome !== 'undefined' && !chrome.sidePanel && typeof browser !== 'undefined' && browser.sidebarAction) {
    console.info('⚙️ sidePanel compatibility shim loaded - using Firefox sidebar API');
    
    const sidePanelCompat = {
      setOptions: async function(options) {
        try {
          const firefoxOptions = {};
          if (options.tabId !== undefined) firefoxOptions.tabId = options.tabId;
          if (options.path !== undefined) firefoxOptions.panel = options.path;
          if (options.enabled !== undefined && options.enabled) {
            await browser.sidebarAction.open();
          }
          await browser.sidebarAction.setPanel(firefoxOptions);
        } catch (error) {
          console.error('❌ sidePanel.setOptions failed:', error);
          throw error;
        }
      },
      
      open: async function(options) {
        try {
          const openOptions = {};
          if (options && options.windowId !== undefined) {
            openOptions.windowId = options.windowId;
          }
          await browser.sidebarAction.open(openOptions);
        } catch (error) {
          console.error('❌ sidePanel.open failed:', error);
          throw error;
        }
      },
      
      getOptions: async function(options) {
        console.warn('⚠️ sidePanel.getOptions: Limited support in Firefox');
        try {
          const panel = await browser.sidebarAction.getPanel(options || {});
          return { path: panel, enabled: true };
        } catch (error) {
          return { enabled: false };
        }
      },
      
      setPanelBehavior: async function() {
        console.warn('⚠️ sidePanel.setPanelBehavior: Not supported in Firefox');
        return;
      },
      
      getPanelBehavior: async function() {
        return { openPanelOnActionClick: false };
      },
      
      onOpened: {
        addListener: function() {
          console.warn('⚠️ sidePanel.onOpened: Cannot be fully emulated in Firefox');
        },
        removeListener: function() {},
        hasListener: function() { return false; }
      }
    };
    
    if (typeof chrome !== 'undefined') chrome.sidePanel = sidePanelCompat;
    if (typeof browser !== 'undefined') browser.sidePanel = sidePanelCompat;
  }
})();
"#;
    
    NewFile {
        path: PathBuf::from("shims/sidepanel-compat.js"),
        content: content.to_string(),
        purpose: "Maps Chrome's sidePanel API to Firefox's sidebarAction".to_string(),
    }
}

fn create_declarative_net_request_stub() -> NewFile {
    let content = r#"// declarativeNetRequest stub for Firefox
// Chrome's declarativeNetRequest API has limited support in Firefox MV3

(function() {
  'use strict';
  
  console.warn('⚠️ declarativeNetRequest compatibility stub loaded');
  console.warn('⚠️ Firefox has limited support for declarativeNetRequest in MV3');
  console.info('💡 Consider using webRequest API for Firefox compatibility');
  
  if (typeof chrome !== 'undefined' && !chrome.declarativeNetRequest) {
    const dnrStub = {
      updateDynamicRules: async function() {
        throw new Error('declarativeNetRequest.updateDynamicRules not available in Firefox');
      },
      updateSessionRules: async function() {
        throw new Error('declarativeNetRequest.updateSessionRules not available in Firefox');
      },
      getDynamicRules: async function() { return []; },
      getSessionRules: async function() { return []; },
      updateEnabledRulesets: async function() {
        throw new Error('declarativeNetRequest.updateEnabledRulesets not available in Firefox');
      },
      getEnabledRulesets: async function() { return []; },
      getMatchedRules: async function() { return { rulesMatchedInfo: [] }; },
      setExtensionActionOptions: async function() { return; },
      getAvailableStaticRuleCount: async function() { return 0; },
      isRegexSupported: async function() {
        return { isSupported: false, reason: 'Not available in Firefox' };
      },
      testMatchOutcome: async function() { return { matchedRules: [] }; },
      
      onRuleMatchedDebug: {
        addListener: function() {
          console.warn('⚠️ declarativeNetRequest.onRuleMatchedDebug: Not supported');
        },
        removeListener: function() {},
        hasListener: function() { return false; }
      },
      
      MAX_NUMBER_OF_RULES: 0,
      MAX_NUMBER_OF_DYNAMIC_AND_SESSION_RULES: 0,
      MAX_NUMBER_OF_ENABLED_STATIC_RULESETS: 0,
      MAX_NUMBER_OF_REGEX_RULES: 0
    };
    
    if (typeof chrome !== 'undefined') chrome.declarativeNetRequest = dnrStub;
    if (typeof browser !== 'undefined') browser.declarativeNetRequest = dnrStub;
  }
})();
"#;
    
    NewFile {
        path: PathBuf::from("shims/declarative-net-request-stub.js"),
        content: content.to_string(),
        purpose: "Stubs declarativeNetRequest API with warnings for Firefox".to_string(),
    }
}

fn create_user_scripts_compat() -> NewFile {
    let content = r#"// userScripts API compatibility shim for Firefox
// Translates Chrome's userScripts API to Firefox's equivalent

(function() {
  'use strict';
  
  if (typeof chrome !== 'undefined' && !chrome.userScripts && typeof browser !== 'undefined') {
    console.info('⚙️ userScripts compatibility shim loaded');
    
    const userScriptsCompat = {
      register: async function(scripts) {
        // Firefox uses browser.contentScripts.register() or browser.userScripts.register()
        if (browser.userScripts && browser.userScripts.register) {
          // Firefox supports userScripts API (Firefox 102+)
          return await browser.userScripts.register(scripts);
        } else if (browser.contentScripts && browser.contentScripts.register) {
          // Fallback to contentScripts API
          console.info('💡 Using contentScripts.register as fallback');
          return await browser.contentScripts.register(scripts);
        } else {
          throw new Error('Neither userScripts nor contentScripts API available');
        }
      },
      
      unregister: async function(filter) {
        console.warn('⚠️ userScripts.unregister: Limited support');
        // Firefox doesn't have direct unregister by filter
        return;
      },
      
      update: async function(scripts) {
        console.warn('⚠️ userScripts.update: Not directly supported, use unregister + register');
        throw new Error('userScripts.update not available, use unregister then register');
      },
      
      getScripts: async function(filter) {
        console.warn('⚠️ userScripts.getScripts: Not supported in Firefox');
        return [];
      }
    };
    
    if (typeof chrome !== 'undefined') chrome.userScripts = userScriptsCompat;
    if (typeof browser !== 'undefined' && !browser.userScripts) {
      browser.userScripts = userScriptsCompat;
    }
  }
})();
"#;
    
    NewFile {
        path: PathBuf::from("shims/user-scripts-compat.js"),
        content: content.to_string(),
        purpose: "Maps Chrome's userScripts API to Firefox's equivalent".to_string(),
    }
}

fn create_tabs_windows_compat() -> NewFile {
    let content = r#"// Legacy tabs/windows API compatibility shim
// Maps deprecated Chrome APIs to modern equivalents

(function() {
  'use strict';
  
  const api = typeof browser !== 'undefined' ? browser : chrome;
  
  if (api && api.tabs) {
    // tabs.getSelected → tabs.query({active: true, currentWindow: true})
    if (!api.tabs.getSelected) {
      api.tabs.getSelected = async function(windowId, callback) {
        console.warn('⚠️ tabs.getSelected is deprecated, using tabs.query instead');
        const query = { active: true };
        if (windowId !== null && windowId !== undefined) {
          query.windowId = windowId;
        } else {
          query.currentWindow = true;
        }
        
        try {
          const tabs = await api.tabs.query(query);
          const result = tabs[0] || null;
          if (callback) callback(result);
          return result;
        } catch (error) {
          if (callback) callback(null);
          throw error;
        }
      };
    }
    
    // tabs.getAllInWindow → tabs.query({windowId: ...})
    if (!api.tabs.getAllInWindow) {
      api.tabs.getAllInWindow = async function(windowId, callback) {
        console.warn('⚠️ tabs.getAllInWindow is deprecated, using tabs.query instead');
        const query = windowId !== null && windowId !== undefined
          ? { windowId }
          : { currentWindow: true };
        
        try {
          const tabs = await api.tabs.query(query);
          if (callback) callback(tabs);
          return tabs;
        } catch (error) {
          if (callback) callback([]);
          throw error;
        }
      };
    }
  }
  
  if (api && api.windows && api.windows.create) {
    // Wrap windows.create to handle focused parameter
    const originalCreate = api.windows.create;
    api.windows.create = async function(createData, callback) {
      console.info('⚙️ windows.create: handling focused parameter');
      
      // Firefox supports focused parameter differently
      const data = { ...createData };
      if (data.focused !== undefined) {
        // Convert to state parameter for Firefox
        if (data.focused === false && !data.state) {
          data.state = 'minimized';
        }
      }
      
      try {
        const result = await originalCreate.call(this, data);
        if (callback) callback(result);
        return result;
      } catch (error) {
        if (callback) callback(null);
        throw error;
      }
    };
  }
  
  console.info('✅ Legacy tabs/windows API compatibility loaded');
})();
"#;
    
    NewFile {
        path: PathBuf::from("shims/tabs-windows-compat.js"),
        content: content.to_string(),
        purpose: "Provides compatibility for legacy tabs/windows APIs".to_string(),
    }
}

fn create_runtime_compat() -> NewFile {
    let content = r#"// Runtime API compatibility stubs
// Handles Chrome-specific runtime methods

(function() {
  'use strict';
  
  const api = typeof browser !== 'undefined' ? browser : chrome;
  
  if (api && api.runtime) {
    // runtime.getPackageDirectoryEntry stub
    if (!api.runtime.getPackageDirectoryEntry) {
      api.runtime.getPackageDirectoryEntry = function(callback) {
        console.warn('⚠️ runtime.getPackageDirectoryEntry is not supported in Firefox');
        console.info('💡 Use browser.runtime.getURL() for extension resources instead');
        
        // Return a stub DirectoryEntry-like object
        const stub = {
          isFile: false,
          isDirectory: true,
          name: 'extension-root',
          fullPath: '/',
          getFile: function() {
            throw new Error('getFile not supported - use browser.runtime.getURL()');
          },
          getDirectory: function() {
            throw new Error('getDirectory not supported - use browser.runtime.getURL()');
          }
        };
        
        if (callback) callback(stub);
        return Promise.resolve(stub);
      };
    }
  }
  
  console.info('✅ Runtime API compatibility loaded');
})();
"#;
    
    NewFile {
        path: PathBuf::from("shims/runtime-compat.js"),
        content: content.to_string(),
        purpose: "Stubs Chrome-specific runtime methods for Firefox".to_string(),
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

fn create_downloads_compat() -> NewFile {
    let content = r#"// Downloads API compatibility for Chrome-specific features
// Firefox doesn't support some Chrome-only downloads methods

(function() {
  'use strict';
  
  const api = typeof browser !== 'undefined' ? browser : chrome;
  
  if (api && api.downloads) {
    // downloads.acceptDanger stub
    if (!api.downloads.acceptDanger) {
      api.downloads.acceptDanger = async function(downloadId) {
        console.warn('⚠️ downloads.acceptDanger is not supported in Firefox');
        console.info('💡 Firefox handles dangerous downloads differently');
        throw new Error('downloads.acceptDanger not available in Firefox');
      };
    }
    
    // downloads.setShelfEnabled stub
    if (!api.downloads.setShelfEnabled) {
      api.downloads.setShelfEnabled = function(enabled) {
        console.warn('⚠️ downloads.setShelfEnabled is not supported in Firefox');
        console.info('💡 This controls Chrome\'s download shelf UI');
        // No-op in Firefox
      };
    }
    
    // Wrap downloads.download to filter unsupported options
    const originalDownload = api.downloads.download;
    api.downloads.download = async function(options) {
      const filteredOptions = { ...options };
      
      // Remove Chrome-only options
      if (filteredOptions.conflictAction) {
        console.warn('⚠️ downloads.download: conflictAction not supported in Firefox');
        delete filteredOptions.conflictAction;
      }
      
      return await originalDownload.call(this, filteredOptions);
    };
  }
  
  console.info('✅ Downloads API compatibility loaded');
})();
"#;
    
    NewFile {
        path: PathBuf::from("shims/downloads-compat.js"),
        content: content.to_string(),
        purpose: "Provides compatibility for Chrome-specific downloads features".to_string(),
    }
}

fn create_privacy_stub() -> NewFile {
    let content = r#"// Privacy API compatibility stub
// Firefox has limited support for chrome.privacy API

(function() {
  'use strict';
  
  console.warn('⚠️ Privacy API stub loaded');
  console.warn('⚠️ Firefox has different privacy settings architecture');
  
  if (typeof chrome !== 'undefined' && !chrome.privacy) {
    const privacyStub = {
      network: {
        networkPredictionEnabled: {
          get: async function() {
            console.warn('⚠️ privacy.network.networkPredictionEnabled: Not supported');
            return { value: false, levelOfControl: 'not_controllable' };
          },
          set: async function() {
            console.warn('⚠️ privacy.network.networkPredictionEnabled: Not supported');
            throw new Error('privacy.network settings not controllable in Firefox');
          },
          clear: async function() {
            console.warn('⚠️ privacy.network.networkPredictionEnabled: Not supported');
          }
        },
        webRTCIPHandlingPolicy: {
          get: async function() {
            console.warn('⚠️ privacy.network.webRTCIPHandlingPolicy: Not supported');
            return { value: 'default', levelOfControl: 'not_controllable' };
          },
          set: async function() {
            console.warn('⚠️ privacy.network.webRTCIPHandlingPolicy: Not supported');
            throw new Error('privacy.network settings not controllable in Firefox');
          },
          clear: async function() {}
        }
      },
      services: {
        alternateErrorPagesEnabled: {
          get: async function() { 
            return { value: false, levelOfControl: 'not_controllable' }; 
          },
          set: async function() {
            throw new Error('privacy.services not controllable in Firefox');
          },
          clear: async function() {}
        },
        autofillEnabled: {
          get: async function() { 
            return { value: true, levelOfControl: 'not_controllable' }; 
          },
          set: async function() {
            throw new Error('privacy.services not controllable in Firefox');
          },
          clear: async function() {}
        },
        safeBrowsingEnabled: {
          get: async function() { 
            return { value: true, levelOfControl: 'not_controllable' }; 
          },
          set: async function() {
            throw new Error('privacy.services not controllable in Firefox');
          },
          clear: async function() {}
        }
      },
      websites: {
        thirdPartyCookiesAllowed: {
          get: async function() { 
            return { value: true, levelOfControl: 'not_controllable' }; 
          },
          set: async function() {
            throw new Error('privacy.websites not controllable in Firefox');
          },
          clear: async function() {}
        },
        hyperlinkAuditingEnabled: {
          get: async function() { 
            return { value: true, levelOfControl: 'not_controllable' }; 
          },
          set: async function() {
            throw new Error('privacy.websites not controllable in Firefox');
          },
          clear: async function() {}
        },
        referrersEnabled: {
          get: async function() { 
            return { value: true, levelOfControl: 'not_controllable' }; 
          },
          set: async function() {
            throw new Error('privacy.websites not controllable in Firefox');
          },
          clear: async function() {}
        }
      }
    };
    
    if (typeof chrome !== 'undefined') chrome.privacy = privacyStub;
    if (typeof browser !== 'undefined') browser.privacy = privacyStub;
    
    console.info('💡 Use Firefox\'s about:preferences for privacy settings');
  }
})();
"#;
    
    NewFile {
        path: PathBuf::from("shims/privacy-stub.js"),
        content: content.to_string(),
        purpose: "Stubs chrome.privacy API which is not available in Firefox".to_string(),
    }
}

fn create_notifications_compat() -> NewFile {
    let content = r#"// Notifications API compatibility for extended features
// Firefox notifications have different capabilities than Chrome

(function() {
  'use strict';
  
  const api = typeof browser !== 'undefined' ? browser : chrome;
  
  if (api && api.notifications && api.notifications.create) {
    const originalCreate = api.notifications.create;
    
    api.notifications.create = async function(notificationId, options) {
      console.info('⚙️ Adapting notification options for Firefox');
      
      const adaptedOptions = { ...options };
      
      // Firefox doesn't support buttons in notifications
      if (adaptedOptions.buttons) {
        console.warn('⚠️ notifications: buttons are not supported in Firefox');
        console.info('💡 Button actions: ' + 
          adaptedOptions.buttons.map(b => b.title).join(', '));
        delete adaptedOptions.buttons;
      }
      
      // Firefox has limited imageUrl support
      if (adaptedOptions.imageUrl) {
        console.warn('⚠️ notifications: imageUrl support is limited in Firefox');
        // Keep it but be aware it might not display
      }
      
      // Firefox doesn't support appIconMaskUrl
      if (adaptedOptions.appIconMaskUrl) {
        console.warn('⚠️ notifications: appIconMaskUrl not supported, using iconUrl instead');
        if (!adaptedOptions.iconUrl) {
          adaptedOptions.iconUrl = adaptedOptions.appIconMaskUrl;
        }
        delete adaptedOptions.appIconMaskUrl;
      }
      
      // Firefox doesn't support progress
      if (adaptedOptions.progress !== undefined) {
        console.warn('⚠️ notifications: progress indicator not supported in Firefox');
        delete adaptedOptions.progress;
      }
      
      // Firefox doesn't support requireInteraction the same way
      if (adaptedOptions.requireInteraction) {
        console.info('⚙️ notifications: requireInteraction support varies in Firefox');
        // Keep it but results may differ
      }
      
      // Firefox doesn't support silent notifications
      if (adaptedOptions.silent) {
        console.warn('⚠️ notifications: silent option not supported in Firefox');
        delete adaptedOptions.silent;
      }
      
      return await originalCreate.call(this, notificationId, adaptedOptions);
    };
    
    console.info('✅ Notifications API compatibility loaded');
  }
})();
"#;
    
    NewFile {
        path: PathBuf::from("shims/notifications-compat.js"),
        content: content.to_string(),
        purpose: "Adapts Chrome notification options to Firefox capabilities".to_string(),
    }
}