//! Interactive CLI mode for Chrome to Firefox Extension Converter

use anyhow::Result;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Input, Select, Confirm};
use std::path::PathBuf;
use std::fs;
use crate::{convert_extension, ConversionOptions, CalculatorType};

/// Scan for extensions in the current directory and subdirectories
fn find_nearby_extensions() -> Vec<PathBuf> {
    let mut extensions = Vec::new();
    
    // Check current directory
    if PathBuf::from("./manifest.json").exists() {
        extensions.push(PathBuf::from("."));
    }
    
    // Check immediate subdirectories
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let manifest_path = entry.path().join("manifest.json");
                    if manifest_path.exists() {
                        extensions.push(entry.path());
                    }
                }
            }
        }
    }
    
    extensions
}

/// Prompt user to select an extension path
fn prompt_for_extension_path(prompt_text: &str) -> Result<PathBuf> {
    let nearby = find_nearby_extensions();
    
    if nearby.is_empty() {
        // No extensions found, just ask for path
        let input_path: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt_text)
            .with_initial_text("./")
            .interact_text()?;
        Ok(PathBuf::from(input_path))
    } else {
        // Show detected extensions + custom option
        let mut options: Vec<String> = nearby.iter()
            .map(|p| {
                let display = p.display().to_string();
                format!("📁 {} (detected)", display)
            })
            .collect();
        
        options.push("✏️  Enter custom path".to_string());
        
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt_text)
            .items(&options)
            .default(0)
            .interact()?;
        
        if selection < nearby.len() {
            Ok(nearby[selection].clone())
        } else {
            // Custom path selected
            let input_path: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter extension path")
                .with_initial_text("./")
                .interact_text()?;
            Ok(PathBuf::from(input_path))
        }
    }
}

/// Run the interactive CLI mode
pub fn run_interactive_mode() -> Result<()> {
    print_banner();
    
    loop {
        println!();
        let options = vec![
            "🔄 Convert Chrome Extension to Firefox",
            "📊 Analyze Chrome Extension",
            "🔍 List Chrome-Only APIs",
            "⌨️  Check Keyboard Shortcuts",
            "❌ Exit",
        ];
        
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do?")
            .items(&options)
            .default(0)
            .interact()?;
        
        match selection {
            0 => handle_convert()?,
            1 => handle_analyze()?,
            2 => handle_chrome_only_apis()?,
            3 => handle_check_shortcuts()?,
            4 => {
                println!("\n{}", "Thanks for using Chrome to Firefox Converter! 👋".green().bold());
                break;
            }
            _ => unreachable!(),
        }
    }
    
    Ok(())
}

fn print_banner() {
    println!("{}", "╔═══════════════════════════════════════════════════════════════╗".blue());
    println!("{}", "║                                                               ║".blue());
    println!("{}", "║     🦊 Chrome to Firefox Extension Converter                 ║".blue().bold());
    println!("{}", "║                                                               ║".blue());
    println!("{}", "║     Convert Chrome MV3 extensions to Firefox-compatible      ║".blue());
    println!("{}", "║     format automatically with API conversions and shims      ║".blue());
    println!("{}", "║                                                               ║".blue());
    println!("{}", "╚═══════════════════════════════════════════════════════════════╝".blue());
}

