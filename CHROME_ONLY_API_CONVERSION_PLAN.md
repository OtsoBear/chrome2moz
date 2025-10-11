# Chrome-Only API Automatic Conversion Plan
## Goal: 95% Automatic Conversion Success Rate

### Priority APIs
1. **`chrome.offscreen.*`** (PRIMARY FOCUS) - Hidden document creation for DOM/Canvas/Audio
2. **`chrome.declarativeContent.*`** - Page action rules based on content
3. **`chrome.tabGroups.*`** - Simple stub to prevent crashes (low priority)

---

## Phase 1: chrome.offscreen → Firefox Alternatives

### 1.1 Detection & Analysis System

#### File: `src/analyzer/offscreen.rs`

```rust
pub struct OffscreenAnalyzer {
    source_dir: PathBuf,
    ast_parser: AstParser,
}

#[derive(Debug, Clone)]
pub struct OffscreenUsage {
    pub call_location: FileLocation,
    pub document_url: String,
    pub reasons: Vec<String>,
    pub justification: Option<String>,
    pub document_analysis: DocumentAnalysis,
}

#[derive(Debug, Clone)]
pub struct DocumentAnalysis {
    pub primary_purpose: OffscreenPurpose,
    pub secondary_purposes: Vec<OffscreenPurpose>,
    pub complexity_score: u8,  // 0-100
    pub dependencies: Vec<String>,
    pub dom_operations: Vec<DomOperation>,
    pub canvas_operations: Vec<CanvasOperation>,
    pub audio_operations: Vec<AudioOperation>,
    pub network_operations: Vec<NetworkOperation>,
    pub message_handlers: Vec<MessageHandler>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OffscreenPurpose {
    CanvasRendering,        // Canvas 2D/WebGL operations
    AudioProcessing,        // Web Audio API
    ImageProcessing,        // Image manipulation, OCR
    DomParsing,            // HTML parsing, scraping
    NetworkProxying,       // Fetch/XHR operations
    LibraryExecution,      // Running libraries needing DOM
    DataProcessing,        // Heavy computation
    CryptoOperations,      // Crypto libraries
    Mixed(Vec<OffscreenPurpose>),
}

impl OffscreenAnalyzer {
    /// Analyze an offscreen document to determine conversion strategy
    pub fn analyze_offscreen_document(&self, html_path: &str) -> Result<DocumentAnalysis> {
        let content = self.read_and_parse_html(html_path)?;
        let scripts = self.extract_all_scripts(&content)?;
        
        let mut analysis = DocumentAnalysis::default();
        
        // Analyze each script block
        for script in scripts {
            self.analyze_script_content(&script, &mut analysis)?;
        }
        
        // Determine primary purpose based on weighted scoring
        analysis.primary_purpose = self.determine_primary_purpose(&analysis);
        analysis.complexity_score = self.calculate_complexity(&analysis);
        
        Ok(analysis)
    }
    
    fn analyze_script_content(&self, script: &str, analysis: &mut DocumentAnalysis) -> Result<()> {
        // Use AST parsing for accurate detection
        let ast = self.ast_parser.parse(script)?;
        
        // Detect canvas operations
        for node in ast.find_calls(&["getContext", "canvas", "OffscreenCanvas"]) {
            analysis.canvas_operations.push(self.extract_canvas_op(&node)?);
        }
        
        // Detect audio operations
        for node in ast.find_calls(&["AudioContext", "createOscillator", "createGain"]) {
            analysis.audio_operations.push(self.extract_audio_op(&node)?);
        }
        
        // Detect DOM operations
        for node in ast.find_calls(&["querySelector", "createElement", "innerHTML"]) {
            analysis.dom_operations.push(self.extract_dom_op(&node)?);
        }
        
        // Detect network operations
        for node in ast.find_calls(&["fetch", "XMLHttpRequest", "axios"]) {
            analysis.network_operations.push(self.extract_network_op(&node)?);
        }
        
        // Detect message handlers
        for node in ast.find_listeners(&["chrome.runtime.onMessage", "addEventListener('message'"]) {
            analysis.message_handlers.push(self.extract_message_handler(&node)?);
        }
        
        // Detect library imports
        for node in ast.find_imports() {
            analysis.dependencies.push(node.module_name.clone());
        }
        
        Ok(())
    }
    
    fn determine_primary_purpose(&self, analysis: &DocumentAnalysis) -> OffscreenPurpose {
        let mut scores = HashMap::new();
        
        // Weight different operation types
        scores.insert(OffscreenPurpose::CanvasRendering, 
            analysis.canvas_operations.len() * 10);
        scores.insert(OffscreenPurpose::AudioProcessing, 
            analysis.audio_operations.len() * 10);
        scores.insert(OffscreenPurpose::DomParsing, 
            analysis.dom_operations.len() * 5);
        scores.insert(OffscreenPurpose::NetworkProxying, 
            analysis.network_operations.len() * 3);
        
        // Check for specific libraries
        for dep in &analysis.dependencies {
            if dep.contains("audio") || dep.contains("tone.js") {
                *scores.entry(OffscreenPurpose::AudioProcessing).or_insert(0) += 20;
            } else if dep.contains("fabric") || dep.contains("konva") || dep.contains("three") {
                *scores.entry(OffscreenPurpose::CanvasRendering).or_insert(0) += 20;
            } else if dep.contains("cheerio") || dep.contains("jsdom") {
                *scores.entry(OffscreenPurpose::DomParsing).or_insert(0) += 20;
            }
        }
        
        // If multiple high scores, return Mixed
        let high_scores: Vec<_> = scores.iter()
            .filter(|(_, &score)| score > 15)
            .collect();
        
        if high_scores.len() > 1 {
            let purposes = high_scores.iter().map(|(p, _)| (*p).clone()).collect();
            return OffscreenPurpose::Mixed(purposes);
        }
        
        // Return highest scoring purpose
        scores.into_iter()
            .max_by_key(|(_, score)| *score)
            .map(|(purpose, _)| purpose)
            .unwrap_or(OffscreenPurpose::LibraryExecution)
    }
    
    fn calculate_complexity(&self, analysis: &DocumentAnalysis) -> u8 {
        let mut score = 0u8;
        
        // More operations = higher complexity
        score += (analysis.canvas_operations.len() as u8).min(30);
        score += (analysis.dom_operations.len() as u8).min(30);
        score += (analysis.audio_operations.len() as u8).min(20);
        score += (analysis.dependencies.len() as u8 * 5).min(20);
        
        score.min(100)
    }
}
```

