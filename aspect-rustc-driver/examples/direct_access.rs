//! Direct TyCtxt access using rustc_interface::run_compiler

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use rustc_interface::interface;
use rustc_session::config::{self, Input};
use rustc_span::edition::Edition;
use std::path::PathBuf;

use aspect_driver::mir_analyzer::{AnalysisStats, MirAnalyzer};

fn main() {
    println!("=== Direct TyCtxt Access Experiment ===");

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rust_file>", args[0]);
        std::process::exit(1);
    }

    let input_file = PathBuf::from(&args[1]);
    if !input_file.exists() {
        eprintln!("Error: File not found: {}", input_file.display());
        std::process::exit(1);
    }

    println!("Analyzing: {}", input_file.display());

    // Create compiler configuration
    let config = interface::Config {
        opts: config::Options {
            maybe_sysroot: None, // Auto-detect
            crate_types: vec![config::CrateType::Rlib],
            edition: Edition::Edition2021,
            ..config::Options::default()
        },
        crate_cfg: vec![],
        crate_check_cfg: vec![],
        input: Input::File(input_file.clone()),
        output_dir: None,
        output_file: None,
        file_loader: None,
        locale_resources: vec![],
        lint_caps: Default::default(),
        psess_created: None,
        hash_untracked_state: None,
        register_lints: None,
        override_queries: None,
        make_codegen_backend: None,
        registry: rustc_driver::diagnostics_registry(),
        ice_file: None,
        using_internal_features: Default::default(),
        expanded_args: vec![],
    };

    // Run compiler and access TyCtxt
    let result = rustc_interface::run_compiler(config, |compiler| {
        println!("\n=== Inside compiler closure ===");

        // The compiler parameter IS the queries object
        // We can call global_ctxt() directly on it
        compiler.enter(|queries| {
            println!("Entered queries");

            // Get global context (type context)
            let gcx_result = queries.global_ctxt();

            match gcx_result {
                Ok(gcx) => {
                    gcx.enter(|tcx| {
                        println!("\n🎉 SUCCESS! We have TyCtxt access!");
                        println!("=== MIR Analysis Starting ===\n");

                        // Create MIR analyzer
                        let analyzer = MirAnalyzer::new(tcx, true);

                        // Extract all functions
                        let functions = analyzer.extract_all_functions();

                        // Print statistics
                        let stats = AnalysisStats::from_functions(&functions);
                        stats.print_summary();

                        println!("\n=== SUCCESS: MIR extraction complete! ===");
                        println!("Extracted {} functions from MIR", functions.len());
                    });
                }
                Err(e) => {
                    eprintln!("Error getting global context: {:?}", e);
                }
            }
        });
    });

    println!("\nCompiler run completed");
}
