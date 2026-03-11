//! Pointcut Example - Pattern-Based Aspect Matching
//!
//! This example demonstrates:
//! 1. Defining pointcut patterns for matching functions
//! 2. Programmatically finding functions that match pointcuts
//! 3. The foundation for declarative aspect application
//!
//! Note: Full automatic weaving (applying aspects without any annotation) would require
//! either a compiler plugin or build-time code generation. This example shows the core

use aspect_core::pointcut::{FunctionInfo, Matcher, Pointcut};

fn main() {
    println!("=== Pointcut Pattern Matching Demo ===\n");

    // Example 1: Parse and use pointcut expressions
    println!("1. Parsing Pointcut Expressions:");
    println!();

    let pc1 = Pointcut::parse("execution(pub fn *(..))").expect("Failed to parse pointcut");
    println!("   Parsed: execution(pub fn *(..))");

    let pc2 = Pointcut::parse("within(crate::api)").expect("Failed to parse pointcut");
    println!("   Parsed: within(crate::api)");

    let pc3 = Pointcut::parse("execution(pub fn *(..)) && within(crate::api)")
        .expect("Failed to parse pointcut");
    println!("   Parsed: execution(pub fn *(..)) && within(crate::api)");
    println!();

    // Example 2: Match functions against pointcuts
    println!("2. Matching Functions Against Pointcuts:");
    println!();

    let functions = vec![
        FunctionInfo {
            name: "fetch_user".to_string(),
            module_path: "crate::api::users".to_string(),
            visibility: "pub".to_string(),
            return_type: None,
        },
        FunctionInfo {
            name: "save_user".to_string(),
            module_path: "crate::api::users".to_string(),
            visibility: "pub".to_string(),
            return_type: None,
        },
        FunctionInfo {
            name: "internal_helper".to_string(),
            module_path: "crate::internal".to_string(),
            visibility: "".to_string(),
            return_type: None,
        },
        FunctionInfo {
            name: "delete_all".to_string(),
            module_path: "crate::admin".to_string(),
            visibility: "pub".to_string(),
            return_type: None,
        },
    ];

    // Test pointcut: all public functions
    println!("   Pointcut: execution(pub fn *(..))");
    for func in &functions {
        if pc1.matches(func) {
            println!("      ✓ Matches: {}::{}", func.module_path, func.name);
        }
    }
    println!();

    // Test pointcut: functions in api module
    println!("   Pointcut: within(crate::api)");
    for func in &functions {
        if pc2.matches(func) {
            println!("      ✓ Matches: {}::{}", func.module_path, func.name);
        }
    }
    println!();

    // Test pointcut: public functions in api module (combined)
    println!("   Pointcut: execution(pub fn *(..)) && within(crate::api)");
    for func in &functions {
        if pc3.matches(func) {
            println!("      ✓ Matches: {}::{}", func.module_path, func.name);
        }
    }
    println!();

    // Example 3: Pattern types
    println!("3. Different Pattern Types:");
    println!();

    // Name patterns
    let save_pattern = Pointcut::parse("execution(fn save_user(..))").expect("Failed to parse");
    println!("   Exact name match: execution(fn save_user(..))");
    for func in &functions {
        if save_pattern.matches(func) {
            println!("      ✓ Matches: {}", func.name);
        }
    }
    println!();

    let save_prefix = Pointcut::parse("execution(fn save*(..))").expect("Failed to parse");
    println!("   Prefix match: execution(fn save*(..))");
    for func in &functions {
        if save_prefix.matches(func) {
            println!("      ✓ Matches: {}", func.name);
        }
    }
    println!();

    // Example 4: OR combinations
    println!("4. OR Combinations:");
    println!();

    let or_pattern =
        Pointcut::parse("within(crate::api) || within(crate::admin)").expect("Failed to parse");
    println!("   Pointcut: within(crate::api) || within(crate::admin)");
    println!("   Meaning: Functions in API OR admin modules");
    println!();
    for func in &functions {
        if or_pattern.matches(func) {
            println!("      ✓ Matches: {}::{}", func.module_path, func.name);
        }
    }
    println!();

    // Example 4b: Parentheses for grouping
    println!("4b. Parentheses for Grouping:");
    println!();

    let grouped =
        Pointcut::parse("(within(crate::api) || within(crate::admin)) && execution(pub fn *(..))")
            .expect("Failed to parse");
    println!(
        "   Pointcut: (within(crate::api) || within(crate::admin)) && execution(pub fn *(..))"
    );
    println!("   Meaning: Public functions in API OR admin modules");
    println!();
    for func in &functions {
        if grouped.matches(func) {
            println!("      ✓ Matches: {}::{}", func.module_path, func.name);
        }
    }
    println!();

    // Example 5: Negation
    println!("5. Negation with NOT:");
    println!();

    let not_internal = Pointcut::parse("execution(pub fn *(..)) && !within(crate::internal)")
        .expect("Failed to parse");
    println!("   Pointcut: execution(pub fn *(..)) && !within(crate::internal)");
    println!("   Meaning: All public functions EXCEPT those in crate::internal");
    println!();
    for func in &functions {
        if not_internal.matches(func) {
            println!("      ✓ Matches: {}::{}", func.module_path, func.name);
        }
    }
    println!();

    println!("=== Demo Complete ===");
    println!();
    println!("Key Points:");
    println!("- Pointcut expressions can match functions by signature and location");
    println!("- Boolean operators (&&, ||, !) allow complex matching logic");
    println!("- This infrastructure enables declarative aspect application");
    println!();
    println!("Future Enhancements:");
    println!("- #[weave] macro to automatically apply matching aspects");
    println!("- Build-time aspect weaving using build.rs");
    println!("- rustc plugin for full compile-time aspect-oriented programming");
}
