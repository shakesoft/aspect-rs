//! MIR extraction using function pointers for query providers

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;

use rustc_driver::Callbacks;
use rustc_interface::interface;
use rustc_middle::ty::TyCtxt;
use std::path::PathBuf;
use std::sync::Mutex;

use aspect_driver::mir_analyzer::{AnalysisStats, MirAnalyzer};
use aspect_driver::types::FunctionMetadata;

// Global state
static RESULTS: Mutex<Option<Vec<FunctionMetadata>>> = Mutex::new(None);

// Function that will be called after analysis with TyCtxt
fn analyze_crate(tcx: TyCtxt<'_>, (): ()) {
    println!("\n🎉 analyze_crate called with TyCtxt!");
    println!("=== MIR Analysis ===\n");

    // Create MIR analyzer
    let analyzer = MirAnalyzer::new(tcx, true);

    // Extract all functions
    let functions = analyzer.extract_all_functions();

    println!("\nExtracted {} functions from MIR", functions.len());

    // Store results
    *RESULTS.lock().unwrap() = Some(functions.clone());

    // Print statistics
    let stats = AnalysisStats::from_functions(&functions);
    stats.print_summary();
}

struct AnalysisCallbacks;

impl Callbacks for AnalysisCallbacks {
    fn config(&mut self, config: &mut interface::Config) {
        println!("✓ Config callback");

        // Override the analysis query provider
        config.override_queries = Some(|_sess, providers| {
            // Replace the analysis provider with our function
            providers.analysis = analyze_crate;
        });
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <file.rs>", args[0]);
        std::process::exit(1);
    }

    let file_path = PathBuf::from(&args[1]);
    let rustc_args = vec![
        "rustc".to_string(),
        file_path.to_string_lossy().to_string(),
        "--crate-type".to_string(),
        "lib".to_string(),
        "--edition".to_string(),
        "2021".to_string(),
    ];

    println!("=== Function Pointer MIR Extraction ===");
    println!("Analyzing: {}", file_path.display());
    println!();

    // Run compiler
    let mut callbacks = AnalysisCallbacks;
    rustc_driver::RunCompiler::new(&rustc_args, &mut callbacks).run();

    // Show results
    if let Some(functions) = RESULTS.lock().unwrap().as_ref() {
        println!("\n=== Final Results ===");
        println!("✅ Successfully extracted {} functions", functions.len());

        if !functions.is_empty() {
            println!("\nFunctions:");
            for func in functions {
                println!("  • {} ({:?})", func.name, func.visibility);
            }
        }
    }
}
