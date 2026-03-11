//! MIR extraction using global state (common pattern in rustc plugins)

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_driver::Callbacks;
use rustc_interface::interface;
use rustc_middle::ty::TyCtxt;
use std::path::PathBuf;
use std::sync::Mutex;

use aspect_driver::mir_analyzer::{AnalysisStats, MirAnalyzer};
use aspect_driver::types::FunctionMetadata;

// Global state to collect results
static EXTRACTED_FUNCTIONS: Mutex<Option<Vec<FunctionMetadata>>> = Mutex::new(None);
static VERBOSE: Mutex<bool> = Mutex::new(false);

struct GlobalStateCallbacks;

impl Callbacks for GlobalStateCallbacks {
    fn config(&mut self, config: &mut interface::Config) {
        let verbose = *VERBOSE.lock().unwrap();
        if verbose {
            println!("✓ Config callback invoked");
        }

        // Use override_queries to get TyCtxt access
        config.override_queries = Some(|_sess, providers| {
            // Save the original provider
            let orig_analysis = providers.analysis;

            // Override analysis query to run after type checking
            providers.analysis = |tcx, ()| {
                // Call original analysis first
                let result = orig_analysis(tcx, ());

                // Now we have TyCtxt - extract functions!
                extract_with_tcx(tcx);

                result
            };
        });
    }
}

// Function that runs with TyCtxt access
fn extract_with_tcx(tcx: TyCtxt<'_>) {
    let verbose = *VERBOSE.lock().unwrap();

    if verbose {
        println!("\n🎉 We have TyCtxt access!");
        println!("=== MIR Analysis Starting ===\n");
    }

    // Create MIR analyzer
    let analyzer = MirAnalyzer::new(tcx, verbose);

    // Extract all functions
    let functions = analyzer.extract_all_functions();

    if verbose {
        println!("\nExtracted {} functions", functions.len());
    }

    // Store in global state
    *EXTRACTED_FUNCTIONS.lock().unwrap() = Some(functions.clone());

    // Print statistics
    let stats = AnalysisStats::from_functions(&functions);
    stats.print_summary();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <file.rs> [--verbose]", args[0]);
        std::process::exit(1);
    }

    // Set verbose flag
    let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");
    *VERBOSE.lock().unwrap() = verbose;

    // Build rustc args
    let file_path = PathBuf::from(&args[1]);
    let rustc_args = vec![
        "rustc".to_string(),
        file_path.to_string_lossy().to_string(),
        "--crate-type".to_string(),
        "lib".to_string(),
        "--edition".to_string(),
        "2021".to_string(),
    ];

    println!("=== Global State MIR Extraction ===");
    println!("Analyzing: {}", file_path.display());
    if verbose {
        println!("Args: {:?}", rustc_args);
    }
    println!();

    // Run compiler with callbacks
    let mut callbacks = GlobalStateCallbacks;
    rustc_driver::RunCompiler::new(&rustc_args, &mut callbacks).run();

    // Retrieve results from global state
    if let Some(functions) = EXTRACTED_FUNCTIONS.lock().unwrap().as_ref() {
        println!("\n=== Extraction Results ===");
        println!("Total functions extracted: {}", functions.len());

        if verbose {
            println!("\nFunction details:");
            for func in functions {
                println!("  • {}", func.name);
                println!("    Visibility: {:?}", func.visibility);
                println!("    Module: {}", func.module_path);
                println!(
                    "    Location: {}:{}",
                    func.location.file, func.location.line
                );
            }
        }

        println!("\n✅ SUCCESS: MIR extraction complete!");
    } else {
        println!("\n⚠️  No functions extracted");
    }
}
