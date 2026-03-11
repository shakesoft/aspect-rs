//! Structured logging aspect with configurable levels.

use aspect_core::{Aspect, AspectError, JoinPoint};
use std::any::Any;

/// Logging aspect with configurable log levels and output.
///
/// Provides structured logging for function entry, exit, and errors.
///
/// # Example
///
/// ```rust,ignore
/// use aspect_std::LoggingAspect;
/// use aspect_macros::aspect;
///
/// #[aspect(LoggingAspect::new())]
/// fn my_function(x: i32) -> Result<i32, String> {
///     Ok(x * 2)
/// }
/// ```
#[derive(Clone)]
pub struct LoggingAspect {
    level: LogLevel,
    log_args: bool,
    log_result: bool,
}

/// Log level for the logging aspect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Trace level (most verbose)
    Trace,
    /// Debug level
    Debug,
    /// Info level (default)
    Info,
    /// Warn level
    Warn,
    /// Error level
    Error,
}

impl LoggingAspect {
    /// Create a new logging aspect with Info level.
    pub fn new() -> Self {
        Self {
            level: LogLevel::Info,
            log_args: false,
            log_result: false,
        }
    }

    /// Set the log level.
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    /// Enable logging of function arguments (disabled by default).
    pub fn log_args(mut self) -> Self {
        self.log_args = true;
        self
    }

    /// Enable logging of function results (disabled by default).
    pub fn log_result(mut self) -> Self {
        self.log_result = true;
        self
    }

    fn log(&self, level: LogLevel, message: &str) {
        if level as u8 >= self.level as u8 {
            match level {
                LogLevel::Trace => log::trace!("{}", message),
                LogLevel::Debug => log::debug!("{}", message),
                LogLevel::Info => log::info!("{}", message),
                LogLevel::Warn => log::warn!("{}", message),
                LogLevel::Error => log::error!("{}", message),
            }
        }
    }
}

impl Default for LoggingAspect {
    fn default() -> Self {
        Self::new()
    }
}

impl Aspect for LoggingAspect {
    fn before(&self, ctx: &JoinPoint) {
        let message = format!(
            "[ENTRY] {} ({}:{})",
            ctx.function_name, ctx.location.file, ctx.location.line
        );
        self.log(self.level, &message);
    }

    fn after(&self, ctx: &JoinPoint, result: &dyn Any) {
        let mut message = format!("[EXIT] {}", ctx.function_name);

        if self.log_result {
            message.push_str(&format!(
                " (result: {:?})",
                std::any::type_name_of_val(result)
            ));
        }

        self.log(self.level, &message);
    }

    fn after_error(&self, ctx: &JoinPoint, error: &AspectError) {
        let message = format!("[ERROR] {} failed: {:?}", ctx.function_name, error);
        self.log(LogLevel::Error, &message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_aspect_creation() {
        let aspect = LoggingAspect::new();
        assert_eq!(aspect.level, LogLevel::Info);
        assert!(!aspect.log_args);
        assert!(!aspect.log_result);
    }

    #[test]
    fn test_logging_aspect_builder() {
        let aspect = LoggingAspect::new()
            .with_level(LogLevel::Debug)
            .log_args()
            .log_result();

        assert_eq!(aspect.level, LogLevel::Debug);
        assert!(aspect.log_args);
        assert!(aspect.log_result);
    }

    #[test]
    fn test_logging_aspect_before() {
        let aspect = LoggingAspect::new();
        let ctx = JoinPoint {
            function_name: "test_function",
            module_path: "test::module",
            location: aspect_core::Location {
                file: "test.rs",
                line: 42,
            },
            args: vec![],
        };

        // Should not panic
        aspect.before(&ctx);
    }
}
