//! Advanced Aspects Example
//!
//! Demonstrates the enhanced standard library aspects:
//! - RateLimitAspect
//! - CircuitBreakerAspect
//! - AuthorizationAspect
//! - ValidationAspect

use aspect_core::prelude::*;
use aspect_macros::aspect;
use aspect_std::prelude::*;
use std::collections::HashSet;
use std::time::Duration;

// ============================================================================
// Rate Limiting Example
// ============================================================================

#[aspect(RateLimitAspect::new(3, Duration::from_secs(2)))]
fn api_call(id: u64) -> Result<String, AspectError> {
    Ok(format!("API response for {}", id))
}

// ============================================================================
// Circuit Breaker Example
// ============================================================================

static mut FAIL_NEXT: bool = false;

#[aspect(CircuitBreakerAspect::new(2, Duration::from_secs(1)))]
fn unreliable_service() -> Result<String, AspectError> {
    unsafe {
        if FAIL_NEXT {
            return Err(AspectError::execution("Service failure"));
        }
    }
    Ok("Success".to_string())
}

// ============================================================================
// Authorization Example
// ============================================================================

fn get_current_roles() -> HashSet<String> {
    // Simulated current user roles
    let mut roles = HashSet::new();
    roles.insert("user".to_string());
    roles
}

fn get_admin_roles() -> HashSet<String> {
    let mut roles = HashSet::new();
    roles.insert("admin".to_string());
    roles
}

#[aspect(AuthorizationAspect::require_role("admin", get_admin_roles))]
fn admin_only_operation() -> Result<(), AspectError> {
    println!("  [ADMIN] Performing admin operation");
    Ok(())
}

#[aspect(AuthorizationAspect::require_role("user", get_current_roles))]
fn user_operation() -> Result<(), AspectError> {
    println!("  [USER] Performing user operation");
    Ok(())
}

// ============================================================================
// Validation Example
// ============================================================================

use aspect_std::validation::ValidationRule;

struct AgeValidator;
impl ValidationRule for AgeValidator {
    fn validate(&self, _ctx: &JoinPoint) -> Result<(), String> {
        // Simplified validation - in real code, extract from ctx.args
        Ok(())
    }
}

#[aspect(ValidationAspect::new().add_rule(Box::new(AgeValidator)))]
fn set_user_age(age: i32) -> Result<(), AspectError> {
    println!("  [VALIDATE] Setting age to {}", age);
    Ok(())
}

// ============================================================================
// Main Demonstration
// ============================================================================

fn main() {
    println!("=== Advanced Aspects Demo ===\n");

    // Rate Limiting Demo
    println!("1. Rate Limiting (3 requests per 2 seconds)");
    for i in 1..=5 {
        match api_call(i) {
            Ok(response) => println!("   Request {}: {}", i, response),
            Err(e) => println!("   Request {}: RATE LIMITED - {}", i, e),
        }
    }
    println!();

    // Circuit Breaker Demo
    println!("2. Circuit Breaker (opens after 2 failures)");
    unsafe {
        FAIL_NEXT = true;
    }

    for i in 1..=4 {
        match unreliable_service() {
            Ok(msg) => println!("   Attempt {}: {}", i, msg),
            Err(e) => println!("   Attempt {}: FAILED - {}", i, e),
        }
    }
    println!();

    // Authorization Demo
    println!("3. Authorization (role-based access control)");
    match user_operation() {
        Ok(_) => println!("   User operation: SUCCESS"),
        Err(e) => println!("   User operation: DENIED - {}", e),
    }

    match admin_only_operation() {
        Ok(_) => println!("   Admin operation: SUCCESS"),
        Err(_) => println!("   Admin operation: DENIED"),
    }
    println!();

    // Validation Demo
    println!("4. Validation (pre-condition checking)");
    match set_user_age(25) {
        Ok(_) => println!("   Age validation: PASSED"),
        Err(e) => println!("   Age validation: FAILED - {}", e),
    }
    println!();

    println!("=== Demo Complete ===\n");
    println!("Key Takeaways:");
    println!("✓ RateLimitAspect prevents resource exhaustion");
    println!("✓ CircuitBreakerAspect protects against cascading failures");
    println!("✓ AuthorizationAspect enforces access control");
    println!("✓ ValidationAspect ensures data integrity");
    println!("✓ All aspects are composable and reusable!");
}
