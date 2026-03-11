//! Input validation aspect example.
//!
//! Demonstrates how to enforce validation rules and constraints
//! declaratively using aspects, separating validation from business logic.

use aspect_core::prelude::*;
use aspect_macros::aspect;
use std::any::Any;

/// Validation rule trait
trait ValidationRule: Send + Sync {
    fn validate(&self, value: &dyn Any) -> Result<(), String>;
    fn description(&self) -> String;
}

/// Range validator for numeric types
struct RangeValidator<T: PartialOrd + std::fmt::Display + 'static> {
    min: T,
    max: T,
    field_name: String,
}

impl<T: PartialOrd + std::fmt::Display + Send + Sync + 'static> ValidationRule
    for RangeValidator<T>
{
    fn validate(&self, value: &dyn Any) -> Result<(), String> {
        if let Some(val) = value.downcast_ref::<T>() {
            if val < &self.min || val > &self.max {
                return Err(format!(
                    "{} must be between {} and {} (got: {})",
                    self.field_name, self.min, self.max, val
                ));
            }
            Ok(())
        } else {
            Ok(()) // Type doesn't match, skip validation
        }
    }

    fn description(&self) -> String {
        format!("{} in range [{}, {}]", self.field_name, self.min, self.max)
    }
}

/// String length validator
struct LengthValidator {
    min_length: usize,
    max_length: usize,
    field_name: String,
}