### 1.2 Conversion Strategy Selection

#### File: `src/transformer/offscreen_converter.rs`

```rust
pub struct OffscreenConverter {
    analyzer: OffscreenAnalyzer,
    user_preferences: ConversionPreferences,
}

#[derive(Debug, Clone)]
pub struct ConversionPreferences {
    pub prefer_workers: bool,
    pub inline_simple_ops: bool,
    pub create_polyfills: bool,
    pub verbosity: VerbosityLevel,
}

#[derive(Debug)]
pub enum ConversionStrategy {
    // Fully automatic conversions
    CanvasWorker {
        worker_path: PathBuf,
        transfer_canvas: bool,
    },
    AudioWorker {
        worker_path: PathBuf,
    },
    BackgroundIntegration {
        merge_into_background: bool,
    },
    ContentScript {
        target_urls: Vec<String>,
        all_urls: bool,
    },
    
    // Semi-automatic (needs user input)
    InteractiveContentScript {
        suggested_urls: Vec<String>,
    },
    SplitConversion {
        strategies: Vec<ConversionStrategy>,
    },
    
    // Fallback
    ManualGuidance {
        reason: String,
        suggestions: Vec<String>,
    },
}

impl OffscreenConverter {
    pub fn determine_strategy(&self, analysis: &DocumentAnalysis, usage: &OffscreenUsage) 
        -> ConversionStrategy {
        
        // Strategy decision tree
        match &analysis.primary_purpose {
            OffscreenPurpose::CanvasRendering if analysis.complexity_score < 70 => {
                // Automatic: Convert to Web Worker with OffscreenCanvas
                ConversionStrategy::CanvasWorker {
                    worker_path: PathBuf::from("workers/canvas-worker.js"),
                    transfer_canvas: true,
                }
            },
            
            OffscreenPurpose::AudioProcessing if analysis.complexity_score < 70 => {
                // Automatic: Convert to Audio Worklet or Web Worker
                ConversionStrategy::AudioWorker {
                    worker_path: PathBuf::from("workers/audio-worker.js"),
                }
            },
            
            OffscreenPurpose::NetworkProxying => {
                // Automatic: Move to background script
                ConversionStrategy::BackgroundIntegration {
                    merge_into_background: true,
                }
            },
            
            OffscreenPurpose::DomParsing if self.can_auto_determine_urls(analysis) => {
                // Automatic: Create content script with detected URLs
                ConversionStrategy::ContentScript {
                    target_urls: self.extract_target_urls(analysis),
                    all_urls: false,
                }
            },
            
            OffscreenPurpose::DomParsing => {
                // Semi-automatic: Need URL clarification
                ConversionStrategy::InteractiveContentScript {
                    suggested_urls: self.suggest_urls(analysis, usage),
                }
            },
            
            OffscreenPurpose::Mixed(purposes) if analysis.complexity_score < 80 => {
                // Automatic split conversion
                let strategies = purposes.iter()
                    .map(|p| self.strategy_for_purpose(p, analysis))
                    .collect();
                ConversionStrategy::SplitConversion { strategies }
            },
            
            _ => {
                // Manual guidance for complex cases
                ConversionStrategy::ManualGuidance {
                    reason: format!(
                        "Complex {} operation (complexity: {})",
                        analysis.primary_purpose.name(),
                        analysis.complexity_score
                    ),
                    suggestions: self.generate_manual_suggestions(analysis),
                }
            }
        }
    }
    
    fn can_auto_determine_urls(&self, analysis: &DocumentAnalysis) -> bool {
        // Check if we can extract URLs from the code
        for op in &analysis.dom_operations {
            if let Some(url_pattern) = &op.target_url {
                if url_pattern.is_static() {
                    return true;
                }
            }
        }
        false
    }
}
```

### 1.3 Automatic Conversion Implementations

#### 1.3.1 Canvas to Web Worker (Target: 25% of cases)

