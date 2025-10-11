//! Converter for chrome.offscreen API to Firefox alternatives

use crate::models::chrome_only::*;
use crate::models::conversion::{NewFile, ModifiedFile, FileChange, ChangeType};
use crate::analyzer::OffscreenAnalyzer;
use anyhow::{Result, anyhow};
use std::path::PathBuf;

pub struct OffscreenConverter {
    _analyzer: OffscreenAnalyzer,
    preferences: ConversionPreferences,
}

impl OffscreenConverter {
    pub fn new(source_dir: PathBuf, preferences: ConversionPreferences) -> Self {
        Self {
            _analyzer: OffscreenAnalyzer::new(source_dir),
            preferences,
        }
    }

    /// Determine the conversion strategy based on document analysis
    pub fn determine_strategy(
        &self,
        analysis: &DocumentAnalysis,
        _usage: &OffscreenUsage,
    ) -> ConversionStrategy {
        match &analysis.primary_purpose {
            OffscreenPurpose::CanvasRendering if analysis.complexity_score < 70 => {
                ConversionStrategy::CanvasWorker {
                    worker_path: PathBuf::from("workers/canvas-worker.js"),
                    transfer_canvas: true,
                }
            }
            
            OffscreenPurpose::AudioProcessing if analysis.complexity_score < 70 => {
                ConversionStrategy::AudioWorker {
                    worker_path: PathBuf::from("workers/audio-worker.js"),
                }
            }
            
            OffscreenPurpose::NetworkProxying => {
                ConversionStrategy::BackgroundIntegration {
                    merge_into_background: true,
                }
            }
            
            OffscreenPurpose::DomParsing if self.can_auto_determine_urls(analysis) => {
                ConversionStrategy::ContentScript {
                    target_urls: self.extract_target_urls(analysis),
                    all_urls: false,
                }
            }
            
            OffscreenPurpose::DomParsing => {
                ConversionStrategy::InteractiveContentScript {
                    suggested_urls: self.suggest_urls(analysis),
                }
            }
            
            OffscreenPurpose::Mixed(purposes) if analysis.complexity_score < 80 => {
                let strategies = purposes
                    .iter()
                    .map(|p| Box::new(self.strategy_for_purpose(p, analysis)))
                    .collect();
                ConversionStrategy::SplitConversion { strategies }
            }
            
            _ => ConversionStrategy::ManualGuidance {
                reason: format!(
                    "Complex {} operation (complexity: {})",
                    analysis.primary_purpose.name(),
                    analysis.complexity_score
                ),
                suggestions: self.generate_manual_suggestions(analysis),
            },
        }
    }

    /// Convert canvas operations to Web Worker
    pub fn convert_canvas_to_worker(
        &self,
        _analysis: &DocumentAnalysis,
        usage: &OffscreenUsage,
    ) -> Result<ChromeOnlyConversionResult> {
        let worker_content = r#"// Auto-generated Web Worker for canvas operations
// Converted from chrome.offscreen document

'use strict';

let canvas = null;
let ctx = null;

self.addEventListener('message', async (event) => {
  const { type, data } = event.data;
  
  switch (type) {
    case 'init':
      canvas = event.data.canvas;
      ctx = canvas.getContext('2d');
      self.postMessage({ type: 'ready' });
      break;
      
    case 'render':
      // Perform canvas operations
      if (ctx && data) {
        // Example: draw based on data
        ctx.fillStyle = data.color || '#000';
        ctx.fillRect(data.x || 0, data.y || 0, data.width || 100, data.height || 100);
        self.postMessage({ type: 'render_complete' });
      }
      break;
      
    default:
      console.error('Unknown message type:', type);
  }
});
"#;

        let main_changes = ModifiedFile {
            path: usage.call_location.file.clone(),
            original_content: String::new(),
            new_content: String::new(),
            changes: vec![FileChange {
                line_number: usage.call_location.line,
                change_type: ChangeType::Modification,
                description: "Converted offscreen canvas to Web Worker".to_string(),
                old_code: Some("chrome.offscreen.createDocument(...)".to_string()),
                new_code: Some(
                    r#"const canvasWorker = new Worker('workers/canvas-worker.js');
const canvas = document.getElementById('canvas') || document.querySelector('canvas');
if (canvas) {
  const offscreen = canvas.transferControlToOffscreen();
  canvasWorker.postMessage({ type: 'init', canvas: offscreen }, [offscreen]);
}"#
                    .to_string(),
                ),
            }],
        };

        Ok(ChromeOnlyConversionResult {
            new_files: vec![NewFile {
                path: PathBuf::from("workers/canvas-worker.js"),
                content: worker_content.to_string(),
                purpose: "Web Worker for canvas operations (converted from offscreen document)"
                    .to_string(),
            }],
            modified_files: vec![main_changes],
            removed_files: vec![PathBuf::from(&usage.document_url)],
            instructions: vec![
                "Canvas operations moved to Web Worker with OffscreenCanvas".to_string(),
                "Uses transferControlToOffscreen() for zero-copy transfer".to_string(),
            ],
            manifest_changes: Vec::new(),
        })
    }

