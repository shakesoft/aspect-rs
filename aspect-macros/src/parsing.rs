//! Parsing utilities for aspect macro attributes.

use syn::{Expr, Result};

/// Information about the aspect to apply.
pub struct AspectInfo {
    /// The expression that evaluates to the aspect instance
    pub aspect_expr: Expr,
    /// Whether the aspect type explicitly overrides `Aspect::around`.
    pub has_custom_sync_around: bool,
    /// Whether the aspect type explicitly overrides `AsyncAspect::around`.
    pub has_custom_async_around: bool,
}

impl AspectInfo {
    /// Parse aspect information from the attribute syntax.
    pub fn parse(aspect_expr: Expr) -> Result<Self> {
        Ok(Self {
            aspect_expr,
            has_custom_sync_around: false,
            has_custom_async_around: false,
        })
    }
}