```rust
impl OffscreenConverter {
    pub fn convert_canvas_to_worker(&self, analysis: &DocumentAnalysis, usage: &OffscreenUsage) 
        -> Result<ConversionResult> {
        
        // Read offscreen document
        let offscreen_html = self.read_offscreen_document(&usage.document_url)?;
        
        // Extract canvas-related code
        let canvas_code = self.extract_canvas_code(&offscreen_html, analysis)?;
        
        // Generate Web Worker
        let worker_content = self.generate_canvas_worker(&canvas_code, analysis)?;
        
        // Generate main script changes
        let main_changes = self.generate_worker_integration(&usage.call_location)?;
        
        Ok(ConversionResult {
            new_files: vec![
                NewFile {
                    path: PathBuf::from("workers/canvas-worker.js"),
                    content: worker_content,
                    purpose: "Web Worker for canvas operations (converted from offscreen document)".into(),
                },
            ],
            modified_files: vec![main_changes],
            removed_files: vec![PathBuf::from(&usage.document_url)],
            instructions: vec![
                "Canvas operations moved to Web Worker with OffscreenCanvas".into(),
                "Uses transferControlToOffscreen() for zero-copy transfer".into(),
            ],
        })
    }
    
    fn generate_canvas_worker(&self, code: &str, analysis: &DocumentAnalysis) -> Result<String> {
        let mut worker = String::new();
        
        // Worker boilerplate
        worker.push_str(r#"
// Auto-generated Web Worker for canvas operations
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
      
"#);
        
        // Generate message handlers for each canvas operation
        for (idx, op) in analysis.canvas_operations.iter().enumerate() {
            worker.push_str(&format!(r#"
    case 'operation_{}':
      {}
      break;
"#, idx, self.adapt_canvas_operation(op)?));
        }
        
        worker.push_str(r#"
    default:
      console.error('Unknown message type:', type);
  }
});

"#);
        
        // Add the original code adapted for worker context
        worker.push_str("// Original offscreen code (adapted for worker)\n");
        worker.push_str(&self.adapt_code_for_worker(code)?);
        
        Ok(worker)
    }
    
    fn generate_worker_integration(&self, call_location: &FileLocation) -> Result<ModifiedFile> {
        let replacement = r#"
// Converted from chrome.offscreen.createDocument
const canvasWorker = new Worker('workers/canvas-worker.js');

// Get canvas and transfer control to worker
const canvas = document.getElementById('canvas') || 
               document.querySelector('canvas');
if (canvas) {
  const offscreen = canvas.transferControlToOffscreen();
  canvasWorker.postMessage({ 
    type: 'init', 
    canvas: offscreen 
  }, [offscreen]);
  
  canvasWorker.addEventListener('message', (event) => {
    if (event.data.type === 'ready') {
      console.log('Canvas worker ready');
      // Original code can now send operations to worker
    }
  });
}
"#;
        
        // Replace the chrome.offscreen.createDocument call
        Ok(ModifiedFile {
            path: call_location.file.clone(),
            changes: vec![
                FileChange {
                    line_number: call_location.line,
                    change_type: ChangeType::Replacement,
                    description: "Converted offscreen canvas to Web Worker".into(),
                    old_code: Some("chrome.offscreen.createDocument(...)".into()),
                    new_code: Some(replacement.into()),
                }
            ],
            original_content: String::new(), // Will be filled by caller
            new_content: String::new(),
        })
    }
}
```

#### 1.3.2 Audio to Web Worker (Target: 15% of cases)

```rust
impl OffscreenConverter {
    pub fn convert_audio_to_worker(&self, analysis: &DocumentAnalysis, usage: &OffscreenUsage)
        -> Result<ConversionResult> {
        
        let offscreen_html = self.read_offscreen_document(&usage.document_url)?;
        let audio_code = self.extract_audio_code(&offscreen_html, analysis)?;
        
        let worker_content = format!(r#"
// Auto-generated Audio Worker
// Converted from chrome.offscreen document

'use strict';

let audioContext = null;
let nodes = {{}};

self.addEventListener('message', async (event) => {{
  const {{ type, data }} = event.data;
  
  switch (type) {{
    case 'init':
      audioContext = new AudioContext();
      self.postMessage({{ type: 'ready' }});
      break;
      
    case 'play':
      // Original audio playback code
      {}
      break;
      
    case 'stop':
      if (audioContext) {{
        audioContext.close();
      }}
      break;
  }}
}});

// Original offscreen audio code (adapted)
{}
"#, self.adapt_audio_play_code(analysis)?, audio_code);
        
        Ok(ConversionResult {
            new_files: vec![
                NewFile {
                    path: PathBuf::from("workers/audio-worker.js"),
                    content: worker_content,
                    purpose: "Web Worker for audio processing (converted from offscreen)".into(),
                }
            ],
            modified_files: vec![self.generate_audio_worker_integration(&usage.call_location)?],
            removed_files: vec![PathBuf::from(&usage.document_url)],
            instructions: vec![
                "Audio operations moved to Web Worker".into(),
                "Web Audio API fully supported in workers".into(),
            ],
        })
    }
}
```

#### 1.3.3 Network to Background Script (Target: 20% of cases)

```rust
impl OffscreenConverter {
    pub fn convert_network_to_background(&self, analysis: &DocumentAnalysis, usage: &OffscreenUsage)
        -> Result<ConversionResult> {
        
        let offscreen_html = self.read_offscreen_document(&usage.document_url)?;
        let network_code = self.extract_network_code(&offscreen_html, analysis)?;
        
        // Background scripts already have fetch() access - just move the code
        let background_addition = format!(r#"
// Moved from offscreen document - network operations
// Firefox background scripts have full fetch() access

{}

// Message handler for requests
browser.runtime.onMessage.addListener((message, sender, sendResponse) => {{
  if (message.type === 'fetch_request') {{
    (async () => {{
      try {{
        const response = await fetch(message.url, message.options);
        const data = await response.json();
        sendResponse({{ success: true, data }});
      }} catch (error) {{
        sendResponse({{ success: false, error: error.message }});
      }}
    }})();
    return true; // Will respond asynchronously
  }}
}});
"#, network_code);
        
        Ok(ConversionResult {
            new_files: vec![],
            modified_files: vec![
                self.add_to_background_script(&background_addition)?,
                self.update_caller_for_messaging(&usage.call_location)?,
            ],
            removed_files: vec![PathBuf::from(&usage.document_url)],
            instructions: vec![
                "Network operations moved to background script".into(),
                "No offscreen document needed for fetch()".into(),
            ],
        })
    }
}
```

#### 1.3.4 DOM Parsing to Content Script (Target: 20% of cases, 15% auto + 5% semi-auto)