fn handle_convert() -> Result<()> {
    println!("\n{}", "=== Convert Chrome Extension to Firefox ===".blue().bold());
    println!();
    
    // Get input path with auto-detection
    let input = prompt_for_extension_path("📁 Select Chrome extension to convert")?;
    
    // Check if input exists
    if !input.exists() {
        println!("{}", "❌ Error: Input path does not exist!".red().bold());
        return Ok(());
    }
    
    // Get output path
    let output_path: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("📂 Enter the output directory path")
        .with_initial_text("./output")
        .interact_text()?;
    
    let output = PathBuf::from(output_path);
    
    // Ask about options
    let preserve_chrome = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("🔧 Preserve Chrome compatibility? (keep both chrome and browser namespaces)")
        .default(true)
        .interact()?;
    
    let generate_report = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("📄 Generate detailed conversion report?")
        .default(true)
        .interact()?;
    
    println!();
    println!("{}", "🚀 Starting conversion...".yellow().bold());
    println!();
    
    let options = ConversionOptions {
        interactive: false, // We're already in interactive mode
        target_calculator: CalculatorType::Both,
        preserve_chrome_compatibility: preserve_chrome,
        generate_report,
    };
    
    match convert_extension(&input, &output, options) {
        Ok(result) => {
            println!("{}", "✅ Conversion completed successfully!".green().bold());
            println!();
            
            // Print full report in console
            println!("{}", "═".repeat(70).blue());
            println!("{}", "📊 CONVERSION REPORT".blue().bold());
            println!("{}", "═".repeat(70).blue());
            println!();
            
            println!("{}", "EXTENSION INFO".bold());
            println!("  Name: {} v{}",
                result.report.summary.extension_name,
                result.report.summary.extension_version);
            println!("  Files Modified: {}", result.modified_files.len());
            println!("  Files Added: {} (compatibility shims)", result.new_files.len());
            println!("  Total Changes: {}", result.report.summary.total_changes);
            println!("  Chrome API Calls Converted: {}", result.report.summary.chrome_api_calls_converted);
            println!();
            
            // Detailed file changes with line-by-line breakdown
            if !result.modified_files.is_empty() {
                println!("{}", "MODIFIED FILES & CHANGES".bold());
                println!("   (All changes listed below have been automatically applied)");
                println!();
                for modified in &result.modified_files {
                    println!("  📄 {}", modified.path.display().to_string().cyan());
                    println!("     {} changes made", modified.changes.len());
                    
                    // Show all changes for this file
                    for change in &modified.changes {
                        let desc = &change.description;
                        println!("     • Line {}: {}",
                            change.line_number.to_string().yellow(),
                            desc);
                        
                        // Add clarification for callback warnings
                        if desc.contains("consider converting to promise") {
                            println!("       {} This callback was converted from chrome.* to browser.*", "ℹ️".dimmed());
                            println!("       {} Optional: You can manually convert to promise style for better Firefox compatibility", "💡".dimmed());
                        }
                    }
                    println!();
                }
            }
            
            // Added shims
            if !result.new_files.is_empty() {
                println!("{}", "ADDED COMPATIBILITY SHIMS".bold());
                for new_file in &result.new_files {
                    println!("  ✨ {}", new_file.path.display());
                    println!("      → {}", new_file.purpose.dimmed());
                }
                println!();
            }
            
            // Warnings with detailed explanations
            if !result.report.warnings.is_empty() {
                println!("{}", "⚠️  WARNINGS & WHAT THEY MEAN".yellow().bold());
                for warning in &result.report.warnings {
                    println!("  • {}", warning);
                    
                    // Provide context for specific warnings
                    if warning.contains("service worker") {
                        println!("    {}", "ℹ️  What this means - Service Worker → Event Page:".dimmed());
                        println!();
                        println!("    {}", "   CHROME (Service Worker):".dimmed());
                        println!("    {}", "   • Can be terminated at ANY time by browser".dimmed());
                        println!("    {}", "   • NO access to DOM or localStorage".dimmed());
                        println!("    {}", "   • Must use chrome.storage API for persistence".dimmed());
                        println!("    {}", "   • Restarts on API events (like messages, alarms)".dimmed());
                        println!();
                        println!("    {}", "   FIREFOX (Event Page):".dimmed());
                        println!("    {}", "   • Stays loaded longer, terminated after ~30s idle".dimmed());
                        println!("    {}", "   • CAN access DOM and use limited localStorage".dimmed());
                        println!("    {}", "   • Better suited for persistent listeners".dimmed());
                        println!("    {}", "   • Reloads on extension startup or API events".dimmed());
                        println!();
                        println!("    {}", "   WHAT COULD BREAK:".yellow());
                        println!("    {}", "   ❌ Assumptions about always-running background script".dimmed());
                        println!();
                        println!("    {}", "   WHAT WORKS AUTOMATICALLY:".green());
                        println!("    {}", "   ✓ Global variables (auto-persisted!)".dimmed());
                        println!("    {}", "   ✓ Long timers (converted to browser.alarms)".dimmed());
                        println!("    {}", "   ✓ Event listeners (runtime.onMessage, tabs.onUpdated, etc.)".dimmed());
                        println!("    {}", "   ✓ chrome.storage for persisting data".dimmed());
                        println!("    {}", "   ✓ Message passing between scripts".dimmed());
                        println!();
                        println!("    {}", "   📦 GLOBAL VARIABLE PERSISTENCE:".cyan());
                        println!("    {}", "   • Auto-detects and persists global variables".dimmed());
                        println!("    {}", "   • Uses browser.storage.local for persistence".dimmed());
                        println!();
                        println!("    {}", "   ⏰ LONG TIMER CONVERSION:".cyan());
                        println!("    {}", "   • setTimeout/setInterval >30s → browser.alarms".dimmed());
                        println!("    {}", "   • Generates alarm listeners automatically".dimmed());
                        println!();
                        println!("    {}", "   ✓ ACTION: Verify data persists and timers work after restarts".cyan());
                    } else if warning.contains("extension ID") || warning.contains("default extension ID") {
                        println!("    {}", "ℹ️  What this means:".dimmed());
                        println!("    {}", "   Firefox requires a unique extension ID for AMO submission.".dimmed());
                        println!("    {}", "   The generated ID uses email format: name@domain".dimmed());
                        println!("    {}", "   ✓ What to do: If publishing to AMO, customize this ID in:".cyan());
                        println!("    {}", "      manifest.json → browser_specific_settings.gecko.id".cyan());
                    }
                    println!();
                }
            }
            
            // Manual actions
            if !result.report.manual_actions.is_empty() {
                println!("{}", "🔧 MANUAL REVIEW REQUIRED".red().bold());
                println!("   The following items need your attention:");
                for action in &result.report.manual_actions {
                    println!("  • {}", action);
                }
                println!();
            }
            
            // Blockers
            if !result.report.blockers.is_empty() {
                println!("{}", "🛑 BLOCKING ISSUES".red().bold());
                println!("   These MUST be addressed before the extension will work:");
                for blocker in &result.report.blockers {
                    println!("  • {}", blocker);
                }
                println!();
            }
            
            // Save detailed report to file
            if generate_report {
                let report_path = output.with_extension("md");
                if let Ok(report_content) = crate::report::generate_report(&result) {
                    if std::fs::write(&report_path, report_content).is_ok() {
                        println!("{}", "📄 Detailed markdown report saved:".bold());
                        println!("   {}", report_path.display());
                        println!();
                    }
                }
            }
            
            // Next steps
            println!("{}", "═".repeat(70).blue());
            println!("{}", "🚀 NEXT STEPS".cyan().bold());
            println!("{}", "═".repeat(70).blue());
            println!();
            println!("{} Review the output directory:", "1.".bold());
            println!("   {}", output.display());
            println!();
            println!("{} Test in Firefox:", "2.".bold());
            println!("   • Open Firefox");
            println!("   • Go to: about:debugging#/runtime/this-firefox");
            println!("   • Click 'Load Temporary Add-on'");
            println!("   • Select: {}/manifest.json", output.display());
            println!();
            println!("{} Verify functionality:", "3.".bold());
            println!("   • Test all major features of your extension");
            println!("   • Open Browser Console (Ctrl+Shift+J / Cmd+Shift+J)");
            println!("   • Check for any errors or warnings");
            println!("   • Verify permissions work correctly");
            println!();
            
            if !result.report.warnings.is_empty() || !result.report.manual_actions.is_empty() {
                println!("{} Address warnings and manual actions:", "4.".bold());
                println!("   • Review items listed above");
                println!("   • Test affected functionality thoroughly");
                println!();
            }
            
            println!("{}", "💡 UNDERSTANDING THE CONVERSION:".cyan().bold());
            println!();
            println!("   {} What was automatically converted:", "✅".bold());
            println!("      • All chrome.* calls → browser.*");
            println!("      • Service worker → event page");
            println!("      • executeScript → message passing");
            println!("      • Added compatibility shims");
            println!();
            println!("   {} What \"consider converting to promise\" means:", "📖".bold());
            println!("      • The API call WAS converted (chrome.* → browser.*)");
            println!("      • It currently uses callbacks (works but not ideal)");
            println!();
            println!("      {} Callback vs Promise Support:", "💡".dimmed());
            println!("         • Chrome: Supports BOTH callbacks AND promises");
            println!("         • Firefox: browser.* API returns promises natively");
            println!("         • Callbacks work via webextension-polyfill compatibility layer");
            println!("         • Promises are MORE reliable and the preferred Firefox style");
            println!();
            println!("      {} Why convert to promises:", "✓".cyan());
            println!("         • Better error handling with try/catch");
            println!("         • Cleaner code with async/await");
            println!("         • Native Firefox API behavior");
            println!("         • Avoids polyfill overhead");
            println!();
            println!("      Example: .get('key', callback) → .get('key').then(...)");
            println!();
            println!("   {} Troubleshooting:", "🔧".bold());
            println!("      • Open Browser Console (Ctrl+Shift+J / Cmd+Shift+J)");
            println!("      • Most issues show clear error messages there");
            println!("      • Check the detailed markdown report for more info");
            
            // Pause before returning to menu
            println!();
            Input::<String>::new()
                .with_prompt("Press Enter to continue")
                .allow_empty(true)
                .interact_text()?;
        }
        Err(e) => {
            println!("{}", "❌ Conversion failed!".red().bold());
            println!("{}", format!("Error: {}", e).red());
            println!();
            Input::<String>::new()
                .with_prompt("Press Enter to continue")
                .allow_empty(true)
                .interact_text()?;
        }
    }
    
    Ok(())
}

