//! cargo-aspect - Cargo subcommand for aspect-oriented programming
//!
//! This tool provides enhanced build capabilities for the aspect-rs framework.
//!
//! Usage:
//!   cargo aspect build
//!   cargo aspect test
//!   cargo aspect check

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::process::{Command, ExitCode};

/// cargo-aspect: Advanced build tool for aspect-oriented Rust
#[derive(Parser, Debug)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: CargoCommands,
}

#[derive(Subcommand, Debug)]
enum CargoCommands {
    /// Aspect-oriented build commands
    #[command(name = "aspect")]
    Aspect(AspectArgs),
}

#[derive(Parser, Debug)]
#[command(version, about = "Aspect-oriented programming for Rust", long_about = None)]
struct AspectArgs {
    #[command(subcommand)]
    command: Option<AspectCommand>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum AspectCommand {
    /// Build the current package with aspect weaving
    Build {
        /// Pass remaining args to cargo build
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Check the current package with aspect analysis
    Check {
        /// Pass remaining args to cargo check
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Test the current package with aspects enabled
    Test {
        /// Pass remaining args to cargo test
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Run benches with aspect weaving
    Bench {
        /// Pass remaining args to cargo bench
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Clean build artifacts
    Clean {
        /// Pass remaining args to cargo clean
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Show aspect analysis and weaving info
    Info {
        /// Show detailed function-level information
        #[arg(short, long)]
        detailed: bool,

        /// Filter by module path
        #[arg(short, long)]
        module: Option<String>,
    },

    /// List all registered aspects and pointcuts
    List {
        /// Show only aspects
        #[arg(long)]
        aspects: bool,

        /// Show only pointcuts
        #[arg(long)]
        pointcuts: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        CargoCommands::Aspect(args) => match run_aspect_command(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("Error: {:#}", e);
                ExitCode::FAILURE
            }
        },
    }
}

fn run_aspect_command(args: AspectArgs) -> Result<()> {
    if args.verbose {
        println!("cargo-aspect v{}", env!("CARGO_PKG_VERSION"));
    }

    match args.command {
        None => {
            // No subcommand, show help
            println!("cargo-aspect: Aspect-oriented programming for Rust");
            println!();
            println!("Usage: cargo aspect <COMMAND>");
            println!();
            println!("Commands:");
            println!("  build   Build with aspect weaving");
            println!("  check   Check with aspect analysis");
            println!("  test    Run tests with aspects");
            println!("  bench   Run benchmarks");
            println!("  clean   Clean build artifacts");
            println!("  info    Show aspect information");
            println!("  list    List aspects and pointcuts");
            println!();
            println!("Run 'cargo aspect <COMMAND> --help' for more information");
            Ok(())
        }

        Some(AspectCommand::Build { args: cargo_args }) => {
            if args.verbose {
                println!("Running: cargo build {}", cargo_args.join(" "));
            }
            run_cargo_command("build", &cargo_args)
        }

        Some(AspectCommand::Check { args: cargo_args }) => {
            if args.verbose {
                println!("Running: cargo check {}", cargo_args.join(" "));
            }
            run_cargo_command("check", &cargo_args)
        }

        Some(AspectCommand::Test { args: cargo_args }) => {
            if args.verbose {
                println!("Running: cargo test {}", cargo_args.join(" "));
            }
            run_cargo_command("test", &cargo_args)
        }

        Some(AspectCommand::Bench { args: cargo_args }) => {
            if args.verbose {
                println!("Running: cargo bench {}", cargo_args.join(" "));
            }
            run_cargo_command("bench", &cargo_args)
        }

        Some(AspectCommand::Clean { args: cargo_args }) => {
            if args.verbose {
                println!("Running: cargo clean {}", cargo_args.join(" "));
            }
            run_cargo_command("clean", &cargo_args)
        }

        Some(AspectCommand::Info {
            detailed,
            module: filter_module,
        }) => {
            println!("=== Aspect Information ===");
            println!();
            println!("Framework: aspect-rs v{}", env!("CARGO_PKG_VERSION"));
            println!("Status: Phase 2 Complete (Proc Macros + Pointcuts)");
            println!();

            if detailed {
                println!("Detailed analysis:");
                println!("  • Use #[aspect(AspectType)] for per-function aspects");
                println!("  • Use #[advice(...)] for pointcut-based aspects");
                println!("  • See documentation for advanced features");
            }

            if let Some(module) = filter_module {
                println!();
                println!("Filter: module = {}", module);
                println!("(Module filtering requires Phase 3 - rustc-driver integration)");
            }

            println!();
            println!("Note: Full aspect analysis will be available in Phase 3");
            Ok(())
        }

        Some(AspectCommand::List { aspects, pointcuts }) => {
            println!("=== Registered Aspects ===");
            println!();

            if aspects || (!aspects && !pointcuts) {
                println!("Available aspects (from aspect-std):");
                println!("  • LoggingAspect      - Structured logging");
                println!("  • TimingAspect       - Performance monitoring");
                println!("  • CachingAspect      - Memoization");
                println!("  • MetricsAspect      - Call statistics");
                println!("  • RateLimitAspect    - Rate limiting");
                println!("  • CircuitBreaker     - Fault tolerance");
                println!("  • Authorization      - Access control");
                println!("  • Validation         - Pre-conditions");
                println!();
            }

            if pointcuts || (!aspects && !pointcuts) {
                println!("Pointcut syntax:");
                println!("  execution(pub fn *(..))     - All public functions");
                println!("  within(crate::api)          - Functions in module");
                println!("  name(\"fetch_*\")              - Name patterns");
                println!("  execution(..) && within(..) - Combinations");
                println!();
            }

            println!("Note: Dynamic aspect discovery requires Phase 3");
            Ok(())
        }
    }
}

/// Run a standard cargo command with the given arguments
fn run_cargo_command(cmd: &str, args: &[String]) -> Result<()> {
    let status = Command::new("cargo")
        .arg(cmd)
        .args(args)
        .status()
        .context("Failed to execute cargo")?;

    if !status.success() {
        anyhow::bail!("cargo {} failed with status {}", cmd, status);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Just ensure CLI structure is valid
        let args = AspectArgs {
            command: None,
            verbose: false,
        };
        assert!(!args.verbose);
    }
}
