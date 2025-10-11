//! Tests for Chrome-only API conversion system

use chrome2moz::models::chrome_only::*;
use chrome2moz::models::conversion::NewFile;
use chrome2moz::transformer::{
    OffscreenConverter, DeclarativeContentConverter, TabGroupsConverter,
};
use chrome2moz::analyzer::{OffscreenAnalyzer, DeclarativeContentAnalyzer};
use std::path::PathBuf;

#[test]
fn test_offscreen_purpose_names() {
    assert_eq!(OffscreenPurpose::CanvasRendering.name(), "Canvas Rendering");
    assert_eq!(OffscreenPurpose::AudioProcessing.name(), "Audio Processing");
    assert_eq!(OffscreenPurpose::DomParsing.name(), "DOM Parsing");
    assert_eq!(OffscreenPurpose::NetworkProxying.name(), "Network Proxying");
    assert_eq!(OffscreenPurpose::LibraryExecution.name(), "Library Execution");
}

#[test]
fn test_document_analysis_default() {
    let analysis = DocumentAnalysis::default();
    assert_eq!(analysis.complexity_score, 0);
    assert!(analysis.canvas_operations.is_empty());
    assert!(analysis.audio_operations.is_empty());
    assert!(analysis.dom_operations.is_empty());
    assert!(analysis.network_operations.is_empty());
}

#[test]
fn test_conversion_preferences_default() {
    let prefs = ConversionPreferences::default();
    assert!(prefs.prefer_workers);
    assert!(!prefs.inline_simple_ops); // Default is false
    assert!(prefs.create_polyfills);
    assert!(prefs.prompt_for_urls);
}

#[test]
fn test_offscreen_converter_canvas_strategy() {
    let temp_dir = std::env::temp_dir();
    let prefs = ConversionPreferences::default();
    let converter = OffscreenConverter::new(temp_dir, prefs);
    
    let analysis = DocumentAnalysis {
        primary_purpose: OffscreenPurpose::CanvasRendering,
        secondary_purposes: vec![],
        complexity_score: 50,
        dependencies: vec![],
        canvas_operations: vec![],
        audio_operations: vec![],
        dom_operations: vec![],
        network_operations: vec![],
        message_handlers: vec![],
    };
    
    let usage = OffscreenUsage {
        call_location: FileLocation {
            file: PathBuf::from("test.js"),
            line: 10,
            column: 5,
        },
        document_url: "offscreen.html".to_string(),
        reasons: vec!["CANVAS".to_string()],
        justification: Some("Canvas rendering".to_string()),
    };
    
    let strategy = converter.determine_strategy(&analysis, &usage);
    
    match strategy {
        ConversionStrategy::CanvasWorker { worker_path, transfer_canvas } => {
            assert!(worker_path.to_str().unwrap().contains("canvas-worker.js"));
            assert!(transfer_canvas);
        }
        _ => panic!("Expected CanvasWorker strategy"),
    }
}

#[test]
fn test_offscreen_converter_audio_strategy() {
    let temp_dir = std::env::temp_dir();
    let prefs = ConversionPreferences::default();
    let converter = OffscreenConverter::new(temp_dir, prefs);
    
    let analysis = DocumentAnalysis {
        primary_purpose: OffscreenPurpose::AudioProcessing,
        secondary_purposes: vec![],
        complexity_score: 60,
        dependencies: vec![],
        canvas_operations: vec![],
        audio_operations: vec![],
        dom_operations: vec![],
        network_operations: vec![],
        message_handlers: vec![],
    };
    
    let usage = OffscreenUsage {
        call_location: FileLocation {
            file: PathBuf::from("test.js"),
            line: 10,
            column: 5,
        },
        document_url: "audio.html".to_string(),
        reasons: vec!["AUDIO_PLAYBACK".to_string()],
        justification: Some("Audio playback".to_string()),
    };
    
    let strategy = converter.determine_strategy(&analysis, &usage);
    
    match strategy {
        ConversionStrategy::AudioWorker { worker_path } => {
            assert!(worker_path.to_str().unwrap().contains("audio-worker.js"));
        }
        _ => panic!("Expected AudioWorker strategy"),
    }
}

#[test]
fn test_offscreen_converter_network_strategy() {
    let temp_dir = std::env::temp_dir();
    let prefs = ConversionPreferences::default();
    let converter = OffscreenConverter::new(temp_dir, prefs);
    
    let analysis = DocumentAnalysis {
        primary_purpose: OffscreenPurpose::NetworkProxying,
        secondary_purposes: vec![],
        complexity_score: 30,
        dependencies: vec![],
        canvas_operations: vec![],
        audio_operations: vec![],
        dom_operations: vec![],
        network_operations: vec![],
        message_handlers: vec![],
    };
    
    let usage = OffscreenUsage {
        call_location: FileLocation {
            file: PathBuf::from("test.js"),
            line: 10,
            column: 5,
        },
        document_url: "fetch.html".to_string(),
        reasons: vec!["FETCH".to_string()],
        justification: Some("Network requests".to_string()),
    };
    
    let strategy = converter.determine_strategy(&analysis, &usage);
    
    match strategy {
        ConversionStrategy::BackgroundIntegration { merge_into_background } => {
            assert!(merge_into_background);
        }
        _ => panic!("Expected BackgroundIntegration strategy"),
    }
}