```rust
impl OffscreenConverter {
    pub fn convert_dom_to_content_script(&self, analysis: &DocumentAnalysis, usage: &OffscreenUsage)
        -> Result<ConversionResult> {
        
        let offscreen_html = self.read_offscreen_document(&usage.document_url)?;
        let dom_code = self.extract_dom_code(&offscreen_html, analysis)?;
        
        // Detect target URLs from code
        let target_urls = self.extract_or_prompt_for_urls(analysis, usage)?;
        
        let content_script = format!(r#"
// Auto-generated Content Script
// Converted from chrome.offscreen document for DOM parsing

'use strict';

// Original DOM parsing code (adapted for content script context)
{}

// Send results back to background
browser.runtime.sendMessage({{
  type: 'dom_parse_complete',
  data: extractedData
}});
"#, self.adapt_dom_code_for_content_script(dom_code)?);
        
        // Update manifest to include content script
        let manifest_update = ManifestChange::AddContentScript {
            matches: target_urls.clone(),
            js: vec!["content-scripts/dom-parser.js".into()],
            run_at: "document_idle".into(),
        };
        
        Ok(ConversionResult {
            new_files: vec![
                NewFile {
                    path: PathBuf::from("content-scripts/dom-parser.js"),
                    content: content_script,
                    purpose: "Content script for DOM parsing (converted from offscreen)".into(),
                }
            ],
            modified_files: vec![
                self.update_caller_for_content_script(&usage.call_location)?,
            ],
            manifest_changes: vec![manifest_update],
            removed_files: vec![PathBuf::from(&usage.document_url)],
            instructions: vec![
                "DOM parsing moved to content script".into(),
                format!("Content script will run on: {:?}", target_urls),
            ],
        })
    }
    
    fn extract_or_prompt_for_urls(&self, analysis: &DocumentAnalysis, usage: &OffscreenUsage)
        -> Result<Vec<String>> {
        
        // Try to extract URLs from code
        let mut detected_urls = Vec::new();
        
        for op in &analysis.dom_operations {
            if let Some(url) = &op.target_url {
                if url.is_static() {
                    detected_urls.push(url.as_pattern());
                }
            }
        }
        
        if !detected_urls.is_empty() {
            return Ok(detected_urls);
        }
        
        // Check for common patterns in justification or reasons
        if let Some(just) = &usage.justification {
            if just.contains("http") {
                // Extract URLs from justification text
                detected_urls.extend(self.extract_urls_from_text(just));
            }
        }
        
        if !detected_urls.is_empty() {
            return Ok(detected_urls);
        }
        
        // If still no URLs, use heuristics or prompt
        if self.user_preferences.prompt_for_urls {
            return Err(anyhow!("Interactive: Need URL patterns for content script"));
        }
        
        // Default to <all_urls> if analysis suggests general DOM parsing
        Ok(vec!["<all_urls>".into()])
    }
}
```

### 1.4 Advanced: Mixed-Purpose Conversion (Target: 10% of cases)

```rust
impl OffscreenConverter {
    pub fn convert_mixed_purpose(&self, analysis: &DocumentAnalysis, usage: &OffscreenUsage)
        -> Result<ConversionResult> {
        
        let OffscreenPurpose::Mixed(purposes) = &analysis.primary_purpose else {
            return Err(anyhow!("Expected mixed purpose"));
        };
        
        // Split the document into multiple conversion targets
        let mut results = Vec::new();
        
        for purpose in purposes {
            match purpose {
                OffscreenPurpose::CanvasRendering => {
                    // Extract only canvas operations
                    let canvas_analysis = self.filter_analysis_by_purpose(analysis, purpose);
                    results.push(self.convert_canvas_to_worker(&canvas_analysis, usage)?);
                },
                OffscreenPurpose::AudioProcessing => {
                    let audio_analysis = self.filter_analysis_by_purpose(analysis, purpose);
                    results.push(self.convert_audio_to_worker(&audio_analysis, usage)?);
                },
                OffscreenPurpose::NetworkProxying => {
                    let network_analysis = self.filter_analysis_by_purpose(analysis, purpose);
                    results.push(self.convert_network_to_background(&network_analysis, usage)?);
                },
                OffscreenPurpose::DomParsing => {
                    let dom_analysis = self.filter_analysis_by_purpose(analysis, purpose);
                    results.push(self.convert_dom_to_content_script(&dom_analysis, usage)?);
                },
                _ => {
                    // Skip or handle other purposes
                }
            }
        }
        
        // Merge all results
        self.merge_conversion_results(results)
    }
    
    fn filter_analysis_by_purpose(&self, analysis: &DocumentAnalysis, purpose: &OffscreenPurpose)
        -> DocumentAnalysis {
        
        let mut filtered = DocumentAnalysis::default();
        filtered.primary_purpose = purpose.clone();
        
        match purpose {
            OffscreenPurpose::CanvasRendering => {
                filtered.canvas_operations = analysis.canvas_operations.clone();
            },
            OffscreenPurpose::AudioProcessing => {
                filtered.audio_operations = analysis.audio_operations.clone();
            },
            OffscreenPurpose::NetworkProxying => {
                filtered.network_operations = analysis.network_operations.clone();
            },
            OffscreenPurpose::DomParsing => {
                filtered.dom_operations = analysis.dom_operations.clone();
            },
            _ => {}
        }
        
        filtered
    }
}
```

### 1.5 Edge Case Handling (Reaching 95%)