fn handle_analyze() -> Result<()> {
    println!("\n{}", "=== Analyze Chrome Extension ===".blue().bold());
    println!();
    
    // Get input path with auto-detection
    let input = prompt_for_extension_path("📁 Select Chrome extension to analyze")?;
    
    // Check if input exists
    if !input.exists() {
        println!("{}", "❌ Error: Input path does not exist!".red().bold());
        return Ok(());
    }
    
    println!();
    println!("{}", "🔍 Analyzing extension...".yellow().bold());
    println!();
    
    match crate::packager::load_extension(&input) {
        Ok(extension) => {
            match crate::analyze_extension(extension) {
                Ok(context) => {
                    println!("{}", "📊 Analysis Results".bold().blue());
                    println!("{}", "═".repeat(60).blue());
                    println!();
                    
                    println!("📦 {}: {} v{}", 
                        "Extension".bold(),
                        context.source.metadata.name,
                        context.source.metadata.version);
                    println!("📋 Manifest Version: {}", context.source.metadata.manifest_version);
                    println!("📄 Files: {}", context.source.metadata.file_count);
                    println!("💾 Size: {} bytes", context.source.metadata.size_bytes);
                    println!();
                    
                    // Count auto-fixable issues upfront for the summary
                    let auto_fixable = context.incompatibilities.iter()
                        .filter(|i| i.auto_fixable)
                        .count();
                    
                    // Group by severity
                    let blockers: Vec<_> = context.incompatibilities.iter()
                        .filter(|i| matches!(i.severity, crate::models::Severity::Blocker))
                        .collect();
                    let majors: Vec<_> = context.incompatibilities.iter()
                        .filter(|i| matches!(i.severity, crate::models::Severity::Major))
                        .collect();
                    let minors: Vec<_> = context.incompatibilities.iter()
                        .filter(|i| matches!(i.severity, crate::models::Severity::Minor))
                        .collect();
                    let infos: Vec<_> = context.incompatibilities.iter()
                        .filter(|i| matches!(i.severity, crate::models::Severity::Info))
                        .collect();
                    
                    if context.incompatibilities.is_empty() {
                        println!("{}", "✅ No incompatibilities found!".green().bold());
                        println!("   This extension should work well in Firefox.");
                    } else {
                        println!("{}", format!("Found {} incompatibilities:",
                            context.incompatibilities.len()).yellow());
                        println!();
                        
                        if !blockers.is_empty() {
                            println!("{}", "🛑 BLOCKERS:".red().bold());
                            for issue in &blockers {
                                println!("  [{}] {}", issue.location, issue.description);
                                if let Some(suggestion) = &issue.suggestion {
                                    println!("    💡 {}", suggestion.dimmed());
                                }
                            }
                            println!();
                        }
                        
                        if !majors.is_empty() {
                            println!("{}", "⚠️  MAJOR ISSUES:".yellow().bold());
                            for issue in &majors {
                                println!("  [{}] {}", issue.location, issue.description);
                                if issue.auto_fixable {
                                    println!("    ✨ Auto-fixable");
                                } else if let Some(suggestion) = &issue.suggestion {
                                    println!("    💡 {}", suggestion.dimmed());
                                }
                            }
                            println!();
                        }
                        
                        if !minors.is_empty() {
                            println!("{}", "ℹ️  MINOR ISSUES:".blue().bold());
                            for issue in &minors {
                                println!("  [{}] {}", issue.location, issue.description);
                                if issue.auto_fixable {
                                    println!("    ✨ Auto-fixable");
                                }
                            }
                            println!();
                        }
                        
                        if !infos.is_empty() {
                            println!("{}", "💡 INFO:".white().bold());
                            for issue in &infos {
                                println!("  [{}] {}", issue.location, issue.description);
                            }
                            println!();
                        }
                    }
                    
                    if !context.decisions.is_empty() {
                        println!("{}", "❓ Decisions needed during conversion:".bold());
                        for decision in &context.decisions {
                            println!("  • {}", decision.question);
                        }
                        println!();
                    }
                    
                    // Summary
                    println!("{}", "📈 Conversion Outlook:".cyan().bold());
                    if context.incompatibilities.is_empty() {
                        println!("  ✅ Excellent - Ready for conversion");
                    } else if blockers.is_empty() {
                        println!("  ✅ Good - {} issues, {} auto-fixable",
                            context.incompatibilities.len(), auto_fixable);
                    } else {
                        println!("  ⚠️  {} blockers need manual attention", blockers.len());
                    }
                }
                Err(e) => {
                    println!("{}", "❌ Analysis failed!".red().bold());
                    println!("{}", format!("Error: {}", e).red());
                }
            }
        }
        Err(e) => {
            println!("{}", "❌ Failed to load extension!".red().bold());
            println!("{}", format!("Error: {}", e).red());
        }
    }
    
    println!();
    Input::<String>::new()
        .with_prompt("Press Enter to continue")
        .allow_empty(true)
        .interact_text()?;
    
    Ok(())
}

