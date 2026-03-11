//! Working MIR extraction using rustc_interface::run_compiler
//!
//! This example uses a simpler approach that should work with recent nightly

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_session;
extern crate rustc_span;

use rustc_driver::Compilation;
use rustc_interface::interface;
use rustc_session::config;
use std::path::PathBuf;
use std::process::Command;

use aspect_driver::mir_analyzer::{AnalysisStats, MirAnalyzer};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <file.rs>", args[0]);
        std::process::exit(1);
    }

    let file_path = PathBuf::from(&args[1]);

    println!("=== Working MIR Extraction ===");
    println!("Analyzing: {}", file_path.display());
    println!();

    // Use the standard rustc args approach
    let mut rustc_args = vec![
        "rustc".to_string(),
        file_path.to_string_lossy().to_string(),
        "--crate-type".to_string(),
        "lib".to_string(),
        "--edition".to_string(),
        "2021".to_string(),
    ];

    // Create a simple callback that just prints when called
    struct ExtractionCallbacks {
        verbose: bool,
    }

    impl rustc_driver::Callbacks for ExtractionCallbacks {
        fn config(&mut self, config: &mut interface::Config) {
            if self.verbose {
                println!("✓ Compiler config callback invoked");
            }

            // Set up for analysis
            config.override_queries = Some(|_sess, providers| {
                // Store original queries
                let prev_mir_built = providers.mir_built;

                // Override mir_built query to intercept MIR
                providers.mir_built = |tcx, def_id| {
                    // Call original
                    let result = prev_mir_built(tcx, def_id);

                    // We have access to tcx here!
                    if self.verbose {
                        println!("  MIR query for: {:?}", def_id);
                    }

                    result
                };
            });
        }
    }

    let mut callbacks = ExtractionCallbacks { verbose: true };

    println!("Running rustc with callbacks...");
    rustc_driver::RunCompiler::new(&rustc_args, &mut callbacks).run();

    println!("\n=== Extraction Complete ===");
}
