//! Chrome to Firefox Extension Converter CLI

use chrome_to_firefox::{convert_extension, ConversionOptions, CalculatorType};
use chrome_to_firefox::scripts::fetch_chrome_only_apis;
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "chrome-to-firefox")]
#[command(about = "Convert Chrome MV3 extensions to Firefox-compatible MV3", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a Chrome extension to Firefox format
    Convert {
        /// Path to the Chrome extension (ZIP, CRX, or directory)
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output path for the converted extension
        #[arg(short, long)]
        output: PathBuf,
        
        /// Skip interactive prompts and use defaults
        #[arg(short = 'y', long)]
        yes: bool,
        
        /// Generate detailed conversion report
        #[arg(short, long)]
        report: bool,
        
        /// Preserve Chrome compatibility (keep both chrome and browser namespaces)
        #[arg(long)]
        preserve_chrome: bool,
    },
    
    /// Analyze an extension without converting
    Analyze {
        /// Path to the extension
        #[arg(short, long)]
        input: PathBuf,
    },

    /// List WebExtension APIs supported in Chrome but not Firefox
    ChromeOnlyApis,
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Convert { input, output, yes, report, preserve_chrome } => {
            println!("{}", "Chrome to Firefox Extension Converter".bold().blue());
            println!("{}", "=".repeat(50).blue());
            println!();
            
            let options = ConversionOptions {
                interactive: !yes,
                target_calculator: CalculatorType::Both,
                preserve_chrome_compatibility: preserve_chrome,
                generate_report: report,
            };
            
            match convert_extension(&input, &output, options) {
                Ok(result) => {
                    println!("{}", "✅ Conversion completed successfully!".green().bold());
                    println!();
                    println!("📊 Summary:");
                    println!("  - Files modified: {}", result.modified_files.len());
                    println!("  - Files added: {}", result.new_files.len());
                    println!("  - Output: {}", output.display());
                    
                    if report {
                        let report_path = output.with_extension("md");
                        if let Ok(report_content) = chrome_to_firefox::report::generate_report(&result) {
                            if std::fs::write(&report_path, report_content).is_ok() {
                                println!("  - Report: {}", report_path.display());
                            }
                        }
                    }
                    
                    if !result.report.warnings.is_empty() {
                        println!();
                        println!("{}", "⚠️  Warnings:".yellow().bold());
                        for warning in &result.report.warnings {
                            println!("  - {}", warning);
                        }
                    }
                    
                    if !result.report.manual_actions.is_empty() {
                        println!();
                        println!("{}", "📝 Manual actions required:".yellow().bold());
                        for action in &result.report.manual_actions {
                            println!("  - {}", action);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", "❌ Conversion failed!".red().bold());
                    eprintln!("{}", format!("Error: {}", e).red());
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Analyze { input } => {
            println!("{}", "Analyzing extension...".bold());
            println!();
            
            match chrome_to_firefox::packager::load_extension(&input) {
                Ok(extension) => {
                    match chrome_to_firefox::analyze_extension(extension) {
                        Ok(context) => {
                            println!("{}", "📊 Analysis Results".bold().blue());
                            println!("{}", "=".repeat(50).blue());
                            println!();
                            
                            println!("Extension: {} v{}", 
                                context.source.metadata.name,
                                context.source.metadata.version);
                            println!("Manifest Version: {}", context.source.metadata.manifest_version);
                            println!("Files: {}", context.source.metadata.file_count);
                            println!();
                            
                            if context.incompatibilities.is_empty() {
                                println!("{}", "✅ No incompatibilities found!".green());
                            } else {
                                println!("{}", format!("Found {} incompatibilities:", 
                                    context.incompatibilities.len()).yellow());
                                println!();
                                
                                for issue in &context.incompatibilities {
                                    let severity_str = match issue.severity {
                                        chrome_to_firefox::models::Severity::Blocker => "🛑 BLOCKER".red(),
                                        chrome_to_firefox::models::Severity::Major => "⚠️  MAJOR".yellow(),
                                        chrome_to_firefox::models::Severity::Minor => "ℹ️  MINOR".blue(),
                                        chrome_to_firefox::models::Severity::Info => "💡 INFO".white(),
                                    };
                                    
                                    println!("{} [{}]", severity_str, issue.location);
                                    println!("  {}", issue.description);
                                    if let Some(suggestion) = &issue.suggestion {
                                        println!("  💡 {}", suggestion.dimmed());
                                    }
                                    if issue.auto_fixable {
                                        println!("  {}", "✨ Auto-fixable".green());
                                    }
                                    println!();
                                }
                            }
                            
                            if !context.decisions.is_empty() {
                                println!("{}", "❓ Decisions needed:".bold());
                                for decision in &context.decisions {
                                    println!("  - {}", decision.question);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("{}", "❌ Analysis failed!".red().bold());
                            eprintln!("{}", format!("Error: {}", e).red());
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", "❌ Failed to load extension!".red().bold());
                    eprintln!("{}", format!("Error: {}", e).red());
                    std::process::exit(1);
                }
            }
        }

        Commands::ChromeOnlyApis => {
            println!(
                "{}",
                "Fetching Chrome-only WebExtension APIs".bold().blue()
            );
            println!();

            let runtime = tokio::runtime::Runtime::new()
                .expect("failed to initialize async runtime");

            if let Err(err) = runtime.block_on(fetch_chrome_only_apis::run()) {
                eprintln!("{}", "❌ Failed to fetch API list".red().bold());
                eprintln!("{}", format!("Error: {err}").red());
                std::process::exit(1);
            }
        }
    }
}