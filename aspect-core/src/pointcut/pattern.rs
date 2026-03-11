//! Pattern types for matching functions.

/// Function visibility pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    /// Public: `pub`
    Public,
    /// Crate-public: `pub(crate)`
    Crate,
    /// Super-public: `pub(super)`
    Super,
    /// Private (no visibility modifier)
    Private,
}

impl Visibility {
    /// Check if a visibility string matches this pattern.
    pub fn matches(&self, vis: &str) -> bool {
        match (self, vis) {
            (Visibility::Public, "pub") => true,
            (Visibility::Crate, "pub(crate)") => true,
            (Visibility::Super, "pub(super)") => true,
            (Visibility::Private, "") => true,
            _ => false,
        }
    }
}

/// Function name pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamePattern {
    /// Match any name: `*`
    Wildcard,
    /// Exact name match: `save_user`
    Exact(String),
    /// Prefix match: `save*`
    Prefix(String),
    /// Suffix match: `*_user`
    Suffix(String),
    /// Contains match: `*save*`
    Contains(String),
}

impl NamePattern {
    /// Check if a function name matches this pattern.
    pub fn matches(&self, name: &str) -> bool {
        match self {
            NamePattern::Wildcard => true,
            NamePattern::Exact(expected) => name == expected,
            NamePattern::Prefix(prefix) => name.starts_with(prefix),
            NamePattern::Suffix(suffix) => name.ends_with(suffix),
            NamePattern::Contains(substring) => name.contains(substring),
        }
    }
}

/// Execution pattern: matches function signatures.
///
/// Examples:
/// - `execution(pub fn *(..))` - all public functions
/// - `execution(fn save(..))` - function named "save"
/// - `execution(pub fn save*(..) -> Result<*, *>)` - public functions starting with "save" returning Result
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionPattern {
    /// Visibility pattern (pub, pub(crate), etc.)
    pub visibility: Option<Visibility>,

    /// Function name pattern
    pub name: NamePattern,

    /// Return type pattern (simplified for now)
    pub return_type: Option<String>,
}

impl ExecutionPattern {
    /// Create a pattern that matches all functions.
    pub fn any() -> Self {
        Self {
            visibility: None,
            name: NamePattern::Wildcard,
            return_type: None,
        }
    }

    /// Create a pattern that matches public functions.
    pub fn public() -> Self {
        Self {
            visibility: Some(Visibility::Public),
            name: NamePattern::Wildcard,
            return_type: None,
        }
    }

    /// Create a pattern that matches a specific function name.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            visibility: None,
            name: NamePattern::Exact(name.into()),
            return_type: None,
        }
    }
}

/// Module pattern: matches functions by module path.
///
/// Examples:
/// - `within(crate::api)` - functions in the api module
/// - `within(crate::api::users)` - functions in the users submodule
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModulePattern {
    /// Module path to match (e.g., "crate::api")
    pub path: String,
}

impl ModulePattern {
    /// Create a new module pattern.
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    /// Check if a module path matches this pattern.
    ///
    /// Supports exact match and prefix match (for submodules).
    pub fn matches_path(&self, module_path: &str) -> bool {
        module_path == self.path || module_path.starts_with(&format!("{}::", self.path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visibility_matches() {
        assert!(Visibility::Public.matches("pub"));
        assert!(Visibility::Crate.matches("pub(crate)"));
        assert!(Visibility::Private.matches(""));
        assert!(!Visibility::Public.matches("pub(crate)"));
    }

    #[test]
    fn test_name_pattern_matches() {
        let wildcard = NamePattern::Wildcard;
        assert!(wildcard.matches("anything"));
        assert!(wildcard.matches("save_user"));

        let exact = NamePattern::Exact("save".to_string());
        assert!(exact.matches("save"));
        assert!(!exact.matches("save_user"));

        let prefix = NamePattern::Prefix("save".to_string());
        assert!(prefix.matches("save"));
        assert!(prefix.matches("save_user"));
        assert!(!prefix.matches("update_user"));

        let suffix = NamePattern::Suffix("_user".to_string());
        assert!(suffix.matches("save_user"));
        assert!(suffix.matches("update_user"));
        assert!(!suffix.matches("save"));
    }

    #[test]
    fn test_module_pattern_matches() {
        let pattern = ModulePattern::new("crate::api");

        assert!(pattern.matches_path("crate::api"));
        assert!(pattern.matches_path("crate::api::users"));
        assert!(pattern.matches_path("crate::api::users::models"));
        assert!(!pattern.matches_path("crate::internal"));
        assert!(!pattern.matches_path("crate"));
    }

    #[test]
    fn test_execution_pattern_builders() {
        let any = ExecutionPattern::any();
        assert_eq!(any.name, NamePattern::Wildcard);
        assert!(any.visibility.is_none());

        let public = ExecutionPattern::public();
        assert_eq!(public.visibility, Some(Visibility::Public));

        let named = ExecutionPattern::named("save_user");
        assert_eq!(named.name, NamePattern::Exact("save_user".to_string()));
    }
}
