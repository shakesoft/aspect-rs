//! Logging aspect example.
//!
//! Demonstrates how to create a basic logging aspect that logs function
//! entry and exit with timestamps.

use aspect_core::prelude::*;
use aspect_macros::aspect;
use std::any::Any;

/// A logging aspect that prints entry and exit messages.
#[derive(Default)]
struct Logger;

impl Aspect for Logger {
    fn before(&self, ctx: &JoinPoint) {
        println!(
            "[{}] [ENTRY] {} at {}",
            current_timestamp(),
            ctx.function_name,
            ctx.location
        );
    }

    fn after(&self, ctx: &JoinPoint, _result: &dyn Any) {
        println!("[{}] [EXIT]  {}", current_timestamp(), ctx.function_name);
    }

    fn after_error(&self, ctx: &JoinPoint, error: &AspectError) {
        eprintln!(
            "[{}] [ERROR] {} failed: {:?}",
            current_timestamp(),
            ctx.function_name,
            error
        );
    }
}

/// Helper function to get current timestamp.
fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}.{:03}", duration.as_secs(), duration.subsec_millis())
}

// Example domain model
#[derive(Debug, Clone)]
struct User {
    id: u64,
    name: String,
}

// Apply logging aspect to a simple function
#[aspect(Logger::default())]
fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}

// Apply logging aspect to a function returning Result
#[aspect(Logger::default())]
fn fetch_user(id: u64) -> Result<User, String> {
    if id == 0 {
        Err("Invalid user ID: 0".to_string())
    } else {
        Ok(User {
            id,
            name: format!("User{}", id),
        })
    }
}

// Apply logging aspect to a function with multiple parameters
#[aspect(Logger::default())]
fn process_data(input: String, multiplier: usize) -> String {
    input.repeat(multiplier)
}

fn main() {
    println!("=== Logging Aspect Example ===\n");

    // Example 1: Simple function
    println!("1. Calling greet(\"Alice\"):");
    let greeting = greet("Alice".to_string());
    println!("   Result: {}\n", greeting);

    // Example 2: Function returning Result (success case)
    println!("2. Calling fetch_user(42):");
    match fetch_user(42) {
        Ok(user) => println!("   Success: {:?}\n", user),
        Err(e) => println!("   Error: {}\n", e),
    }

    // Example 3: Function returning Result (error case)
    println!("3. Calling fetch_user(0) (will fail):");
    match fetch_user(0) {
        Ok(user) => println!("   Success: {:?}\n", user),
        Err(e) => println!("   Error: {}\n", e),
    }

    // Example 4: Function with multiple parameters
    println!("4. Calling process_data(\"Rust \", 3):");
    let result = process_data("Rust ".to_string(), 3);
    println!("   Result: {}\n", result);

    println!("=== Demo Complete ===");
}