#[test]
fn test_offscreen_canvas_conversion() {
    let temp_dir = std::env::temp_dir();
    let prefs = ConversionPreferences::default();
    let converter = OffscreenConverter::new(temp_dir, prefs);
    
    let analysis = DocumentAnalysis {
        primary_purpose: OffscreenPurpose::CanvasRendering,
        secondary_purposes: vec![],
        complexity_score: 50,
        dependencies: vec![],
        canvas_operations: vec![],
        audio_operations: vec![],
        dom_operations: vec![],
        network_operations: vec![],
        message_handlers: vec![],
    };
    
    let usage = OffscreenUsage {
        call_location: FileLocation {
            file: PathBuf::from("test.js"),
            line: 10,
            column: 5,
        },
        document_url: "offscreen.html".to_string(),
        reasons: vec!["CANVAS".to_string()],
        justification: Some("Canvas rendering".to_string()),
    };
    
    let result = converter.convert_canvas_to_worker(&analysis, &usage).unwrap();
    
    assert_eq!(result.new_files.len(), 1);
    assert!(result.new_files[0].path.to_str().unwrap().contains("canvas-worker.js"));
    assert!(result.new_files[0].content.contains("canvas")); // Check for canvas-related code
    assert!(result.modified_files.len() > 0);
    assert!(result.instructions.len() > 0);
}

#[test]
fn test_declarative_content_converter_simple() {
    let converter = DeclarativeContentConverter::new();
    
    let rules = vec![DeclarativeContentRule {
        conditions: vec![PageCondition::PageStateMatcher {
            page_url: UrlFilter {
                host_equals: Some("example.com".to_string()),
                host_contains: None,
                host_prefix: None,
                host_suffix: None,
                path_contains: None,
                path_equals: None,
                path_prefix: None,
                path_suffix: None,
                query_contains: None,
                query_equals: None,
                query_prefix: None,
                query_suffix: None,
                url_matches: None,
                schemes: None,
            },
            css: Some(vec!["video".to_string()]),
            is_bookmarked: None,
        }],
        actions: vec![PageAction::ShowPageAction],
        location: FileLocation {
            file: PathBuf::from("background.js"),
            line: 10,
            column: 5,
        },
    }];
    
    let result = converter.convert(&rules).unwrap();
    
    assert_eq!(result.new_files.len(), 2);
    assert!(result.new_files.iter().any(|f| f.path.to_str().unwrap().contains("page-condition-checker.js")));
    assert!(result.new_files.iter().any(|f| f.path.to_str().unwrap().contains("background_declarative_content_handler.js")));
    assert!(result.manifest_changes.len() >= 2);
    assert!(result.instructions.len() > 0);
}

#[test]
fn test_tab_groups_converter_stub() {
    let converter = TabGroupsConverter::new();
    let result = converter.generate_stub().unwrap();
    
    assert_eq!(result.new_files.len(), 1);
    assert!(result.new_files[0].path.to_str().unwrap().contains("tab-groups-stub.js"));
    assert!(result.new_files[0].content.contains("tabGroups"));
    assert!(result.new_files[0].content.contains("not supported in Firefox"));
    assert!(result.instructions.len() > 0);
}

#[test]
fn test_chrome_only_conversion_result_default() {
    let result = ChromeOnlyConversionResult::default();
    assert!(result.new_files.is_empty());
    assert!(result.modified_files.is_empty());
    assert!(result.manifest_changes.is_empty());
    assert!(result.removed_files.is_empty());
    assert!(result.instructions.is_empty());
}

#[test]
fn test_url_filter_to_match_pattern() {
    let filter = UrlFilter {
        host_equals: Some("example.com".to_string()),
        host_contains: None,
        host_prefix: None,
        host_suffix: None,
        path_contains: None,
        path_equals: None,
        path_prefix: None,
        path_suffix: None,
        query_contains: None,
        query_equals: None,
        query_prefix: None,
        query_suffix: None,
        url_matches: None,
        schemes: None,
    };
    
    let pattern = filter.to_match_pattern();
    assert!(pattern.contains("example.com"));
}

#[test]
fn test_manifest_change_variants() {
    let change1 = ManifestChange::AddContentScript {
        matches: vec!["*://example.com/*".to_string()],
        js: vec!["content.js".to_string()],
        run_at: "document_idle".to_string(),
    };
    
    let change2 = ManifestChange::AddPermission("storage".to_string());
    
    // Just verify they can be created
    match change1 {
        ManifestChange::AddContentScript { .. } => (),
        _ => panic!("Expected AddContentScript"),
    }
    
    match change2 {
        ManifestChange::AddPermission(_) => (),
        _ => panic!("Expected AddPermission"),
    }
}

#[test]
fn test_offscreen_analyzer_creation() {
    let temp_dir = std::env::temp_dir();
    let _analyzer = OffscreenAnalyzer::new(temp_dir);
    // Just verify it can be created
}

#[test]
fn test_declarative_content_analyzer_creation() {
    let _analyzer = DeclarativeContentAnalyzer::new();
    // Just verify it can be created
}

#[test]
fn test_conversion_result_merge() {
    let result1 = ChromeOnlyConversionResult {
        new_files: vec![NewFile {
            path: PathBuf::from("file1.js"),
            content: "content1".to_string(),
            purpose: "test".to_string(),
        }],
        modified_files: vec![],
        manifest_changes: vec![],
        removed_files: vec![],
        instructions: vec!["instruction1".to_string()],
    };
    
    let result2 = ChromeOnlyConversionResult {
        new_files: vec![NewFile {
            path: PathBuf::from("file2.js"),
            content: "content2".to_string(),
            purpose: "test2".to_string(),
        }],
        modified_files: vec![],
        manifest_changes: vec![],
        removed_files: vec![],
        instructions: vec!["instruction2".to_string()],
    };
    
    let merged = ChromeOnlyConversionResult {
        new_files: [result1.new_files, result2.new_files].concat(),
        modified_files: vec![],
        manifest_changes: vec![],
        removed_files: vec![],
        instructions: [result1.instructions, result2.instructions].concat(),
    };
    
    assert_eq!(merged.new_files.len(), 2);
    assert_eq!(merged.instructions.len(), 2);
}