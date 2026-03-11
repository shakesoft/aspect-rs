//! Real-World API Server Example
//!
//! Demonstrates multiple aspects working together in a realistic scenario.

use aspect_core::prelude::*;
use aspect_macros::aspect;
use aspect_std::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// Domain Models
// ============================================================================

#[derive(Debug, Clone)]
struct User {
    id: u64,
    username: String,
    email: String,
}

type Database = Arc<Mutex<HashMap<u64, User>>>;

fn init_database() -> Database {
    let db = Arc::new(Mutex::new(HashMap::new()));
    {
        let mut users = db.lock().unwrap();
        users.insert(
            1,
            User {
                id: 1,
                username: "alice".to_string(),
                email: "alice@example.com".to_string(),
            },
        );
        users.insert(
            2,
            User {
                id: 2,
                username: "bob".to_string(),
                email: "bob@example.com".to_string(),
            },
        );
    }
    db
}

// ============================================================================
// API Handlers with Aspects
// ============================================================================

/// GET /users/:id - with logging and timing
#[aspect(LoggingAspect::new())]
#[aspect(TimingAspect::new())]
fn get_user(db: &Database, id: u64) -> Result<Option<User>, AspectError> {
    println!("  [HANDLER] GET /users/{}", id);
    std::thread::sleep(std::time::Duration::from_millis(10)); // Simulate work
    Ok(db.lock().unwrap().get(&id).cloned())
}

/// POST /users - with logging and timing
#[aspect(LoggingAspect::new())]
#[aspect(TimingAspect::new())]
fn create_user(
    db: &Database,
    id: u64,
    username: String,
    email: String,
) -> Result<User, AspectError> {
    println!("  [HANDLER] POST /users");

    // Validation
    if username.is_empty() {
        return Err(AspectError::execution("Username cannot be empty"));
    }
    if !email.contains('@') {
        return Err(AspectError::execution("Invalid email format"));
    }

    std::thread::sleep(std::time::Duration::from_millis(15)); // Simulate work

    let user = User {
        id,
        username: username.to_string(),
        email: email.to_string(),
    };
    db.lock().unwrap().insert(id, user.clone());
    Ok(user)
}

/// GET /users - with logging and timing
#[aspect(LoggingAspect::new())]
#[aspect(TimingAspect::new())]
fn list_users(db: &Database) -> Result<Vec<User>, AspectError> {
    println!("  [HANDLER] GET /users");
    std::thread::sleep(std::time::Duration::from_millis(20)); // Simulate work
    Ok(db.lock().unwrap().values().cloned().collect())
}

/// DELETE /users/:id - with logging and timing
#[aspect(LoggingAspect::new())]
#[aspect(TimingAspect::new())]
fn delete_user(db: &Database, id: u64) -> Result<bool, AspectError> {
    println!("  [HANDLER] DELETE /users/{}", id);
    std::thread::sleep(std::time::Duration::from_millis(12)); // Simulate work
    Ok(db.lock().unwrap().remove(&id).is_some())
}

// ============================================================================
// Main Application
// ============================================================================

fn main() {
    println!("=== API Server with Aspects Demo ===\n");
    println!("This example shows multiple aspects applied to API handlers:");
    println!("- LoggingAspect: Tracks entry/exit of each handler");
    println!("- TimingAspect: Measures execution time\n");

    let db = init_database();

    // Simulate API requests
    println!("1. GET /users/1 (existing user)");
    match get_user(&db, 1) {
        Ok(Some(user)) => println!("   Found: {} ({})\n", user.username, user.email),
        Ok(None) => println!("   Not found\n"),
        Err(e) => println!("   Error: {:?}\n", e),
    }

    println!("2. GET /users/999 (non-existent user)");
    match get_user(&db, 999) {
        Ok(Some(user)) => println!("   Found: {}\n", user.username),
        Ok(None) => println!("   Not found\n"),
        Err(e) => println!("   Error: {:?}\n", e),
    }

    println!("3. POST /users (create new user)");
    match create_user(
        &db,
        3,
        "charlie".to_string(),
        "charlie@example.com".to_string(),
    ) {
        Ok(user) => println!("   Created: {} (ID: {})\n", user.username, user.id),
        Err(e) => println!("   Error: {:?}\n", e),
    }

    println!("4. POST /users (invalid email)");
    match create_user(&db, 4, "dave".to_string(), "invalid-email".to_string()) {
        Ok(user) => println!("   Created: {}\n", user.username),
        Err(e) => println!("   Validation failed: {}\n", e),
    }

    println!("5. GET /users (list all)");
    match list_users(&db) {
        Ok(users) => {
            println!("   Found {} users:", users.len());
            for user in users {
                println!("     - {} ({})", user.username, user.email);
            }
            println!();
        }
        Err(e) => println!("   Error: {:?}\n", e),
    }

    println!("6. DELETE /users/2");
    match delete_user(&db, 2) {
        Ok(true) => println!("   Deleted successfully\n"),
        Ok(false) => println!("   User not found\n"),
        Err(e) => println!("   Error: {:?}\n", e),
    }

    println!("=== Demo Complete ===\n");
    println!("Key Takeaways:");
    println!("✓ Logging automatically applied to all handlers");
    println!("✓ Timing measured for each request");
    println!("✓ Error handling integrated with aspects");
    println!("✓ Clean separation of concerns");
    println!("✓ No manual instrumentation needed!");
}