```rust
// File: src/transformer/offscreen_edge_cases.rs

impl OffscreenConverter {
    /// Handle library-heavy offscreen documents
    pub fn handle_library_execution(&self, analysis: &DocumentAnalysis) -> Result<ConversionResult> {
        // Check if libraries are worker-compatible
        let worker_compatible = analysis.dependencies.iter()
            .all(|dep| self.is_worker_compatible(dep));
        
        if worker_compatible {
            // Move to worker
            return self.convert_to_generic_worker(analysis);
        }
        
        // Some libraries need DOM - try content script
        let dom_needed = analysis.dependencies.iter()
            .any(|dep| self.requires_dom(dep));
        
        if dom_needed {
            return self.convert_to_content_script_with_libs(analysis);
        }
        
        // Fallback: inline into background if possible
        self.inline_into_background(analysis)
    }
    
    /// Handle image processing operations
    pub fn handle_image_processing(&self, analysis: &DocumentAnalysis) -> Result<ConversionResult> {
        // Image processing can often use OffscreenCanvas
        if analysis.canvas_operations.iter().any(|op| op.is_image_related()) {
            return self.convert_canvas_to_worker(analysis, &OffscreenUsage::default());
        }
        
        // Or use background script with ImageBitmap
        self.convert_to_background_with_imagebitmap(analysis)
    }
    
    /// Handle crypto operations
    pub fn handle_crypto_operations(&self, analysis: &DocumentAnalysis) -> Result<ConversionResult> {
        // Crypto can run in workers
        let worker_content = format!(r#"
// Crypto operations worker
'use strict';

self.addEventListener('message', async (event) => {{
  const {{ operation, data }} = event.data;
  
  try {{
    let result;
    switch (operation) {{
      case 'encrypt':
        result = await crypto.subtle.encrypt(data.algorithm, data.key, data.data);
        break;
      case 'decrypt':
        result = await crypto.subtle.decrypt(data.algorithm, data.key, data.data);
        break;
      case 'hash':
        result = await crypto.subtle.digest(data.algorithm, data.data);
        break;
      default:
        throw new Error('Unknown operation: ' + operation);
    }}
    
    self.postMessage({{ success: true, result }});
  }} catch (error) {{
    self.postMessage({{ success: false, error: error.message }});
  }}
}});

// Original crypto code
{}
"#, self.extract_crypto_code(analysis)?);
        
        Ok(ConversionResult {
            new_files: vec![
                NewFile {
                    path: PathBuf::from("workers/crypto-worker.js"),
                    content: worker_content,
                    purpose: "Crypto operations worker".into(),
                }
            ],
            modified_files: vec![],
            removed_files: vec![],
            instructions: vec!["Crypto operations moved to Web Worker".into()],
        })
    }
    
    /// Smart fallback for truly complex cases
    pub fn generate_smart_fallback(&self, analysis: &DocumentAnalysis) -> ConversionResult {
        let suggestions = vec![
            format!(
                "This offscreen document is complex (score: {}/100)",
                analysis.complexity_score
            ),
            "Suggested approach:".into(),
        ];
        
        // Provide specific suggestions based on what we detected
        let mut specific_suggestions = Vec::new();
        
        if !analysis.canvas_operations.is_empty() {
            specific_suggestions.push(
                format!("• {} canvas operations → Consider Web Worker with OffscreenCanvas",
                    analysis.canvas_operations.len())
            );
        }
        
        if !analysis.audio_operations.is_empty() {
            specific_suggestions.push(
                format!("• {} audio operations → Consider Audio Worklet or Web Worker",
                    analysis.audio_operations.len())
            );
        }
        
        if !analysis.dom_operations.is_empty() {
            specific_suggestions.push(
                format!("• {} DOM operations → Consider content script on target URLs",
                    analysis.dom_operations.len())
            );
        }
        
        if !analysis.network_operations.is_empty() {
            specific_suggestions.push(
                format!("• {} network operations → Can move to background script",
                    analysis.network_operations.len())
            );
        }
        
        // Generate partial conversion code
        let partial_code = self.generate_partial_conversion_template(analysis);
        
        ConversionResult {
            new_files: vec![
                NewFile {
                    path: PathBuf::from("OFFSCREEN_CONVERSION_GUIDE.md"),
                    content: format!("# Offscreen Document Conversion Guide\n\n{}\n\n{}\n\n## Partial Implementation Template\n\n```javascript\n{}\n```",
                        suggestions.join("\n"),
                        specific_suggestions.join("\n"),
                        partial_code
                    ),
                    purpose: "Manual conversion guide for complex offscreen document".into(),
                }
            ],
            modified_files: vec![],
            removed_files: vec![],
            instructions: vec![
                "Complex offscreen document requires manual review".into(),
                "See OFFSCREEN_CONVERSION_GUIDE.md for detailed suggestions".into(),
                "Partial implementation template provided".into(),
            ],
        }
    }
}
```

---

## Phase 2: chrome.declarativeContent → Firefox Alternative

### 2.1 Analysis & Detection

```rust
// File: src/analyzer/declarative_content.rs

pub struct DeclarativeContentAnalyzer;

#[derive(Debug, Clone)]
pub struct DeclarativeContentRule {
    pub conditions: Vec<PageCondition>,
    pub actions: Vec<PageAction>,
    pub location: FileLocation,
}

#[derive(Debug, Clone)]
pub enum PageCondition {
    PageStateMatcher {
        page_url: UrlFilter,
        css: Option<Vec<String>>,
        is_bookmarked: Option<bool>,
    },
}

#[derive(Debug, Clone)]
pub enum PageAction {
    ShowPageAction,
    SetIcon { icon_path: String },
}

impl DeclarativeContentAnalyzer {
    pub fn analyze_usage(&self, source: &str) -> Result<Vec<DeclarativeContentRule>> {
        let ast = parse_javascript(source)?;
        let mut rules = Vec::new();
        
        // Find chrome.declarativeContent.onPageChanged.addRules calls
        for call in ast.find_calls(&["chrome.declarativeContent.onPageChanged.addRules"]) {
            let rule = self.extract_rule(&call)?;
            rules.push(rule);
        }
        
        Ok(rules)
    }
}
```