fn handle_chrome_only_apis() -> Result<()> {
    println!("\n{}", "=== Chrome-Only APIs ===".blue().bold());
    println!();
    println!("This will fetch the list of WebExtension APIs that exist in Chrome but not Firefox.");
    println!("The report highlights which APIs already have shims or detection in this tool.");
    println!();
    
    let proceed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Continue? (requires internet connection)")
        .default(true)
        .interact()?;
    
    if !proceed {
        return Ok(());
    }
    
    println!();
    println!("{}", "🌐 Fetching Chrome-only APIs from MDN...".yellow().bold());
    println!();
    
    let runtime = tokio::runtime::Runtime::new()?;
    match runtime.block_on(crate::scripts::fetch_chrome_only_apis::run()) {
        Ok(_) => {
            println!();
            Input::<String>::new()
                .with_prompt("Press Enter to continue")
                .allow_empty(true)
                .interact_text()?;
        }
        Err(e) => {
            println!("{}", "❌ Failed to fetch API list".red().bold());
            println!("{}", format!("Error: {}", e).red());
            println!();
            Input::<String>::new()
                .with_prompt("Press Enter to continue")
                .allow_empty(true)
                .interact_text()?;
        }
    }
    
    Ok(())
}

fn handle_check_shortcuts() -> Result<()> {
    println!("\n{}", "=== Check Keyboard Shortcuts ===".blue().bold());
    println!();
    println!("This will check for potential keyboard shortcut conflicts with Firefox.");
    println!();
    
    let proceed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Continue? (requires internet connection)")
        .default(true)
        .interact()?;
    
    if !proceed {
        return Ok(());
    }
    
    println!();
    println!("{}", "⌨️  Checking keyboard shortcuts...".yellow().bold());
    println!();
    
    let runtime = tokio::runtime::Runtime::new()?;
    let current_dir = std::env::current_dir().ok();
    let project_path = current_dir.as_deref();
    
    match runtime.block_on(crate::scripts::check_keyboard_shortcuts::run_with_project_path(project_path)) {
        Ok(_) => {
            println!();
            Input::<String>::new()
                .with_prompt("Press Enter to continue")
                .allow_empty(true)
                .interact_text()?;
        }
        Err(e) => {
            println!("{}", "❌ Failed to check keyboard shortcuts".red().bold());
            println!("{}", format!("Error: {}", e).red());
            println!();
            Input::<String>::new()
                .with_prompt("Press Enter to continue")
                .allow_empty(true)
                .interact_text()?;
        }
    }
    
    Ok(())
}