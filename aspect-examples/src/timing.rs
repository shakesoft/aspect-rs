//! Timing/performance monitoring aspect example.
//!
//! Demonstrates how to create a timing aspect that measures function
//! execution time and reports performance metrics.

use aspect_core::prelude::*;
use aspect_macros::aspect;
use std::any::Any;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// A timing aspect that measures function execution duration.
///
/// Uses thread-safe storage to track the start time across the
/// before/after advice boundary.
struct Timer {
    start_times: Arc<Mutex<Vec<Instant>>>,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            start_times: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Aspect for Timer {
    fn before(&self, ctx: &JoinPoint) {
        self.start_times.lock().unwrap().push(Instant::now());
        println!("[TIMER] Started: {}", ctx.function_name);
    }

    fn after(&self, ctx: &JoinPoint, _result: &dyn Any) {
        if let Some(start) = self.start_times.lock().unwrap().pop() {
            let elapsed = start.elapsed();
            println!(
                "[TIMER] {} took {:?} ({} μs)",
                ctx.function_name,
                elapsed,
                elapsed.as_micros()
            );
        }
    }

    fn after_error(&self, ctx: &JoinPoint, error: &AspectError) {
        if let Some(start) = self.start_times.lock().unwrap().pop() {
            let elapsed = start.elapsed();
            println!(
                "[TIMER] {} FAILED after {:?}: {:?}",
                ctx.function_name, elapsed, error
            );
        }
    }
}

// Simulate a fast operation
#[aspect(Timer::default())]
fn quick_operation(n: u32) -> u32 {
    n * 2
}

// Simulate a medium operation
#[aspect(Timer::default())]
fn medium_operation(n: u32) -> u32 {
    std::thread::sleep(std::time::Duration::from_millis(10));
    (1..=n).sum()
}

// Simulate a slow operation
#[aspect(Timer::default())]
fn slow_operation(iterations: u64) -> u64 {
    std::thread::sleep(std::time::Duration::from_millis(100));
    (0..iterations).map(|i| i * i).sum()
}

// Fibonacci (recursive, shows nested timing)
#[aspect(Timer::default())]
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

// Function that may fail
#[aspect(Timer::default())]
fn divide(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        Err("Division by zero".to_string())
    } else {
        std::thread::sleep(std::time::Duration::from_millis(5));
        Ok(a / b)
    }
}

fn main() {
    println!("=== Timing Aspect Example ===\n");

    // Example 1: Quick operation
    println!("1. Quick operation:");
    let result = quick_operation(21);
    println!("   Result: {}\n", result);

    // Example 2: Medium operation
    println!("2. Medium operation:");
    let result = medium_operation(100);
    println!("   Result: {}\n", result);

    // Example 3: Slow operation
    println!("3. Slow operation:");
    let result = slow_operation(1000);
    println!("   Result: {}\n", result);

    // Example 4: Recursive function (shows nested timing)
    println!("4. Fibonacci (recursive - will show nested timings):");
    let result = fibonacci(10);
    println!("   Result: {}\n", result);

    // Example 5: Function with Result (success)
    println!("5. Division (success case):");
    match divide(42, 6) {
        Ok(result) => println!("   Result: {}\n", result),
        Err(e) => println!("   Error: {}\n", e),
    }

    // Example 6: Function with Result (error)
    println!("6. Division by zero (error case):");
    match divide(42, 0) {
        Ok(result) => println!("   Result: {}\n", result),
        Err(e) => println!("   Error: {}\n", e),
    }

    println!("=== Demo Complete ===");
}