### 2.2 Automatic Conversion Strategy

**Target: 90% automatic** (most declarativeContent usage is simple pattern matching)

```rust
// File: src/transformer/declarative_content_converter.rs

pub struct DeclarativeContentConverter;

impl DeclarativeContentConverter {
    pub fn convert(&self, rules: &[DeclarativeContentRule]) -> Result<ConversionResult> {
        // Strategy: Convert to content scripts + messaging
        
        let mut content_script_matches = HashSet::new();
        let mut conditions_code = Vec::new();
        
        for rule in rules {
            // Extract URL patterns
            for condition in &rule.conditions {
                if let PageCondition::PageStateMatcher { page_url, css, .. } = condition {
                    content_script_matches.insert(page_url.to_match_pattern());
                    
                    // Generate condition checking code
                    let check_code = if let Some(selectors) = css {
                        format!(r#"
// Check page conditions
const elements = document.querySelectorAll('{}');
if (elements.length > 0) {{
  // Condition met - notify background
  browser.runtime.sendMessage({{
    type: 'page_condition_met',
    action: 'show_page_action',
    tabId: browser.runtime.getTabId?.() // Firefox specific
  }});
}}
"#, selectors.join(", "))
                    } else {
                        // Just URL matching - no content script needed
                        String::new()
                    };
                    
                    conditions_code.push(check_code);
                }
            }
        }
        
        // Generate content script
        let content_script = format!(r#"
// Auto-generated content script
// Converted from chrome.declarativeContent rules

'use strict';

(function() {{
  // Check conditions on page load
  function checkConditions() {{
    {}
  }}
  
  // Run on page load
  if (document.readyState === 'loading') {{
    document.addEventListener('DOMContentLoaded', checkConditions);
  }} else {{
    checkConditions();
  }}
  
  // Also check on DOM mutations (for dynamic content)
  const observer = new MutationObserver(checkConditions);
  observer.observe(document.body, {{
    childList: true,
    subtree: true
  }});
}})();
"#, conditions_code.join("\n\n"));
        
        // Generate background script handler
        let background_handler = r#"
// Auto-generated handler for declarativeContent conversion
browser.runtime.onMessage.addListener((message, sender) => {
  if (message.type === 'page_condition_met') {
    // Show page action for this tab
    if (sender.tab?.id) {
      browser.pageAction.show(sender.tab.id);
      
      // Set icon if specified
      if (message.iconPath) {
        browser.pageAction.setIcon({
          tabId: sender.tab.id,
          path: message.iconPath
        });
      }
    }
  }
});
"#;
        
        // Generate manifest changes
        let manifest_changes = vec![
            ManifestChange::AddContentScript {
                matches: content_script_matches.into_iter().collect(),
                js: vec!["content-scripts/page-condition-checker.js".into()],
                run_at: "document_idle".into(),
            },
            ManifestChange::AddPermission("pageAction".into()),
        ];
        
        Ok(ConversionResult {
            new_files: vec![
                NewFile {
                    path: PathBuf::from("content-scripts/page-condition-checker.js"),
                    content: content_script,
                    purpose: "Checks page conditions (converted from declarativeContent)".into(),
                }
            ],
            modified_files: vec![
                self.add_to_background(background_handler)?,
            ],
            manifest_changes,
            removed_files: vec![],
            instructions: vec![
                "declarativeContent rules converted to content script + messaging".into(),
                "Page action will be shown when conditions are met".into(),
                "Firefox requires explicit pageAction permission".into(),
            ],
        })
    }
}
```

### 2.3 Advanced: Complex Conditions (Target: 5% edge cases)

```rust
impl DeclarativeContentConverter {
    pub fn convert_complex_conditions(&self, rules: &[DeclarativeContentRule]) -> Result<ConversionResult> {
        // Handle complex regex patterns, multiple conditions, etc.
        
        let content_script = r#"
// Complex condition checker with caching
'use strict';

class ConditionChecker {
  constructor() {
    this.cache = new Map();
    this.checkInterval = null;
  }
  
  async checkAllConditions() {
    const results = [];
    
    // Check URL patterns
    const currentUrl = window.location.href;
    for (const pattern of this.urlPatterns) {
      if (this.matchesPattern(currentUrl, pattern)) {
        results.push({ type: 'url', pattern, matched: true });
      }
    }
    
    // Check CSS selectors
    for (const selector of this.cssSelectors) {
      const elements = document.querySelectorAll(selector);
      if (elements.length > 0) {
        results.push({ type: 'css', selector, count: elements.length });
      }
    }
    
    // Check custom conditions
    for (const condition of this.customConditions) {
      if (await this.evaluateCondition(condition)) {
        results.push({ type: 'custom', condition, matched: true });
      }
    }
    
    // Notify background if any condition met
    if (results.length > 0) {
      browser.runtime.sendMessage({
        type: 'conditions_met',
        results,
        url: currentUrl
      });
    }
  }
  
  matchesPattern(url, pattern) {
    // Convert Chrome URL pattern to regex
    const regex = new RegExp(
      pattern
        .replace(/\*/g, '.*')
        .replace(/\?/g, '\\?')
    );
    return regex.test(url);
  }
  
  async evaluateCondition(condition) {
    // Evaluate complex conditions
    try {
      return await eval(condition.expression);
    } catch (e) {
      console.error('Condition evaluation error:', e);
      return false;
    }
  }
  
  startMonitoring() {
    // Check on load
    this.checkAllConditions();
    
    // Check periodically for SPA changes
    this.checkInterval = setInterval(() => {
      this.checkAllConditions();
    }, 1000);
    
    // Check on history changes
    window.addEventListener('popstate', () => this.checkAllConditions());
    
    // Check on mutations
    const observer = new MutationObserver(() => this.checkAllConditions());
    observer.observe(document.body, {
      childList: true,
      subtree: true,
      attributes: true
    });
  }
}

const checker = new ConditionChecker();
checker.startMonitoring();
"#;
        
        Ok(ConversionResult {
            new_files: vec![
                NewFile {
                    path: PathBuf::from("content-scripts/advanced-condition-checker.js"),
                    content: content_script.into(),
                    purpose: "Advanced condition checking for complex declarativeContent rules".into(),
                }
            ],
            modified_files: vec![],
            removed_files: vec![],
            instructions: vec![
                "Complex declarativeContent rules converted with monitoring".into(),
                "Handles dynamic content and SPA navigation".into(),
            ],
        })
    }
}
```

