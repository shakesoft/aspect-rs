//! Error types for aspect execution.

use std::error::Error;
use std::fmt;

/// Errors that can occur during aspect execution.
///
/// This type wraps errors that occur during the execution of advised functions
/// or within aspect logic itself.
#[derive(Debug)]
pub enum AspectError {
    /// An error occurred during function execution
    ExecutionError {
        /// The underlying error message
        message: String,
        /// Optional source error
        source: Option<Box<dyn Error + Send + Sync>>,
    },

    /// An error occurred during aspect weaving
    WeavingError {
        /// Description of the weaving error
        message: String,
    },

    /// A custom error defined by user code
    Custom(Box<dyn Error + Send + Sync>),
}

impl AspectError {
    /// Creates a new execution error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use aspect_core::error::AspectError;
    ///
    /// let err = AspectError::execution("Database connection failed");
    /// ```
    pub fn execution(message: impl Into<String>) -> Self {
        Self::ExecutionError {
            message: message.into(),
            source: None,
        }
    }

    /// Creates a new execution error with a source.
    ///
    /// # Example
    ///
    /// ```rust
    /// use aspect_core::error::AspectError;
    /// use std::io;
    ///
    /// let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    /// let err = AspectError::execution_with_source("Failed to read file", io_err);
    /// ```
    pub fn execution_with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self::ExecutionError {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Creates a new weaving error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use aspect_core::error::AspectError;
    ///
    /// let err = AspectError::weaving("Invalid pointcut expression");
    /// ```
    pub fn weaving(message: impl Into<String>) -> Self {
        Self::WeavingError {
            message: message.into(),
        }
    }

    /// Creates a custom error from any error type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use aspect_core::error::AspectError;
    /// use std::io;
    ///
    /// let io_err = io::Error::new(io::ErrorKind::Other, "custom error");
    /// let err = AspectError::custom(io_err);
    /// ```
    pub fn custom(error: impl Error + Send + Sync + 'static) -> Self {
        Self::Custom(Box::new(error))
    }
}

impl fmt::Display for AspectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExecutionError { message, .. } => {
                write!(f, "Execution error: {}", message)
            }
            Self::WeavingError { message } => {
                write!(f, "Weaving error: {}", message)
            }
            Self::Custom(err) => write!(f, "Custom error: {}", err),
        }
    }
}

impl Error for AspectError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ExecutionError { source, .. } => source
                .as_ref()
                .map(|e| e.as_ref() as &(dyn Error + 'static)),
            Self::Custom(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

// Conversion from common error types
impl From<String> for AspectError {
    fn from(s: String) -> Self {
        Self::execution(s)
    }
}

impl From<&str> for AspectError {
    fn from(s: &str) -> Self {
        Self::execution(s)
    }
}

impl From<Box<dyn Error + Send + Sync>> for AspectError {
    fn from(err: Box<dyn Error + Send + Sync>) -> Self {
        Self::Custom(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_execution_error() {
        let err = AspectError::execution("test error");
        assert!(matches!(err, AspectError::ExecutionError { .. }));
        assert_eq!(err.to_string(), "Execution error: test error");
    }

    #[test]
    fn test_execution_error_with_source() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = AspectError::execution_with_source("read failed", io_err);

        assert!(err.source().is_some());
    }

    #[test]
    fn test_weaving_error() {
        let err = AspectError::weaving("invalid pointcut");
        assert!(matches!(err, AspectError::WeavingError { .. }));
        assert_eq!(err.to_string(), "Weaving error: invalid pointcut");
    }

    #[test]
    fn test_custom_error() {
        let io_err = io::Error::new(io::ErrorKind::Other, "custom");
        let err = AspectError::custom(io_err);

        assert!(matches!(err, AspectError::Custom(_)));
    }

    #[test]
    fn test_from_string() {
        let err: AspectError = "error message".into();
        assert!(matches!(err, AspectError::ExecutionError { .. }));
    }
}
