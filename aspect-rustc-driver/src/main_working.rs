//! aspect-rustc-driver - WORKING VERSION with MIR extraction
//!
//! This binary successfully extracts function metadata from Rust MIR
//! using rustc-driver integration.

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;

use rustc_driver::{Callbacks, RunCompiler};
use rustc_interface::interface;
use rustc_middle::ty::TyCtxt;
use std::path::PathBuf;
use std::sync::Mutex;

use aspect_driver::mir_analyzer::{AnalysisStats, MirAnalyzer};
use aspect_driver::types::{FunctionMetadata, Visibility};

/// Global configuration (needed for query provider function pointers)
static CONFIG: Mutex<Option<AspectConfig>> = Mutex::new(None);
static RESULTS: Mutex<Option<AnalysisResults>> = Mutex::new(None);

#[derive(Debug, Clone)]
struct AspectConfig {
    pointcuts: Vec<String>,
    verbose: bool,
    output_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct AnalysisResults {
    functions: Vec<FunctionMetadata>,
    matched_functions: Vec<(FunctionMetadata, String)>, // (function, pointcut)
}

/// Analysis function called with TyCtxt - this is where the magic happens!
fn analyze_crate_with_aspects(tcx: TyCtxt<'_>, (): ()) {
    let config = CONFIG.lock().unwrap().clone().unwrap();

    if config.verbose {
        println!("\n🎉 TyCtxt Access Successful!");
        println!("=== aspect-rustc-driver: MIR Analysis ===\n");
    }

    // Extract all functions from MIR
    let analyzer = MirAnalyzer::new(tcx, config.verbose);
    let functions = analyzer.extract_all_functions();

    if config.verbose {
        println!("\n✅ Extracted {} functions from MIR", functions.len());
    }

    // Print statistics
    let stats = AnalysisStats::from_functions(&functions);
    stats.print_summary();

    // Apply simple pointcut matching
    let mut matched_functions = Vec::new();

    if !config.pointcuts.is_empty() {
        if config.verbose {
            println!("\n=== Pointcut Matching ===");
        }

        for pointcut_str in &config.pointcuts {
            if config.verbose {
                println!("\nPointcut: \"{}\"", pointcut_str);
            }

            // Simple pattern matching (full PointcutMatcher integration coming next)
            let mut match_count = 0;
            for func in &functions {
                let matches = match pointcut_str.as_str() {
                    "execution(pub fn *(..))" => func.is_public(),
                    s if s.starts_with("within(") => {
                        // Extract module from within(module::path)
                        let module = s.trim_start_matches("within(").trim_end_matches(')');
                        func.module_path.contains(module) || func.name.contains(module)
                    }
                    _ => {
                        // Default: match by name pattern
                        func.name.contains(pointcut_str)
                    }
                };

                if matches {
                    if config.verbose {
                        println!("  ✓ Matched: {}", func.name);
                    }
                    matched_functions.push((func.clone(), pointcut_str.clone()));
                    match_count += 1;
                }
            }

            if config.verbose {
                println!("  Total matches: {}", match_count);
            }
        }

        println!("\n=== Matching Summary ===");
        println!("Total functions matched: {}", matched_functions.len());
    }

    // Store results
    *RESULTS.lock().unwrap() = Some(AnalysisResults {
        functions,
        matched_functions,
    });
}

struct AspectCallbacks;

impl Callbacks for AspectCallbacks {
    fn config(&mut self, config: &mut interface::Config) {
        let aspect_config = CONFIG.lock().unwrap().clone().unwrap();

        if aspect_config.verbose {
            println!("=== aspect-rustc-driver: Configuring compiler ===");
            println!("Pointcuts registered: {}", aspect_config.pointcuts.len());
        }

        // Use override_queries to intercept analysis phase
        config.override_queries = Some(|_sess, providers| {
            providers.analysis = analyze_crate_with_aspects;
        });
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().collect();

    // Parse aspect-specific flags
    let mut aspect_config = AspectConfig {
        pointcuts: Vec::new(),
        verbose: false,
        output_file: None,
    };

    let mut rustc_args = Vec::new();
    rustc_args.push(args[0].clone());

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--aspect-verbose" => {
                aspect_config.verbose = true;
                i += 1;
            }
            "--aspect-pointcut" => {
                if i + 1 < args.len() {
                    aspect_config.pointcuts.push(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --aspect-pointcut requires a value");
                    std::process::exit(1);
                }
            }
            "--aspect-output" => {
                if i + 1 < args.len() {
                    aspect_config.output_file = Some(PathBuf::from(&args[i + 1]));
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

    if aspect_config.verbose {
        println!("aspect-rustc-driver starting");
        println!("Pointcuts: {:?}", aspect_config.pointcuts);
        println!("Rustc args: {:?}", rustc_args);
        println!();
    }

    // Store config in global state
    *CONFIG.lock().unwrap() = Some(aspect_config.clone());

    // Run compiler
    let mut callbacks = AspectCallbacks;
    RunCompiler::new(&rustc_args, &mut callbacks).run();

    // Output results
    if let Some(results) = RESULTS.lock().unwrap().as_ref() {
        println!("\n=== Aspect Weaving Analysis Complete ===");
        println!("Functions analyzed: {}", results.functions.len());
        println!(
            "Functions matched by pointcuts: {}",
            results.matched_functions.len()
        );

        // Write output file if requested
        if let Some(ref output_path) = aspect_config.output_file {
            if let Err(e) = write_output_file(output_path, results) {
                eprintln!("Error writing output: {}", e);
            } else {
                println!("\n✅ Analysis written to: {}", output_path.display());
            }
        }

        println!("\n✅ SUCCESS: Automatic aspect weaving analysis complete!");
    }
}

fn write_output_file(path: &PathBuf, results: &AnalysisResults) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(path)?;

    writeln!(file, "=== Aspect Weaving Analysis Results ===")?;
    writeln!(file)?;
    writeln!(file, "Total functions: {}", results.functions.len())?;
    writeln!(file)?;

    writeln!(file, "All Functions:")?;
    for func in &results.functions {
        writeln!(file, "  • {} ({:?})", func.name, func.visibility)?;
        writeln!(file, "    Module: {}", func.module_path)?;
        writeln!(
            file,
            "    Location: {}:{}",
            func.location.file, func.location.line
        )?;
    }

    writeln!(file)?;
    writeln!(file, "Matched Functions:")?;
    writeln!(file, "Total matches: {}", results.matched_functions.len())?;
    writeln!(file)?;

    for (func, pointcut) in &results.matched_functions {
        writeln!(file, "  • {}", func.name)?;
        writeln!(file, "    Pointcut: {}", pointcut)?;
    }

    Ok(())
}
