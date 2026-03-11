//! Caching/memoization aspect example (simplified).
//!
//! Demonstrates how to track function calls that would benefit from caching.
//! This simplified version shows the aspect pattern without actual caching complexity.

use aspect_core::prelude::*;
use aspect_macros::aspect;
use std::any::Any;

/// A cache monitoring aspect that tracks function calls.
///
/// Note: This is a simplified demonstration of aspect-based monitoring.
/// Production caching would require:
/// - Argument-based cache keys
/// - Actual caching storage
/// - Cache eviction policies (LRU, TTL, size limits)
/// - "around" advice for intercepting and returning cached values
#[derive(Default)]
struct CacheMonitor {
    call_count: std::sync::Arc<std::sync::Mutex<usize>>,
}

impl CacheMonitor {
    fn new() -> Self {
        Self::default()
    }
}

impl Aspect for CacheMonitor {
    fn before(&self, ctx: &JoinPoint) {
        let call_num = {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
            *count
        };

        println!(
            "[CACHE-MONITOR] Call #{} to {} - cache miss, executing function",
            call_num, ctx.function_name
        );
    }

    fn after(&self, ctx: &JoinPoint, _result: &dyn Any) {
        println!(
            "[CACHE-MONITOR] {} completed - result cached for future calls",
            ctx.function_name
        );
    }
}

// Example functions

#[aspect(CacheMonitor::new())]
fn compute_expensive(n: u64) -> u64 {
    // Simulate expensive computation
    std::thread::sleep(std::time::Duration::from_millis(50));
    (0..n).sum()
}

#[aspect(CacheMonitor::new())]
fn fetch_data(key: String) -> String {
    // Simulate data fetching
    std::thread::sleep(std::time::Duration::from_millis(30));
    format!("Data for key: {}", key)
}

#[aspect(CacheMonitor::new())]
fn calculate(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        Err("Cannot divide by zero".to_string())
    } else {
        std::thread::sleep(std::time::Duration::from_millis(10));
        Ok(a / b)
    }
}

fn main() {
    println!("=== Caching Aspect Example (Monitoring) ===\n");
    println!("This example demonstrates tracking function calls");
    println!("that would benefit from caching.\n");

    // Example 1: Multiple calls to expensive computation
    println!("1. Calling compute_expensive(100) three times:");
    let result1 = compute_expensive(100);
    println!("   Result: {}\n", result1);

    let result2 = compute_expensive(100);
    println!("   Result: {}\n", result2);

    let result3 = compute_expensive(100);
    println!("   Result: {}\n", result3);

    println!("Note: In a real caching implementation with 'around' advice,");
    println!("calls #2 and #3 would return cached results instantly.\n");

    // Example 2: Data fetching
    println!("2. Calling fetch_data with same key twice:");
    let data1 = fetch_data("user:42".to_string());
    println!("   Result: {}\n", data1);

    let data2 = fetch_data("user:42".to_string());
    println!("   Result: {}\n", data2);

    println!("Note: Second call could use cached data.\n");

    // Example 3: Function with Result
    println!("3. Calling calculate(100, 5):");
    match calculate(100, 5) {
        Ok(result) => println!("   Result: {}\n", result),
        Err(e) => println!("   Error: {}\n", e),
    }

    // Example 4: Function that errors (not cached)
    println!("4. Calling calculate(100, 0) (will fail):");
    match calculate(100, 0) {
        Ok(result) => println!("   Result: {}\n", result),
        Err(e) => println!("   Error: {}\n", e),
    }

    println!("\n=== Demo Complete ===");
    println!("\nKey Takeaway:");
    println!("Aspects can monitor function calls to identify");
    println!("caching opportunities and optimize performance.");
}
