//! Security and authorization aspect example.
//!
//! Demonstrates how to implement role-based access control (RBAC)
//! using aspects to enforce security policies declaratively.

use aspect_core::prelude::*;
use aspect_macros::aspect;
use std::any::Any;
use std::sync::RwLock;

/// Simple user representation
#[derive(Debug, Clone)]
struct User {
    username: String,
    roles: Vec<String>,
}

impl User {
    fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }
}

/// Thread-local current user context (simulated)
static CURRENT_USER: RwLock<Option<User>> = RwLock::new(None);

fn set_current_user(user: User) {
    *CURRENT_USER.write().unwrap() = Some(user);
}

fn get_current_user() -> Option<User> {
    CURRENT_USER.read().unwrap().clone()
}

fn clear_current_user() {
    *CURRENT_USER.write().unwrap() = None;
}

/// Authorization aspect that checks user roles before execution
struct AuthorizationAspect {
    required_roles: Vec<String>,
    require_all: bool, // true = all roles required, false = any role required
}

impl AuthorizationAspect {
    fn require_role(role: &str) -> Self {
        Self {
            required_roles: vec![role.to_string()],
            require_all: false,
        }
    }

    fn require_any_role(roles: &[&str]) -> Self {
        Self {
            required_roles: roles.iter().map(|r| r.to_string()).collect(),
            require_all: false,
        }
    }

    fn require_all_roles(roles: &[&str]) -> Self {
        Self {
            required_roles: roles.iter().map(|r| r.to_string()).collect(),
            require_all: true,
        }
    }

    fn check_authorization(&self, user: &User) -> Result<(), String> {
        if self.require_all {
            // All roles required
            for role in &self.required_roles {
                if !user.has_role(role) {
                    return Err(format!(
                        "Access denied: user '{}' missing required role '{}'",
                        user.username, role
                    ));
                }
            }
            Ok(())
        } else {
            // Any role is sufficient
            let role_refs: Vec<&str> = self.required_roles.iter().map(|s| s.as_str()).collect();
            if user.has_any_role(&role_refs) {
                Ok(())
            } else {
                Err(format!(
                    "Access denied: user '{}' needs one of: {}",
                    user.username,
                    self.required_roles.join(", ")
                ))
            }
        }
    }
}

impl Aspect for AuthorizationAspect {
    fn before(&self, ctx: &JoinPoint) {
        match get_current_user() {
            None => {
                panic!(
                    "Authorization failed for {}: No user logged in",
                    ctx.function_name
                );
            }
            Some(user) => {
                if let Err(msg) = self.check_authorization(&user) {
                    panic!("Authorization failed for {}: {}", ctx.function_name, msg);
                }
                println!(
                    "[AUTH] ✓ User '{}' authorized for {}",
                    user.username, ctx.function_name
                );
            }
        }
    }
}

/// Audit aspect that logs all security-sensitive operations
#[derive(Default)]
struct AuditAspect;

impl Aspect for AuditAspect {
    fn before(&self, ctx: &JoinPoint) {
        let user = get_current_user()
            .map(|u| u.username)
            .unwrap_or_else(|| "anonymous".to_string());

        println!(
            "[AUDIT] User '{}' accessing {} at {}:{}",
            user, ctx.function_name, ctx.location.file, ctx.location.line
        );
    }

    fn after(&self, ctx: &JoinPoint, _result: &dyn Any) {
        let user = get_current_user()
            .map(|u| u.username)
            .unwrap_or_else(|| "anonymous".to_string());

        println!("[AUDIT] User '{}' completed {}", user, ctx.function_name);
    }

    fn after_error(&self, ctx: &JoinPoint, error: &AspectError) {
        let user = get_current_user()
            .map(|u| u.username)
            .unwrap_or_else(|| "anonymous".to_string());

        println!(
            "[AUDIT] ⚠ User '{}' failed {}: {:?}",
            user, ctx.function_name, error
        );
    }
}

// Example protected functions

#[aspect(AuthorizationAspect::require_role("admin"))]
#[aspect(AuditAspect::default())]
fn delete_user(user_id: u64) -> Result<(), String> {
    println!("  [SYSTEM] Deleting user {}", user_id);
    Ok(())
}

#[aspect(AuthorizationAspect::require_any_role(&["admin", "moderator"]))]
#[aspect(AuditAspect::default())]
fn ban_user(user_id: u64, reason: String) -> Result<(), String> {
    println!("  [SYSTEM] Banning user {} (reason: {})", user_id, reason);
    Ok(())
}

#[aspect(AuthorizationAspect::require_role("user"))]
#[aspect(AuditAspect::default())]
fn view_profile(user_id: u64) -> Result<String, String> {
    println!("  [SYSTEM] Fetching profile for user {}", user_id);
    Ok(format!("Profile data for user {}", user_id))
}

#[aspect(AuditAspect::default())]
fn public_endpoint() -> String {
    println!("  [SYSTEM] Public endpoint accessed");
    "Public data".to_string()
}

fn main() {
    println!("=== Security & Authorization Aspect Example ===\n");

    // Example 1: Admin user accessing admin function
    println!("1. Admin user deleting a user:");
    set_current_user(User {
        username: "admin_user".to_string(),
        roles: vec!["admin".to_string(), "user".to_string()],
    });

    match delete_user(42) {
        Ok(_) => println!("   ✓ Operation succeeded\n"),
        Err(e) => println!("   ✗ Operation failed: {}\n", e),
    }

    // Example 2: Moderator banning a user (requires admin OR moderator)
    println!("2. Moderator banning a user:");
    set_current_user(User {
        username: "mod_user".to_string(),
        roles: vec!["moderator".to_string(), "user".to_string()],
    });

    match ban_user(99, "spam".to_string()) {
        Ok(_) => println!("   ✓ Operation succeeded\n"),
        Err(e) => println!("   ✗ Operation failed: {}\n", e),
    }

    // Example 3: Regular user viewing profile
    println!("3. Regular user viewing profile:");
    set_current_user(User {
        username: "regular_user".to_string(),
        roles: vec!["user".to_string()],
    });

    match view_profile(42) {
        Ok(data) => println!("   ✓ Got: {}\n", data),
        Err(e) => println!("   ✗ Failed: {}\n", e),
    }

    // Example 4: Public endpoint (no auth required)
    println!("4. Public endpoint (no authorization):");
    let result = public_endpoint();
    println!("   ✓ Got: {}\n", result);

    // Example 5: Unauthorized access attempt (will panic)
    println!("5. Regular user trying to delete (will panic):");
    println!("   Attempting unauthorized operation...");

    let result = std::panic::catch_unwind(|| delete_user(123));

    match result {
        Ok(_) => println!("   ✗ Unexpected success!"),
        Err(_) => println!("   ✓ Access denied as expected (caught panic)\n"),
    }

    // Example 6: No user logged in (will panic)
    println!("6. No user logged in (will panic):");
    clear_current_user();

    let result = std::panic::catch_unwind(|| view_profile(42));

    match result {
        Ok(_) => println!("   ✗ Unexpected success!"),
        Err(_) => println!("   ✓ Denied as expected (caught panic)\n"),
    }

    println!("=== Demo Complete ===");
    println!("\nKey Takeaways:");
    println!("✓ Authorization logic separated from business code");
    println!("✓ Role-based access control enforced declaratively");
    println!("✓ Audit logging automatic for all protected functions");
    println!("✓ Multiple aspects compose cleanly (auth + audit)");
    println!("✓ Security policies centralized and reusable");
}