impl ValidationRule for LengthValidator {
    fn validate(&self, value: &dyn Any) -> Result<(), String> {
        if let Some(s) = value.downcast_ref::<&str>() {
            let len = s.len();
            if len < self.min_length || len > self.max_length {
                return Err(format!(
                    "{} length must be between {} and {} characters (got: {})",
                    self.field_name, self.min_length, self.max_length, len
                ));
            }
            Ok(())
        } else if let Some(s) = value.downcast_ref::<String>() {
            let len = s.len();
            if len < self.min_length || len > self.max_length {
                return Err(format!(
                    "{} length must be between {} and {} characters (got: {})",
                    self.field_name, self.min_length, self.max_length, len
                ));
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    fn description(&self) -> String {
        format!(
            "{} length in [{}, {}]",
            self.field_name, self.min_length, self.max_length
        )
    }
}

/// Email format validator
struct EmailValidator {
    field_name: String,
}

impl ValidationRule for EmailValidator {
    fn validate(&self, value: &dyn Any) -> Result<(), String> {
        if let Some(s) = value.downcast_ref::<&str>() {
            if !s.contains('@') || !s.contains('.') {
                return Err(format!("{} must be a valid email address", self.field_name));
            }
            Ok(())
        } else if let Some(s) = value.downcast_ref::<String>() {
            if !s.contains('@') || !s.contains('.') {
                return Err(format!("{} must be a valid email address", self.field_name));
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    fn description(&self) -> String {
        format!("{} is valid email", self.field_name)
    }
}

/// Validation aspect that enforces rules
struct ValidationAspect {
    rules: Vec<Box<dyn ValidationRule>>,
}

impl ValidationAspect {
    fn new() -> Self {
        Self { rules: Vec::new() }
    }

    fn add_rule(mut self, rule: Box<dyn ValidationRule>) -> Self {
        self.rules.push(rule);
        self
    }
}

impl Aspect for ValidationAspect {
    fn before(&self, ctx: &JoinPoint) {
        if !self.rules.is_empty() {
            println!(
                "[VALIDATION] Checking {} rules for {}",
                self.rules.len(),
                ctx.function_name
            );
        }

        // Note: In a real implementation, you'd need to capture function arguments
        // This is a simplified demonstration
        for rule in &self.rules {
            println!("  [VALIDATION] Rule: {}", rule.description());
        }
    }
}

// Example: Simple validation using before advice with argument capture
fn validate_age(age: i32) -> Result<(), String> {
    if age < 0 || age > 150 {
        Err(format!("Age must be between 0 and 150 (got: {})", age))
    } else {
        Ok(())
    }
}

fn validate_username(username: &str) -> Result<(), String> {
    let len = username.len();
    if len < 3 || len > 20 {
        Err(format!(
            "Username must be 3-20 characters (got: {} chars)",
            len
        ))
    } else if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        Err("Username must contain only alphanumeric characters or underscore".to_string())
    } else {
        Ok(())
    }
}

fn validate_email(email: &str) -> Result<(), String> {
    if !email.contains('@') || !email.contains('.') {
        Err(format!("Invalid email address: {}", email))
    } else {
        Ok(())
    }
}

/// Validation aspect that checks constraints before execution
#[derive(Default)]
struct ConstraintValidator;

impl Aspect for ConstraintValidator {
    fn before(&self, ctx: &JoinPoint) {
        println!("[VALIDATOR] Checking constraints for {}", ctx.function_name);
    }

    fn after(&self, ctx: &JoinPoint, _result: &dyn Any) {
        println!(
            "[VALIDATOR] ✓ All constraints satisfied for {}",
            ctx.function_name
        );
    }

    fn after_error(&self, ctx: &JoinPoint, error: &AspectError) {
        println!(
            "[VALIDATOR] ✗ Validation failed for {}: {:?}",
            ctx.function_name, error
        );
    }
}

// Example functions with validation

#[aspect(ConstraintValidator)]
fn create_user(username: String, email: String, age: i32) -> Result<u64, String> {
    // Validate inputs
    validate_username(&username)?;
    validate_email(&email)?;
    validate_age(age)?;

    println!(
        "  [APP] Creating user: '{}' <{}> age {}",
        username, email, age
    );

    // Simulate user creation
    Ok(12345) // User ID
}

#[aspect(ConstraintValidator)]
fn update_profile(user_id: u64, bio: String) -> Result<(), String> {
    // Validate bio length
    if bio.len() > 500 {
        return Err(format!("Bio too long: {} chars (max 500)", bio.len()));
    }

    println!("  [APP] Updating profile for user {}", user_id);
    println!("  [APP] New bio: {}", bio);
    Ok(())
}

#[aspect(ConstraintValidator)]
fn set_age(user_id: u64, age: i32) -> Result<(), String> {
    validate_age(age)?;

    println!("  [APP] Setting age for user {} to {}", user_id, age);
    Ok(())
}

fn main() {
    println!("=== Validation Aspect Example ===\n");

    // Example 1: Valid user creation
    println!("1. Creating user with valid data:");
    match create_user("alice_123".to_string(), "alice@example.com".to_string(), 25) {
        Ok(user_id) => println!("   ✓ User created with ID: {}\n", user_id),
        Err(e) => println!("   ✗ Failed: {}\n", e),
    }

    // Example 2: Invalid username (too short)
    println!("2. Creating user with invalid username (too short):");
    match create_user("ab".to_string(), "bob@example.com".to_string(), 30) {
        Ok(user_id) => println!("   ✗ Unexpected success: {}\n", user_id),
        Err(e) => println!("   ✓ Validation failed as expected: {}\n", e),
    }

    // Example 3: Invalid email
    println!("3. Creating user with invalid email:");
    match create_user("charlie".to_string(), "not-an-email".to_string(), 28) {
        Ok(user_id) => println!("   ✗ Unexpected success: {}\n", user_id),
        Err(e) => println!("   ✓ Validation failed as expected: {}\n", e),
    }

    // Example 4: Invalid age
    println!("4. Creating user with invalid age:");
    match create_user("dave".to_string(), "dave@example.com".to_string(), 200) {
        Ok(user_id) => println!("   ✗ Unexpected success: {}\n", user_id),
        Err(e) => println!("   ✓ Validation failed as expected: {}\n", e),
    }

    // Example 5: Valid profile update
    println!("5. Updating profile with valid bio:");
    match update_profile(123, "Software developer and Rust enthusiast".to_string()) {
        Ok(_) => println!("   ✓ Profile updated\n"),
        Err(e) => println!("   ✗ Failed: {}\n", e),
    }

    // Example 6: Bio too long
    println!("6. Updating profile with bio too long:");
    let long_bio = "a".repeat(600);
    match update_profile(123, long_bio) {
        Ok(_) => println!("   ✗ Unexpected success\n"),
        Err(e) => println!("   ✓ Validation failed as expected: {}\n", e),
    }

    // Example 7: Valid age update
    println!("7. Setting valid age:");
    match set_age(123, 35) {
        Ok(_) => println!("   ✓ Age updated\n"),
        Err(e) => println!("   ✗ Failed: {}\n", e),
    }

    // Example 8: Invalid age (negative)
    println!("8. Setting invalid age (negative):");
    match set_age(123, -5) {
        Ok(_) => println!("   ✗ Unexpected success\n"),
        Err(e) => println!("   ✓ Validation failed as expected: {}\n", e),
    }

    println!("=== Demo Complete ===");
    println!("\nKey Takeaways:");
    println!("✓ Validation logic separated from business logic");
    println!("✓ Constraints enforced declaratively");
    println!("✓ Clear error messages for validation failures");
    println!("✓ Aspect tracks validation success/failure");
    println!("✓ Reusable validation rules across functions");
    println!("\nNote: Full validation aspects would capture and validate arguments");
    println!("automatically. This example shows the pattern with manual checks.");
}
