//! Analyzer for chrome.offscreen API usage
//! Uses regex-based detection for simplicity (assumes pre-compiled JS)

use crate::models::chrome_only::*;
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

pub struct OffscreenAnalyzer {
    source_dir: PathBuf,
}

impl OffscreenAnalyzer {
    pub fn new(source_dir: PathBuf) -> Self {
        Self {
            source_dir,
        }
    }

    /// Detect chrome.offscreen API usage in JavaScript code using regex
    pub fn detect_usage(&self, code: &str, file_path: &Path) -> Result<Vec<OffscreenUsage>> {
        let mut usages = Vec::new();
        
        // Look for chrome.offscreen.createDocument calls (regex-based)
        if code.contains("chrome.offscreen.createDocument") ||
           code.contains("browser.offscreen.createDocument") {
            // Extract details from the code
            let lines: Vec<&str> = code.lines().collect();
            for (line_num, line) in lines.iter().enumerate() {
                if line.contains("offscreen.createDocument") {
                    usages.push(OffscreenUsage {
                        call_location: FileLocation::new(
                            file_path.to_path_buf(),
                            line_num + 1,
                            0,
                        ),
                        document_url: self.extract_document_url(line).unwrap_or_default(),
                        reasons: self.extract_reasons(line),
                        justification: self.extract_justification(line),
                    });
                }
            }
        }
        
        Ok(usages)
    }

    /// Analyze an offscreen document to determine conversion strategy
    pub fn analyze_offscreen_document(&self, html_path: &str) -> Result<DocumentAnalysis> {
        let full_path = self.source_dir.join(html_path);
        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| anyhow!("Failed to read {}: {}", html_path, e))?;
        
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

    fn extract_all_scripts(&self, html_content: &str) -> Result<Vec<String>> {
        let mut scripts = Vec::new();
        
        // Simple extraction - look for <script> tags
        let mut current_script = String::new();
        let mut in_script = false;
        
        for line in html_content.lines() {
            if line.contains("<script") {
                in_script = true;
                current_script.clear();
                continue;
            }
            if line.contains("</script>") {
                in_script = false;
                if !current_script.is_empty() {
                    scripts.push(current_script.clone());
                }
                continue;
            }
            if in_script {
                current_script.push_str(line);
                current_script.push('\n');
            }
        }
        
        Ok(scripts)
    }

    fn analyze_script_content(&self, script: &str, analysis: &mut DocumentAnalysis) -> Result<()> {
        // Regex-based detection (simple pattern matching)
        
        // Detect canvas operations
        if script.contains("getContext") || script.contains("canvas") || script.contains("OffscreenCanvas") {
            analysis.canvas_operations.push(CanvasOperation {
                operation_type: "canvas_usage".to_string(),
                context_type: self.detect_context_type(script),
            });
        }
        
        // Detect audio operations
        if script.contains("AudioContext") || script.contains("createOscillator") || script.contains("createGain") {
            analysis.audio_operations.push(AudioOperation {
                operation_type: "audio_usage".to_string(),
            });
        }
        
        // Detect DOM operations
        if script.contains("querySelector") || script.contains("createElement") || script.contains("innerHTML") {
            analysis.dom_operations.push(DomOperation {
                operation_type: "dom_manipulation".to_string(),
                target_url: None,
                selector: None,
            });
        }
        
        // Detect network operations
        if script.contains("fetch") || script.contains("XMLHttpRequest") || script.contains("axios") {
            analysis.network_operations.push(NetworkOperation {
                operation_type: "network_request".to_string(),
                target_url: None,
            });
        }
        
        // Detect message handlers
        if script.contains("chrome.runtime.onMessage") || script.contains("addEventListener('message'") {
            analysis.message_handlers.push(MessageHandler {
                handler_type: "message_listener".to_string(),
                message_type: None,
            });
        }
        
        // Detect library imports (regex-based)
        for dep in self.detect_dependencies(script) {
            if !analysis.dependencies.contains(&dep) {
                analysis.dependencies.push(dep);
            }
        }
        
        Ok(())
    }

