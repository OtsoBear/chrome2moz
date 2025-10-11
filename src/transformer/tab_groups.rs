//! Transformer for chrome.tabGroups API (stub/polyfill)

use crate::models::chrome_only::*;
use crate::models::conversion::NewFile;
use anyhow::Result;
use std::path::PathBuf;

pub struct TabGroupsConverter;

impl TabGroupsConverter {
    pub fn new() -> Self {
        Self
    }

    /// Generate a stub for chrome.tabGroups that prevents crashes
    pub fn generate_stub(&self) -> Result<ChromeOnlyConversionResult> {
        let stub_content = r#"// Tab Groups Stub for Firefox
// Firefox doesn't support tab groups - this stub prevents crashes

'use strict';

console.warn('⚠️ chrome.tabGroups is not supported in Firefox');
console.info('💡 Firefox does not have native tab grouping functionality');

const TabGroupsStub = {
  query: async () => {
    console.warn('chrome.tabGroups.query() called - returning empty array');
    return [];
  },
  
  create: async (createProperties) => {
    console.warn('chrome.tabGroups.create() called - returning dummy group');
    return {
      id: -1,
      title: createProperties?.title || '',
      color: 'grey',
      collapsed: false
    };
  },
  
  update: async (groupId, updateProperties) => {
    console.warn('chrome.tabGroups.update() called - no-op');
    return { id: groupId, ...updateProperties };
  },
  
  get: async (groupId) => {
    console.warn('chrome.tabGroups.get() called - returning dummy group');
    return { id: groupId, title: '', color: 'grey', collapsed: false };
  },
  
  move: async (groupId) => {
    console.warn('chrome.tabGroups.move() called - no-op');
    return { id: groupId };
  },
  
  onCreated: {
    addListener: (callback) => {
      console.warn('chrome.tabGroups.onCreated.addListener() - will never fire');
    },
    removeListener: () => {},
    hasListener: () => false
  },
  
  onUpdated: {
    addListener: (callback) => {
      console.warn('chrome.tabGroups.onUpdated.addListener() - will never fire');
    },
    removeListener: () => {},
    hasListener: () => false
  },
  
  onRemoved: {
    addListener: (callback) => {
      console.warn('chrome.tabGroups.onRemoved.addListener() - will never fire');
    },
    removeListener: () => {},
    hasListener: () => false
  },
  
  onMoved: {
    addListener: (callback) => {
      console.warn('chrome.tabGroups.onMoved.addListener() - will never fire');
    },
    removeListener: () => {},
    hasListener: () => false
  }
};

// Attach stub to browser namespace
if (typeof browser !== 'undefined' && !browser.tabGroups) {
  browser.tabGroups = TabGroupsStub;
}

if (typeof chrome !== 'undefined' && !chrome.tabGroups) {
  chrome.tabGroups = TabGroupsStub;
}
"#;

        Ok(ChromeOnlyConversionResult {
            new_files: vec![NewFile {
                path: PathBuf::from("shims/tab-groups-stub.js"),
                content: stub_content.to_string(),
                purpose: "Prevents crashes from chrome.tabGroups calls (no functionality)".to_string(),
            }],
            modified_files: Vec::new(),
            manifest_changes: Vec::new(),
            removed_files: Vec::new(),
            instructions: vec![
                "Tab groups API stubbed to prevent crashes".to_string(),
                "⚠️ No tab grouping functionality - Firefox doesn't support this".to_string(),
                "Extension will run but tab group features won't work".to_string(),
            ],
        })
    }
}

impl Default for TabGroupsConverter {
    fn default() -> Self {
        Self::new()
    }
}