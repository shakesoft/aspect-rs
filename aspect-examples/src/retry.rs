//! Retry aspect example - True around advice demonstration.
//!
//! Shows how to implement retry logic using around advice to intercept
//! and retry failed operations without modifying business logic.

use aspect_core::prelude::*;
use aspect_macros::aspect;
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// A retry aspect that automatically retries failed operations.
struct RetryAspect {
    max_attempts: usize,
    backoff_ms: u64,
    attempt_counter: AtomicUsize,
}

impl RetryAspect {
    fn new(max_attempts: usize, backoff_ms: u64) -> Self {
        Self {
            max_attempts,
            backoff_ms,
            attempt_counter: AtomicUsize::new(0),
        }
    }

    fn attempts(&self) -> usize {
        self.attempt_counter.load(Ordering::SeqCst)
    }
}

impl Aspect for RetryAspect {
    fn around(&self, pjp: ProceedingJoinPoint) -> Result<Box<dyn Any>, AspectError> {
        let function_name = pjp.context().function_name;

        // Reset counter
        self.attempt_counter.store(0, Ordering::SeqCst);

        let mut last_error = None;

        for attempt in 1..=self.max_attempts {
            self.attempt_counter.fetch_add(1, Ordering::SeqCst);

            println!(
                "[RETRY] Attempt {}/{} for {}",
                attempt, self.max_attempts, function_name
            );

            // Try to execute the function
            // Note: We can't actually retry with the same PJP since proceed() consumes it
            // This is a demonstration - in production you'd need to restructure this
            match pjp.proceed() {
                Ok(result) => {
                    if attempt > 1 {
                        println!(
                            "[RETRY] ✓ Success on attempt {}/{}",
                            attempt, self.max_attempts
                        );
                    }
                    return Ok(result);
                }
                Err(error) => {
                    last_error = Some(error);

                    if attempt < self.max_attempts {
                        let backoff = Duration::from_millis(
                            self.backoff_ms * 2_u64.pow((attempt - 1) as u32),
                        );
                        println!(
                            "[RETRY] ✗ Attempt {} failed, retrying in {:?}...",
                            attempt, backoff
                        );
                        std::thread::sleep(backoff);
                    }
                }
            }

            // Break after first attempt for this demo (PJP consumed)
            break;
        }

        Err(last_error.unwrap_or_else(|| AspectError::execution("All retries failed")))
    }
}

// Simulated unstable service that fails sometimes
static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

#[aspect(RetryAspect::new(3, 100))]
fn unstable_service(fail_until: usize) -> Result<String, String> {
    let call_num = CALL_COUNT.fetch_add(1, Ordering::SeqCst) + 1;

    if call_num < fail_until {
        println!("  [SERVICE] Call #{} - FAILING", call_num);
        Err(format!(
            "Service temporarily unavailable (call #{})",
            call_num
        ))
    } else {
        println!("  [SERVICE] Call #{} - SUCCESS", call_num);
        Ok(format!("Data from call #{}", call_num))
    }
}

// Circuit breaker aspect
struct CircuitBreakerAspect {
    failure_count: AtomicUsize,
    failure_threshold: usize,
}

impl CircuitBreakerAspect {
    fn new(failure_threshold: usize) -> Self {
        Self {
            failure_count: AtomicUsize::new(0),
            failure_threshold,
        }
    }

    fn failures(&self) -> usize {
        self.failure_count.load(Ordering::SeqCst)
    }
}

impl Aspect for CircuitBreakerAspect {
    fn before(&self, ctx: &JoinPoint) {
        let failures = self.failure_count.load(Ordering::SeqCst);

        if failures >= self.failure_threshold {
            println!(
                "[CIRCUIT-BREAKER] ⚠ Circuit OPEN for {} ({} failures) - Fast failing",
                ctx.function_name, failures
            );
            // In a real implementation, we'd prevent execution here
            // For demo purposes, we'll just log
        }
    }

    fn after(&self, ctx: &JoinPoint, _result: &dyn Any) {
        // Reset on success
        let prev = self.failure_count.swap(0, Ordering::SeqCst);
        if prev > 0 {
            println!(
                "[CIRCUIT-BREAKER] ✓ Success - Circuit CLOSED (was {} failures)",
                prev
            );
        }
    }

    fn after_error(&self, ctx: &JoinPoint, error: &AspectError) {
        let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        println!(
            "[CIRCUIT-BREAKER] ✗ Failure #{} in {}",
            failures, ctx.function_name
        );

        if failures >= self.failure_threshold {
            println!("[CIRCUIT-BREAKER] ⚠ Circuit now OPEN - Will fast-fail future calls");
        }
    }
}

static FLAKY_COUNT: AtomicUsize = AtomicUsize::new(0);

#[aspect(CircuitBreakerAspect::new(3))]
fn flaky_operation(id: u32) -> Result<u32, String> {
    let call_num = FLAKY_COUNT.fetch_add(1, Ordering::SeqCst) + 1;

    // Fail the first 3 calls, then succeed
    if call_num <= 3 {
        Err(format!("Flaky failure #{}", call_num))
    } else {
        Ok(id * 2)
    }
}

fn main() {
    println!("=== Retry & Circuit Breaker Aspect Examples ===\n");

    // Example 1: Retry aspect (would work if we could clone PJP)
    println!("1. Retry Aspect (Note: Limited by ProceedingJoinPoint consumption):");
    CALL_COUNT.store(0, Ordering::SeqCst);

    match unstable_service(2) {
        Ok(data) => println!("   Result: {}\n", data),
        Err(e) => println!("   Error: {}\n", e),
    }

    // Example 2: Circuit breaker
    println!("2. Circuit Breaker Aspect:");
    println!("   Attempting flaky_operation multiple times...\n");

    for i in 1..=5 {
        println!("   Call #{}:", i);
        match flaky_operation(i) {
            Ok(result) => println!("   ✓ Success: {}\n", result),
            Err(e) => println!("   ✗ Error: {}\n", e),
        }
    }

    println!("\n=== Demo Complete ===");
    println!("\nKey Takeaways:");
    println!("✓ Around advice enables powerful control flow patterns");
    println!("✓ Retry logic can be extracted from business code");
    println!("✓ Circuit breakers protect against cascading failures");
    println!("✓ Aspects compose cleanly with before/after/after_error");
    println!("\nNote: True retry in around() requires cloneable ProceedingJoinPoint");
    println!("or alternative design - this is a known limitation to explore!");
}