    fn determine_primary_purpose(&self, analysis: &DocumentAnalysis) -> OffscreenPurpose {
        let mut scores: HashMap<String, usize> = HashMap::new();
        
        // Weight different operation types
        scores.insert("canvas".to_string(), analysis.canvas_operations.len() * 10);
        scores.insert("audio".to_string(), analysis.audio_operations.len() * 10);
        scores.insert("dom".to_string(), analysis.dom_operations.len() * 5);
        scores.insert("network".to_string(), analysis.network_operations.len() * 3);
        
        // Check for specific libraries
        for dep in &analysis.dependencies {
            if dep.contains("audio") || dep.contains("tone.js") {
                *scores.entry("audio".to_string()).or_insert(0) += 20;
            } else if dep.contains("fabric") || dep.contains("konva") || dep.contains("three") {
                *scores.entry("canvas".to_string()).or_insert(0) += 20;
            } else if dep.contains("cheerio") || dep.contains("jsdom") {
                *scores.entry("dom".to_string()).or_insert(0) += 20;
            }
        }
        
        // If multiple high scores, return Mixed
        let high_scores: Vec<_> = scores.iter()
            .filter(|(_, &score)| score > 15)
            .collect();
        
        if high_scores.len() > 1 {
            let mut purposes = Vec::new();
            for (key, _) in high_scores {
                purposes.push(Box::new(match key.as_str() {
                    "canvas" => OffscreenPurpose::CanvasRendering,
                    "audio" => OffscreenPurpose::AudioProcessing,
                    "dom" => OffscreenPurpose::DomParsing,
                    "network" => OffscreenPurpose::NetworkProxying,
                    _ => OffscreenPurpose::Unknown,
                }));
            }
            return OffscreenPurpose::Mixed(purposes);
        }
        
        // Return highest scoring purpose
        scores.into_iter()
            .max_by_key(|(_, score)| *score)
            .map(|(purpose, _)| match purpose.as_str() {
                "canvas" => OffscreenPurpose::CanvasRendering,
                "audio" => OffscreenPurpose::AudioProcessing,
                "dom" => OffscreenPurpose::DomParsing,
                "network" => OffscreenPurpose::NetworkProxying,
                _ => OffscreenPurpose::LibraryExecution,
            })
            .unwrap_or(OffscreenPurpose::Unknown)
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

    // Helper methods
    
    fn extract_document_url(&self, line: &str) -> Option<String> {
        // Simple extraction - look for url: 'path' or url: "path"
        if let Some(start) = line.find("url:") {
            let after_url = &line[start + 4..];
            if let Some(quote_start) = after_url.find(|c| c == '\'' || c == '"') {
                let quote_char = after_url.chars().nth(quote_start).unwrap();
                let content = &after_url[quote_start + 1..];
                if let Some(quote_end) = content.find(quote_char) {
                    return Some(content[..quote_end].to_string());
                }
            }
        }
        None
    }

    fn extract_reasons(&self, line: &str) -> Vec<String> {
        let mut reasons = Vec::new();
        
        // Look for reasons array
        if line.contains("reasons:") {
            if line.contains("AUDIO_PLAYBACK") {
                reasons.push("AUDIO_PLAYBACK".to_string());
            }
            if line.contains("DOM_SCRAPING") {
                reasons.push("DOM_SCRAPING".to_string());
            }
            if line.contains("WORKERS") {
                reasons.push("WORKERS".to_string());
            }
        }
        
        reasons
    }

    fn extract_justification(&self, line: &str) -> Option<String> {
        // Look for justification field
        if let Some(start) = line.find("justification:") {
            let after_just = &line[start + 14..];
            if let Some(quote_start) = after_just.find(|c| c == '\'' || c == '"') {
                let quote_char = after_just.chars().nth(quote_start).unwrap();
                let content = &after_just[quote_start + 1..];
                if let Some(quote_end) = content.find(quote_char) {
                    return Some(content[..quote_end].to_string());
                }
            }
        }
        None
    }

    fn detect_context_type(&self, script: &str) -> Option<String> {
        if script.contains("'2d'") || script.contains("\"2d\"") {
            Some("2d".to_string())
        } else if script.contains("'webgl'") || script.contains("\"webgl\"") {
            Some("webgl".to_string())
        } else {
            None
        }
    }

    fn detect_dependencies(&self, script: &str) -> Vec<String> {
        let mut deps = Vec::new();
        
        // Look for common library patterns
        let libraries = vec![
            "three.js", "fabric.js", "konva", "tone.js",
            "cheerio", "jsdom", "axios", "lodash",
        ];
        
        for lib in libraries {
            if script.contains(lib) {
                deps.push(lib.to_string());
            }
        }
        
        deps
    }
}