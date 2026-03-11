//! aspect-rustc-driver - Compiler driver for automatic aspect weaving
//!
//! This binary integrates with rustc through rustc_driver to enable
//! automatic aspect weaving based on pointcut expressions.
//!

#![feature(rustc_private)]
#![allow(unused_imports)]

// Import rustc internals
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_interface::interface;
use rustc_middle::ty::TyCtxt;
use rustc_session::config::ErrorOutputType;
use std::path::PathBuf;
use std::process::Command;

use aspect_driver::mir_analyzer::{AnalysisStats, MirAnalyzer};
use aspect_driver::types::FunctionMetadata;

/// Configuration for aspect weaving
#[derive(Debug, Clone)]
struct AspectConfig {
    /// Aspect pointcut expressions
    pointcuts: Vec<String>,

    /// Aspect names to apply
    aspects: Vec<String>,

    /// Enable verbose output
    verbose: bool,

    /// Output file for analysis results
    output_file: Option<PathBuf>,
}

impl Default for AspectConfig {
    fn default() -> Self {
        Self {
            pointcuts: Vec::new(),
            aspects: Vec::new(),
            verbose: false,
            output_file: None,
        }
    }
}

/// Compiler callbacks for aspect weaving
struct AspectCompilerCallbacks {
    config: AspectConfig,
    extracted_functions: Vec<FunctionMetadata>,
}

impl AspectCompilerCallbacks {
    fn new(config: AspectConfig) -> Self {
        Self {
            config,
            extracted_functions: Vec::new(),
        }
    }
}

impl Callbacks for AspectCompilerCallbacks {
    fn config(&mut self, _config: &mut interface::Config) {
        if self.config.verbose {
            println!("=== aspect-rustc-driver: Configuring compiler ===");
            println!("Pointcuts registered: {}", self.config.pointcuts.len());
        }
    }

    // Note: The after_analysis callback signature has changed in recent nightly versions
    // We need to research the correct method to access TyCtxt for MIR extraction
    //
    // Working approach demonstrated in examples/mir_extraction.rs:
    // - config() callback works ✓
    // - Need to find correct method for TyCtxt access
    //
    // Once we have TyCtxt access, the implementation will be:
    // 1. Create MirAnalyzer::new(tcx, verbose)
    // 2. Call analyzer.extract_all_functions()
    // 3. Apply pointcut matching
    // 4. Generate aspect weaving code
}

impl AspectCompilerCallbacks {
    // MIR analysis methods will be added once we have stable callback APIs
    //
    // The infrastructure is ready in aspect-driver/src/mir_analyzer.rs:
    // - MirAnalyzer::new(tcx, verbose)
    // - analyzer.extract_all_functions()
    // - AnalysisStats::from_functions()
    //
    // These will be called from the after_analysis callback when
    // rustc APIs stabilize or we pin to a specific compatible version.
}

fn main() {
    // Get command-line arguments
    let mut args: Vec<String> = std::env::args().collect();

    // Check for aspect-specific flags
    let mut config = AspectConfig::default();
    let mut rustc_args = Vec::new();

    rustc_args.push(args[0].clone()); // Program name

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--aspect-verbose" => {
                config.verbose = true;
                i += 1;
            }
            "--aspect-pointcut" => {
                if i + 1 < args.len() {
                    config.pointcuts.push(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --aspect-pointcut requires a value");
                    std::process::exit(1);
                }
            }
            "--aspect-output" => {
                if i + 1 < args.len() {
                    config.output_file = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("Error: --aspect-output requires a value");
                    std::process::exit(1);
                }
            }
            arg => {
                rustc_args.push(arg.to_string());
                i += 1;
            }
        }
    }

    if config.verbose {
        println!("aspect-rustc-driver starting");
        println!("Pointcuts: {:?}", config.pointcuts);
        println!("Rustc args: {:?}", rustc_args);
    }

    // Create callbacks
    let mut callbacks = AspectCompilerCallbacks::new(config);

    // Run the compiler
    RunCompiler::new(&rustc_args, &mut callbacks).run();
}
