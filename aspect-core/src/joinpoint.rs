//! JoinPoint and related types.
//!
//! A joinpoint represents a specific point in program execution where an aspect
//! can be applied, such as a function call.

use crate::error::AspectError;
use std::any::Any;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

/// Information about a specific point in program execution.
///
/// A `JoinPoint` provides context about where an aspect is being applied,
/// including the function name, module path, and source location.
#[derive(Debug)]
pub struct JoinPoint {
    /// The name of the function being called
    pub function_name: &'static str,

    /// The module path containing the function
    pub module_path: &'static str,

    /// Source code location information
    pub location: Location,

    /// Arguments passed to the function
    pub args: Vec<Box<dyn Any>>,
}

impl JoinPoint {
    /// Creates a new JoinPoint.
    pub fn new(
        function_name: &'static str,
        module_path: &'static str,
        location: Location,
        args: Vec<Box<dyn Any>>,
    ) -> Self {
        Self {
            function_name,
            module_path,
            location,
            args,
        }
    }

    /// Returns the fully qualified name of the function.
    pub fn qualified_name(&self) -> String {
        format!("{}::{}", self.module_path, self.function_name)
    }
}

impl fmt::Display for JoinPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}::{}@{}:{}",
            self.module_path, self.function_name, self.location.file, self.location.line
        )
    }
}

/// Async joinpoint context used by `AsyncAspect`.
#[derive(Debug)]
pub struct AsyncJoinPoint {
    /// The name of the function being called.
    pub function_name: &'static str,

    /// The module path containing the function.
    pub module_path: &'static str,

    /// Source code location information.
    pub location: Location,

    /// Arguments passed to the function.
    pub args: Vec<Box<dyn Any + Send + Sync>>,
}

impl AsyncJoinPoint {
    /// Creates a new async joinpoint.
    pub fn new(
        function_name: &'static str,
        module_path: &'static str,
        location: Location,
        args: Vec<Box<dyn Any + Send + Sync>>,
    ) -> Self {
        Self {
            function_name,
            module_path,
            location,
            args,
        }
    }

    /// Returns the fully qualified name of the function.
    pub fn qualified_name(&self) -> String {
        format!("{}::{}", self.module_path, self.function_name)
    }
}

impl fmt::Display for AsyncJoinPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}::{}@{}:{}",
            self.module_path, self.function_name, self.location.file, self.location.line
        )
    }
}

/// Source code location information.
#[derive(Debug, Clone, Copy)]
pub struct Location {
    /// The source file path
    pub file: &'static str,

    /// The line number in the source file
    pub line: u32,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

/// A proceeding joinpoint that can be used in "around" advice.
pub struct ProceedingJoinPoint<'a> {
    /// The original function to execute
    inner: Box<dyn FnOnce() -> Result<Box<dyn Any>, AspectError> + 'a>,

    /// Context information about this joinpoint
    context: JoinPoint,
}

impl<'a> ProceedingJoinPoint<'a> {
    /// Creates a new ProceedingJoinPoint.
    pub fn new<F>(f: F, context: JoinPoint) -> Self
    where
        F: FnOnce() -> Result<Box<dyn Any>, AspectError> + 'a,
    {
        Self {
            inner: Box::new(f),
            context,
        }
    }

    /// Proceeds with the original function execution.
    pub fn proceed(self) -> Result<Box<dyn Any>, AspectError> {
        (self.inner)()
    }

    /// Splits the proceeding joinpoint into its context and executable closure.
    pub fn into_parts(
        self,
    ) -> (
        JoinPoint,
        Box<dyn FnOnce() -> Result<Box<dyn Any>, AspectError> + 'a>,
    ) {
        (self.context, self.inner)
    }

    /// Returns a reference to the joinpoint context.
    pub fn context(&self) -> &JoinPoint {
        &self.context
    }

    /// Returns a reference to the arguments passed to the function.
    pub fn args(&self) -> &[Box<dyn Any>] {
        &self.context.args
    }
}

impl<'a> fmt::Debug for ProceedingJoinPoint<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProceedingJoinPoint")
            .field("context", &self.context)
            .finish()
    }
}

/// Boxed future used by `AsyncProceedingJoinPoint`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// An async proceeding joinpoint used by `AsyncAspect`.
pub struct AsyncProceedingJoinPoint<'a> {
    inner: Box<
        dyn FnOnce() -> BoxFuture<'a, Result<Box<dyn Any + Send + Sync>, AspectError>> + Send + 'a,
    >,
    context: AsyncJoinPoint,
}

impl<'a> AsyncProceedingJoinPoint<'a> {
    /// Creates a new async proceeding joinpoint.
    pub fn new<F>(f: F, context: AsyncJoinPoint) -> Self
    where
        F: FnOnce() -> BoxFuture<'a, Result<Box<dyn Any + Send + Sync>, AspectError>> + Send + 'a,
    {
        Self {
            inner: Box::new(f),
            context,
        }
    }

    /// Proceeds with the original async function execution.
    pub async fn proceed(self) -> Result<Box<dyn Any + Send + Sync>, AspectError> {
        (self.inner)().await
    }

    /// Splits the async proceeding joinpoint into its context and executable closure.
    pub fn into_parts(
        self,
    ) -> (
        AsyncJoinPoint,
        Box<
            dyn FnOnce() -> BoxFuture<'a, Result<Box<dyn Any + Send + Sync>, AspectError>>
                + Send
                + 'a,
        >,
    ) {
        (self.context, self.inner)
    }

    /// Returns a reference to the joinpoint context.
    pub fn context(&self) -> &AsyncJoinPoint {
        &self.context
    }

    /// Returns a reference to the arguments passed to the function.
    pub fn args(&self) -> &[Box<dyn Any + Send + Sync>] {
        &self.context.args
    }
}

impl<'a> fmt::Debug for AsyncProceedingJoinPoint<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncProceedingJoinPoint")
            .field("context", &self.context)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joinpoint_qualified_name() {
        let jp = JoinPoint {
            function_name: "my_func",
            module_path: "crate::module",
            location: Location {
                file: "src/lib.rs",
                line: 10,
            },
            args: vec![],
        };

        assert_eq!(jp.qualified_name(), "crate::module::my_func");
    }

    #[test]
    fn test_async_joinpoint_qualified_name() {
        let jp = AsyncJoinPoint {
            function_name: "my_async_func",
            module_path: "crate::module",
            location: Location {
                file: "src/lib.rs",
                line: 20,
            },
            args: vec![],
        };

        assert_eq!(jp.qualified_name(), "crate::module::my_async_func");
    }
}
