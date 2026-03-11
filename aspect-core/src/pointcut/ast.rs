//! Abstract Syntax Tree for pointcut expressions.

use super::parser::parse_pointcut;
use super::pattern::{ExecutionPattern, ModulePattern};

/// A pointcut expression that matches joinpoints (functions).
#[derive(Debug, Clone, PartialEq)]
pub enum Pointcut {
    /// Match function execution: `execution(pub fn save(..))`
    Execution(ExecutionPattern),

    /// Match functions within a module: `within(crate::api)`
    Within(ModulePattern),

    /// Logical AND: both pointcuts must match
    And(Box<Pointcut>, Box<Pointcut>),

    /// Logical OR: either pointcut must match
    Or(Box<Pointcut>, Box<Pointcut>),

    /// Logical NOT: pointcut must not match
    Not(Box<Pointcut>),
}

impl Pointcut {
    /// Parse a pointcut expression from a string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use aspect_core::pointcut::Pointcut;
    ///
    /// let pc = Pointcut::parse("execution(pub fn *(..))").unwrap();
    /// let pc = Pointcut::parse("within(crate::api)").unwrap();
    /// let pc = Pointcut::parse("execution(pub fn *(..)) && within(crate::api)").unwrap();
    /// ```
    pub fn parse(input: &str) -> Result<Self, String> {
        parse_pointcut(input)
    }

    /// Create an AND pointcut.
    pub fn and(self, other: Pointcut) -> Pointcut {
        Pointcut::And(Box::new(self), Box::new(other))
    }

    /// Create an OR pointcut.
    pub fn or(self, other: Pointcut) -> Pointcut {
        Pointcut::Or(Box::new(self), Box::new(other))
    }

    /// Create a NOT pointcut.
    pub fn not(self) -> Pointcut {
        Pointcut::Not(Box::new(self))
    }

    /// Convenience method to create an execution pointcut for all public functions.
    ///
    /// Equivalent to `Pointcut::parse("execution(pub fn *(..))")`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use aspect_core::pointcut::Pointcut;
    ///
    /// let pc = Pointcut::public_functions();
    /// ```
    pub fn public_functions() -> Self {
        use super::pattern::{ExecutionPattern, NamePattern, Visibility};
        Pointcut::Execution(ExecutionPattern {
            visibility: Some(Visibility::Public),
            name: NamePattern::Wildcard,
            return_type: None,
        })
    }

    /// Convenience method to create an execution pointcut for all functions.
    ///
    /// Equivalent to `Pointcut::parse("execution(fn *(..))")`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use aspect_core::pointcut::Pointcut;
    ///
    /// let pc = Pointcut::all_functions();
    /// ```
    pub fn all_functions() -> Self {
        use super::pattern::{ExecutionPattern, NamePattern};
        Pointcut::Execution(ExecutionPattern {
            visibility: None,
            name: NamePattern::Wildcard,
            return_type: None,
        })
    }

    /// Convenience method to create a within pointcut for a module.
    ///
    /// # Example
    ///
    /// ```rust
    /// use aspect_core::pointcut::Pointcut;
    ///
    /// let pc = Pointcut::within_module("crate::api");
    /// ```
    pub fn within_module(module_path: impl Into<String>) -> Self {
        use super::pattern::ModulePattern;
        Pointcut::Within(ModulePattern {
            path: module_path.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pointcut_combinators() {
        use crate::pointcut::pattern::*;

        let pc1 = Pointcut::Execution(ExecutionPattern {
            visibility: Some(Visibility::Public),
            name: NamePattern::Wildcard,
            return_type: None,
        });

        let pc2 = Pointcut::Within(ModulePattern {
            path: "crate::api".to_string(),
        });

        let and_pc = pc1.clone().and(pc2.clone());
        assert!(matches!(and_pc, Pointcut::And(_, _)));

        let or_pc = pc1.clone().or(pc2.clone());
        assert!(matches!(or_pc, Pointcut::Or(_, _)));

        let not_pc = pc1.not();
        assert!(matches!(not_pc, Pointcut::Not(_)));
    }
}
