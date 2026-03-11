//! Authorization aspect for role-based access control.

use aspect_core::{Aspect, JoinPoint};
use std::collections::HashSet;
use std::sync::Arc;

/// Role-based access control aspect.
///
/// Enforces authorization checks before function execution based on
/// required roles or permissions.
///
/// # Example
///
/// ```rust,ignore
/// use aspect_std::AuthorizationAspect;
/// use aspect_macros::aspect;
///
/// // Require "admin" role
/// let auth = AuthorizationAspect::require_role("admin", || {
///     get_current_user_roles()
/// });
///
/// #[aspect(auth)]
/// fn delete_user(user_id: u64) -> Result<(), String> {
///     // Only admins can delete users
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct AuthorizationAspect {
    required_roles: Arc<HashSet<String>>,
    role_provider: Arc<dyn Fn() -> HashSet<String> + Send + Sync>,
    mode: AuthMode,
}

/// Authorization mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AuthMode {
    /// Require ALL specified roles.
    RequireAll,
    /// Require ANY of the specified roles.
    RequireAny,
}

impl AuthorizationAspect {
    /// Create an authorization aspect that requires a specific role.
    ///
    /// # Arguments
    /// * `role` - The required role
    /// * `role_provider` - Function that returns the current user's roles
    ///
    /// # Example
    /// ```rust,ignore
    /// let auth = AuthorizationAspect::require_role("admin", || {
    ///     vec!["admin".to_string()].into_iter().collect()
    /// });
    /// ```
    pub fn require_role<F>(role: &str, role_provider: F) -> Self
    where
        F: Fn() -> HashSet<String> + Send + Sync + 'static,
    {
        let mut roles = HashSet::new();
        roles.insert(role.to_string());

        Self {
            required_roles: Arc::new(roles),
            role_provider: Arc::new(role_provider),
            mode: AuthMode::RequireAll,
        }
    }

    /// Create an authorization aspect that requires multiple roles.
    ///
    /// # Arguments
    /// * `roles` - The required roles
    /// * `role_provider` - Function that returns the current user's roles
    /// * `mode` - Whether to require ALL or ANY of the roles
    ///
    /// # Example
    /// ```rust,ignore
    /// let auth = AuthorizationAspect::require_roles(
    ///     &["admin", "moderator"],
    ///     || get_current_roles(),
    ///     AuthMode::RequireAny
    /// );
    /// ```
    pub fn require_roles<F>(roles: &[&str], role_provider: F, mode: AuthMode) -> Self
    where
        F: Fn() -> HashSet<String> + Send + Sync + 'static,
    {
        let role_set: HashSet<String> = roles.iter().map(|r| r.to_string()).collect();

        Self {
            required_roles: Arc::new(role_set),
            role_provider: Arc::new(role_provider),
            mode,
        }
    }

    /// Check if the current user is authorized.
    fn check_authorization(&self) -> Result<(), String> {
        let current_roles = (self.role_provider)();

        let authorized = match self.mode {
            AuthMode::RequireAll => {
                // User must have ALL required roles
                self.required_roles
                    .iter()
                    .all(|r| current_roles.contains(r))
            }
            AuthMode::RequireAny => {
                // User must have ANY of the required roles
                self.required_roles
                    .iter()
                    .any(|r| current_roles.contains(r))
            }
        };

        if authorized {
            Ok(())
        } else {
            let required: Vec<_> = self.required_roles.iter().cloned().collect();
            let mode_str = match self.mode {
                AuthMode::RequireAll => "all",
                AuthMode::RequireAny => "any",
            };
            Err(format!(
                "Access denied: requires {} of roles {:?}",
                mode_str, required
            ))
        }
    }
}

impl Aspect for AuthorizationAspect {
    fn before(&self, ctx: &JoinPoint) {
        if let Err(msg) = self.check_authorization() {
            panic!("Authorization failed for {}: {}", ctx.function_name, msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_roles(roles: Vec<&str>) -> HashSet<String> {
        roles.into_iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_require_role_success() {
        let auth = AuthorizationAspect::require_role("admin", || mock_roles(vec!["admin"]));

        assert!(auth.check_authorization().is_ok());
    }

    #[test]
    fn test_require_role_failure() {
        let auth = AuthorizationAspect::require_role("admin", || mock_roles(vec!["user"]));

        assert!(auth.check_authorization().is_err());
    }

    #[test]
    fn test_require_all_success() {
        let auth = AuthorizationAspect::require_roles(
            &["admin", "moderator"],
            || mock_roles(vec!["admin", "moderator", "user"]),
            AuthMode::RequireAll,
        );

        assert!(auth.check_authorization().is_ok());
    }

    #[test]
    fn test_require_all_failure() {
        let auth = AuthorizationAspect::require_roles(
            &["admin", "moderator"],
            || mock_roles(vec!["admin"]),
            AuthMode::RequireAll,
        );

        assert!(auth.check_authorization().is_err());
    }

    #[test]
    fn test_require_any_success() {
        let auth = AuthorizationAspect::require_roles(
            &["admin", "moderator"],
            || mock_roles(vec!["moderator"]),
            AuthMode::RequireAny,
        );

        assert!(auth.check_authorization().is_ok());
    }

    #[test]
    fn test_require_any_failure() {
        let auth = AuthorizationAspect::require_roles(
            &["admin", "moderator"],
            || mock_roles(vec!["user"]),
            AuthMode::RequireAny,
        );

        assert!(auth.check_authorization().is_err());
    }

    #[test]
    fn test_empty_roles() {
        let auth = AuthorizationAspect::require_role("admin", || mock_roles(vec![]));

        assert!(auth.check_authorization().is_err());
    }

    #[test]
    fn test_multiple_roles_user() {
        let auth = AuthorizationAspect::require_role("admin", || {
            mock_roles(vec!["user", "moderator", "admin"])
        });

        assert!(auth.check_authorization().is_ok());
    }
}