---

## Phase 3: chrome.tabGroups → Simple Warning Stub

**Target: 100% automatic** (just prevent crashes, don't emulate functionality)

### Strategy: No-op Stub with Console Warnings

```rust
// File: src/transformer/tab_groups_converter.rs

pub struct TabGroupsConverter;

impl TabGroupsConverter {
    pub fn convert(&self) -> Result<ConversionResult> {
        // Simple stub that prevents crashes but doesn't emulate functionality
        
        let stub = r#"
// Tab Groups Stub for Firefox
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
  }
};

// Attach stub to browser namespace
if (!browser.tabGroups) {
  browser.tabGroups = TabGroupsStub;
}

if (typeof chrome !== 'undefined' && !chrome.tabGroups) {
  chrome.tabGroups = TabGroupsStub;
}
"#;
        
        Ok(ConversionResult {
            new_files: vec![
                NewFile {
                    path: PathBuf::from("shims/tab-groups-stub.js"),
                    content: stub.into(),
                    purpose: "Prevents crashes from chrome.tabGroups calls (no functionality)".into(),
                },
            ],
            modified_files: vec![],
            manifest_changes: vec![],
            removed_files: vec![],
            instructions: vec![
                "Tab groups API stubbed to prevent crashes".into(),
                "⚠️ No tab grouping functionality - Firefox doesn't support this".into(),
                "Extension will run but tab group features won't work".into(),
            ],
        })
    }
}
```

---

## Phase 4: Integration & Testing

### 4.1 Main Converter Integration

```rust
// File: src/transformer/chrome_only_converter.rs

pub struct ChromeOnlyApiConverter {
    offscreen: OffscreenConverter,
    declarative_content: DeclarativeContentConverter,
    tab_groups: TabGroupsConverter,
}

impl ChromeOnlyApiConverter {
    pub async fn convert_all(&self, context: &ConversionContext) -> Result<ConversionResult> {
        let mut all_results = Vec::new();
        
        // Convert chrome.offscreen
        let offscreen_usages = self.detect_offscreen_usage(context)?;
        for usage in offscreen_usages {
            let analysis = self.offscreen.analyzer.analyze_offscreen_document(&usage.document_url)?;
            let result = self.offscreen.convert(&analysis, &usage)?;
            all_results.push(result);
        }
        
        // Convert chrome.declarativeContent
        let declarative_rules = self.detect_declarative_content(context)?;
        if !declarative_rules.is_empty() {
            let result = self.declarative_content.convert(&declarative_rules)?;
            all_results.push(result);
        }
        
        // Convert chrome.tabGroups
        let tab_group_ops = self.detect_tab_groups_usage(context)?;
        if !tab_group_ops.is_empty() {
            let result = self.tab_groups.convert(&tab_group_ops)?;
            all_results.push(result);
        }
        
        // Merge all results
        self.merge_all_results(all_results)
    }
}
```

### 4.2 Success Metrics & Testing

```rust
// File: tests/chrome_only_conversion_tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_canvas_offscreen_conversion() {
        let offscreen_html = r#"
        <html>
        <body>
          <canvas id="offscreen-canvas"></canvas>
          <script>
            const canvas = document.getElementById('offscreen-canvas');
            const ctx = canvas.getContext('2d');
            ctx.fillRect(0, 0, 100, 100);
          </script>
        </body>
        </html>
        "#;
        
        let converter = OffscreenConverter::new();
        let result = converter.convert_canvas_document(offscreen_html).unwrap();
        
        assert!(result.new_files.iter().any(|f| f.path.to_str().unwrap().contains("canvas-worker")));
        assert_eq!(result.instructions.len() > 0, true);
    }
    
    #[test]
    fn test_declarative_content_conversion() {
        let chrome_code = r#"
        chrome.declarativeContent.onPageChanged.addRules([{
          conditions: [
            new chrome.declarativeContent.PageStateMatcher({
              pageUrl: { hostEquals: 'example.com' },
              css: ['video']
            })
          ],
          actions: [new chrome.declarativeContent.ShowPageAction()]
        }]);
        "#;
        
        let converter = DeclarativeContentConverter::new();
        let result = converter.convert_from_code(chrome_code).unwrap();
        
        assert!(result.new_files.iter().any(|f| f.path.to_str().unwrap().contains("content-script")));
    }
    
    #[test]
    fn test_tab_groups_polyfill() {
        let chrome_code = r#"
        const group = await chrome.tabGroups.create({ title: "My Group" });
        await chrome.tabs.group({ groupId: group.id, tabIds: [1, 2, 3] });
        "#;
        
        let converter = TabGroupsConverter::new();
        let result = converter.convert_from_code(chrome_code).unwrap();
        
        assert!(result.new_files.iter().any(|f| f.path.to_str().unwrap().contains("tab-groups-polyfill")));
    }
}
```

---

## Phase 5: Success Rate Optimization

### 5.1 Telemetry & Learning

```rust
// File: src/analytics/conversion_analytics.rs

pub struct ConversionAnalytics {
    successful_conversions: HashMap<String, u32>,
    failed_conversions: HashMap<String, Vec<String>>,
    user_feedback: Vec<FeedbackEntry>,
}

#[derive(Debug, Clone)]
pub struct FeedbackEntry {
    pub conversion_type: String,
    pub success: bool,
    pub manual_intervention_needed: bool,
    pub time_to_fix: Option<Duration>,
    pub notes: String,
}

impl ConversionAnalytics {
    pub fn track_conversion(&mut self, api: &str, success: bool, reason: Option<String>) {
        if success {
            *self.successful_conversions.entry(api.to_string()).or_insert(0) += 1;
        } else {
            self.failed_conversions
                .entry(api.to_string())
                .or_insert_with(Vec::new)
                .push(reason.unwrap_or_else(|| "Unknown".to_string()));
        }
    }
    
    pub fn get_success_rate(&self, api: &str) -> f64 {
        let successes = self.successful_conversions.get(api).copied().unwrap_or(0);
        let failures = self.failed_conversions.get(api).map(|v| v.len()).unwrap_or(0);
        
        if successes + failures as u32 == 0 {
            return 0.0;
        }
        
        (successes as f64) / (successes + failures as u32) as f64 * 100.0
    }
    
    pub fn generate_improvement_report(&self) -> String {
        let mut report = String::from("# Conversion Success Report\n\n");
        
        for (api, _) in &self.successful_conversions {
            let rate = self.get_success_rate(api);
            report.push_str(&format!("## {}\n", api));
            report.push_str(&format!("Success Rate: {:.1}%\n\n", rate));
            
            if let Some(failures) = self.failed_conversions.get(api) {
                report.push_str("Common failure reasons:\n");
                for reason in failures {
                    report.push_str(&format!("- {}\n", reason));
                }
            }
            report.push_str("\n");
        }
        
        report
    }
}
```

### 5.2 Continuous Improvement

```markdown
## Improvement Cycle

1. **Track Conversions**: Log every conversion attempt with metadata
2. **Analyze Failures**: Categorize why conversions failed
3. **Update Heuristics**: Improve detection patterns based on failures
4. **Expand Test Suite**: Add tests for failed cases once fixed
5. **Iterate**: Aim to increase success rate by 1-2% per iteration

## Target Breakdown for 95% Success

### chrome.offscreen (PRIMARY FOCUS - 80% of effort)
- Canvas operations: 98% (high confidence, uses OffscreenCanvas)
- Audio operations: 97% (well-defined Web Audio API in workers)
- Network operations: 99% (trivial to move to background)
- Simple DOM parsing: 92% (URL detection + content scripts)
- Complex DOM parsing: 85% (may need user input for URLs)
- Mixed purposes: 88% (automatic splitting logic)
- Edge cases: 82% (library compatibility checks)

**chrome.offscreen overall: ~95% automatic**

### chrome.declarativeContent (15% of effort)
- Simple rules: 95% (URL + CSS patterns)
- Complex conditions: 85% (multiple conditions, regex)

**chrome.declarativeContent overall: ~92% automatic**

### chrome.tabGroups (5% of effort)
- Simple stub: 100% (prevents crashes, no functionality)

**chrome.tabGroups: 100% automatic stub generation**

### Combined Success Rate
**Overall: ~95% with smart fallbacks and prioritized focus on chrome.offscreen**
```

---

## Implementation Roadmap

### Sprint 1: Foundation & Offscreen Analysis (Weeks 1-2)
- [ ] Implement `OffscreenAnalyzer` with AST parsing
- [ ] Create detection patterns for offscreen, declarativeContent, tabGroups
- [ ] Build conversion framework focusing on chrome.offscreen

### Sprint 2: Offscreen - Canvas & Audio (Weeks 3-4)
- [ ] Canvas → Worker conversion (98% target)
- [ ] Audio → Worker conversion (95% target)
- [ ] Network → Background conversion (99% target)
- [ ] Test with real extensions

### Sprint 3: Offscreen - DOM & Mixed (Weeks 5-6)
- [ ] DOM → Content script (85-90% target)
- [ ] Mixed-purpose splitting logic
- [ ] Edge case handlers (crypto, image processing, etc.)
- [ ] Comprehensive offscreen testing

### Sprint 4: DeclarativeContent & TabGroups (Week 7)
- [ ] DeclarativeContent rule analysis and conversion
- [ ] TabGroups simple stub (no-op with warnings)
- [ ] Integration testing

### Sprint 5: Polish & Testing (Weeks 8-9)
- [ ] Integration testing with real extensions
- [ ] Analytics system for tracking success
- [ ] Documentation and examples
- [ ] User guidance for manual cases

### Sprint 6: Optimization & Validation (Weeks 10-11)
- [ ] Failure analysis and heuristic improvements
- [ ] Success rate validation
- [ ] Final testing to confirm 95%+ for offscreen
- [ ] Release candidate

---

## Success Criteria

### Automatic Conversion Rate
- **95% or higher** for common use cases
- Clear guidance for the remaining 5%
- Zero silent failures

### Quality Metrics
- All conversions compile without errors
- Generated code passes Firefox WebExtension validation
- Comprehensive test coverage (>90%)
- Real-world extension compatibility testing

### User Experience
- Clear progress indication
- Detailed reports on what was converted
- Actionable guidance for manual steps
- Migration guides for complex cases

---

## Maintenance & Updates

### Monitoring
- Track conversion success rates
- Collect user feedback
- Monitor new Chrome APIs
- Update detection patterns

### Updates
- Quarterly review of MDN compatibility data
- Add new API conversions as needed
- Improve heuristics based on failures
- Expand test suite continuously