    /// Convert audio operations to Web Worker
    pub fn convert_audio_to_worker(
        &self,
        _analysis: &DocumentAnalysis,
        usage: &OffscreenUsage,
    ) -> Result<ChromeOnlyConversionResult> {
        let worker_content = r#"// Auto-generated Audio Worker
// Converted from chrome.offscreen document

'use strict';

let audioContext = null;
let nodes = {};

self.addEventListener('message', async (event) => {
  const { type, data } = event.data;
  
  switch (type) {
    case 'init':
      audioContext = new AudioContext();
      self.postMessage({ type: 'ready' });
      break;
      
    case 'play':
      if (audioContext && data.frequency) {
        const oscillator = audioContext.createOscillator();
        oscillator.frequency.value = data.frequency;
        oscillator.connect(audioContext.destination);
        oscillator.start();
        
        setTimeout(() => oscillator.stop(), data.duration || 1000);
        self.postMessage({ type: 'playing' });
      }
      break;
      
    case 'stop':
      if (audioContext) {
        audioContext.close();
      }
      break;
  }
});
"#;

        let main_changes = ModifiedFile {
            path: usage.call_location.file.clone(),
            original_content: String::new(),
            new_content: String::new(),
            changes: vec![FileChange {
                line_number: usage.call_location.line,
                change_type: ChangeType::Modification,
                description: "Converted offscreen audio to Web Worker".to_string(),
                old_code: Some("chrome.offscreen.createDocument(...)".to_string()),
                new_code: Some(
                    r#"const audioWorker = new Worker('workers/audio-worker.js');
audioWorker.postMessage({ type: 'init' });
audioWorker.addEventListener('message', (event) => {
  if (event.data.type === 'ready') {
    // Worker is ready to play audio
  }
});"#
                    .to_string(),
                ),
            }],
        };

        Ok(ChromeOnlyConversionResult {
            new_files: vec![NewFile {
                path: PathBuf::from("workers/audio-worker.js"),
                content: worker_content.to_string(),
                purpose: "Web Worker for audio processing (converted from offscreen)".to_string(),
            }],
            modified_files: vec![main_changes],
            removed_files: vec![PathBuf::from(&usage.document_url)],
            instructions: vec![
                "Audio operations moved to Web Worker".to_string(),
                "Web Audio API fully supported in workers".to_string(),
            ],
            manifest_changes: Vec::new(),
        })
    }

