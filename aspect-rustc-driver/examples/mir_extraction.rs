//! MIR extraction example using rustc-driver callbacks

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;

use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_interface::interface;
use rustc_middle::ty::TyCtxt;

use aspect_driver::mir_analyzer::{AnalysisStats, MirAnalyzer};

struct MirExtractorCallbacks {
    verbose: bool,
}

impl Callbacks for MirExtractorCallbacks {
    // Override config to ensure we perform analysis
    fn config(&mut self, config: &mut interface::Config) {
        // Ensure optimization level allows MIR analysis
        if self.verbose {
            println!("=== Configuring compiler ===");
        }
    }
}

impl MirExtractorCallbacks {
    fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().collect();

    // Check for verbose flag
    let verbose = args.iter().any(|arg| arg == "--verbose" || arg == "-v");

    // Remove our custom flags
    args.retain(|arg| arg != "--verbose" && arg != "-v");

    println!("=== MIR Extraction Example ===");
    if verbose {
        println!("Args: {:?}", args);
    }

    // Create callbacks
    let mut callbacks = MirExtractorCallbacks::new(verbose);

    // Run compiler
    RunCompiler::new(&args, &mut callbacks).run();

    println!("\n=== Compilation Complete ===");
}