    /// Convert network operations to background script
    pub fn convert_network_to_background(
        &self,
        _analysis: &DocumentAnalysis,
        usage: &OffscreenUsage,
    ) -> Result<ChromeOnlyConversionResult> {
        let background_addition = r#"
// Moved from offscreen document - network operations
// Firefox background scripts have full fetch() access

browser.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'fetch_request') {
    (async () => {
      try {
        const response = await fetch(message.url, message.options);
        const data = await response.json();
        sendResponse({ success: true, data });
      } catch (error) {
        sendResponse({ success: false, error: error.message });
      }
    })();
    return true; // Will respond asynchronously
  }
});
"#;

        let main_changes = ModifiedFile {
            path: usage.call_location.file.clone(),
            original_content: String::new(),
            new_content: String::new(),
            changes: vec![FileChange {
                line_number: usage.call_location.line,
                change_type: ChangeType::Modification,
                description: "Converted offscreen network ops to background script".to_string(),
                old_code: Some("chrome.offscreen.createDocument(...)".to_string()),
                new_code: Some(
                    r#"// Network operations moved to background script
browser.runtime.sendMessage({
  type: 'fetch_request',
  url: 'https://api.example.com/data',
  options: {}
}).then(response => {
  if (response.success) {
    console.log('Data:', response.data);
  }
});"#
                    .to_string(),
                ),
            }],
        };

        Ok(ChromeOnlyConversionResult {
            new_files: vec![NewFile {
                path: PathBuf::from("background_network_addition.js"),
                content: background_addition.to_string(),
                purpose: "Background script addition for network operations".to_string(),
            }],
            modified_files: vec![main_changes],
            removed_files: vec![PathBuf::from(&usage.document_url)],
            instructions: vec![
                "Network operations moved to background script".to_string(),
                "No offscreen document needed for fetch()".to_string(),
                "Add the background_network_addition.js content to your background script"
                    .to_string(),
            ],
            manifest_changes: Vec::new(),
        })
    }

    /// Convert DOM parsing to content script
    pub fn convert_dom_to_content_script(
        &self,
        analysis: &DocumentAnalysis,
        usage: &OffscreenUsage,
    ) -> Result<ChromeOnlyConversionResult> {
        let target_urls = self.extract_or_prompt_for_urls(analysis)?;

        let content_script = r#"// Auto-generated Content Script
// Converted from chrome.offscreen document for DOM parsing

'use strict';

// DOM parsing code
function extractData() {
  const data = {
    title: document.title,
    headings: Array.from(document.querySelectorAll('h1, h2, h3')).map(h => h.textContent),
    links: Array.from(document.querySelectorAll('a')).map(a => a.href)
  };
  return data;
}

// Send results back to background
const extractedData = extractData();
browser.runtime.sendMessage({
  type: 'dom_parse_complete',
  data: extractedData
});
"#;

        let main_changes = ModifiedFile {
            path: usage.call_location.file.clone(),
            original_content: String::new(),
            new_content: String::new(),
            changes: vec![FileChange {
                line_number: usage.call_location.line,
                change_type: ChangeType::Modification,
                description: "Converted offscreen DOM parsing to content script".to_string(),
                old_code: Some("chrome.offscreen.createDocument(...)".to_string()),
                new_code: Some(
                    r#"// DOM parsing moved to content script
browser.runtime.onMessage.addListener((message, sender) => {
  if (message.type === 'dom_parse_complete') {
    console.log('Parsed data:', message.data);
  }
});"#
                    .to_string(),
                ),
            }],
        };

        Ok(ChromeOnlyConversionResult {
            new_files: vec![NewFile {
                path: PathBuf::from("content-scripts/dom-parser.js"),
                content: content_script.to_string(),
                purpose: "Content script for DOM parsing (converted from offscreen)".to_string(),
            }],
            modified_files: vec![main_changes],
            removed_files: vec![PathBuf::from(&usage.document_url)],
            instructions: vec![
                "DOM parsing moved to content script".to_string(),
                format!("Content script will run on: {:?}", target_urls),
                "Add content script to manifest.json".to_string(),
            ],
            manifest_changes: vec![ManifestChange::AddContentScript {
                matches: target_urls,
                js: vec!["content-scripts/dom-parser.js".to_string()],
                run_at: "document_idle".to_string(),
            }],
        })
    }

    // Helper methods

    fn can_auto_determine_urls(&self, analysis: &DocumentAnalysis) -> bool {
        analysis
            .dom_operations
            .iter()
            .any(|op| op.target_url.is_some())
    }

    fn extract_target_urls(&self, analysis: &DocumentAnalysis) -> Vec<String> {
        analysis
            .dom_operations
            .iter()
            .filter_map(|op| op.target_url.as_ref())
            .map(|url| {
                if url.starts_with("http") {
                    format!("*://{}/*", url.split('/').nth(2).unwrap_or("*"))
                } else {
                    "*://*/*".to_string()
                }
            })
            .collect()
    }

    fn suggest_urls(&self, _analysis: &DocumentAnalysis) -> Vec<String> {
        vec![
            "*://*.example.com/*".to_string(),
            "*://*/*".to_string(),
        ]
    }

    fn extract_or_prompt_for_urls(&self, analysis: &DocumentAnalysis) -> Result<Vec<String>> {
        let urls = self.extract_target_urls(analysis);
        if !urls.is_empty() {
            Ok(urls)
        } else if self.preferences.prompt_for_urls {
            Err(anyhow!("Interactive: Need URL patterns for content script"))
        } else {
            Ok(vec!["<all_urls>".to_string()])
        }
    }

    fn strategy_for_purpose(
        &self,
        purpose: &OffscreenPurpose,
        _analysis: &DocumentAnalysis,
    ) -> ConversionStrategy {
        match purpose {
            OffscreenPurpose::CanvasRendering => ConversionStrategy::CanvasWorker {
                worker_path: PathBuf::from("workers/canvas-worker.js"),
                transfer_canvas: true,
            },
            OffscreenPurpose::AudioProcessing => ConversionStrategy::AudioWorker {
                worker_path: PathBuf::from("workers/audio-worker.js"),
            },
            OffscreenPurpose::NetworkProxying => ConversionStrategy::BackgroundIntegration {
                merge_into_background: true,
            },
            _ => ConversionStrategy::ManualGuidance {
                reason: format!("Unsupported purpose: {}", purpose.name()),
                suggestions: vec!["Manual conversion required".to_string()],
            },
        }
    }

    fn generate_manual_suggestions(&self, analysis: &DocumentAnalysis) -> Vec<String> {
        let mut suggestions = vec![
            format!(
                "This offscreen document is complex (score: {}/100)",
                analysis.complexity_score
            ),
            "Suggested approach:".to_string(),
        ];

        if !analysis.canvas_operations.is_empty() {
            suggestions.push(format!(
                "• {} canvas operations → Consider Web Worker with OffscreenCanvas",
                analysis.canvas_operations.len()
            ));
        }

        if !analysis.audio_operations.is_empty() {
            suggestions.push(format!(
                "• {} audio operations → Consider Audio Worklet or Web Worker",
                analysis.audio_operations.len()
            ));
        }

        if !analysis.dom_operations.is_empty() {
            suggestions.push(format!(
                "• {} DOM operations → Consider content script on target URLs",
                analysis.dom_operations.len()
            ));
        }

        if !analysis.network_operations.is_empty() {
            suggestions.push(format!(
                "• {} network operations → Can move to background script",
                analysis.network_operations.len()
            ));
        }

        suggestions
    }
}