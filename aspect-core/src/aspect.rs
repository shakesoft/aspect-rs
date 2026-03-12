//! Aspect trait definition.
//!
//! The `Aspect` trait is the core abstraction for defining cross-cutting concerns
//! in aspect-oriented programming.

use crate::error::AspectError;
use crate::joinpoint::{AsyncJoinPoint, AsyncProceedingJoinPoint, JoinPoint, ProceedingJoinPoint};
use std::any::Any;
use std::future::Future;

/// The core trait for defining synchronous aspects.
pub trait Aspect: Send + Sync {
    /// Advice executed before the target function runs.
    fn before(&self, _ctx: &JoinPoint) {}

    /// Advice executed after the target function completes successfully.
    fn after(&self, _ctx: &JoinPoint, _result: &dyn Any) {}

    /// Advice executed when the target function encounters an error.
    fn after_error(&self, _ctx: &JoinPoint, _error: &AspectError) {}

    /// Advice that wraps the entire target function execution.
    fn around(&self, pjp: ProceedingJoinPoint) -> Result<Box<dyn Any>, AspectError> {
        let (ctx, proceed) = pjp.into_parts();

        self.before(&ctx);

        let result = proceed();

        match &result {
            Ok(value) => self.after(&ctx, value.as_ref()),
            Err(error) => self.after_error(&ctx, error),
        }

        result
    }
}

/// The core trait for defining asynchronous aspects.
pub trait AsyncAspect: Send + Sync {
    /// Async advice executed before the target function runs.
    fn before(&self, _ctx: &AsyncJoinPoint) -> impl Future<Output = ()> + Send {
        async {}
    }

    /// Async advice executed after the target function completes successfully.
    fn after(
        &self,
        _ctx: &AsyncJoinPoint,
        _result: &(dyn Any + Send + Sync),
    ) -> impl Future<Output = ()> + Send {
        async {}
    }

    /// Async advice executed when the target function encounters an error.
    fn after_error(
        &self,
        _ctx: &AsyncJoinPoint,
        _error: &AspectError,
    ) -> impl Future<Output = ()> + Send {
        async {}
    }

    /// Async advice that wraps the entire target function execution.
    fn around(
        &self,
        pjp: AsyncProceedingJoinPoint<'_>,
    ) -> impl Future<Output = Result<Box<dyn Any + Send + Sync>, AspectError>> + Send {
        async move {
            let (ctx, proceed) = pjp.into_parts();

            self.before(&ctx).await;

            let result = proceed().await;

            match &result {
                Ok(value) => self.after(&ctx, value.as_ref()).await,
                Err(error) => self.after_error(&ctx, error).await,
            }

            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::joinpoint::Location;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct TestAspect {
        called: Arc<Mutex<Vec<String>>>,
    }

    impl Default for TestAspect {
        fn default() -> Self {
            Self {
                called: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl Aspect for TestAspect {
        fn before(&self, ctx: &JoinPoint) {
            self.called
                .lock()
                .unwrap()
                .push(format!("before:{}", ctx.function_name));
        }

        fn after(&self, ctx: &JoinPoint, _result: &dyn Any) {
            self.called
                .lock()
                .unwrap()
                .push(format!("after:{}", ctx.function_name));
        }
    }

    #[test]
    fn test_aspect_trait() {
        let aspect = TestAspect::default();
        let ctx = JoinPoint {
            function_name: "test_function",
            module_path: "test::module",
            location: Location {
                file: "test.rs",
                line: 42,
            },
            args: vec![],
        };

        aspect.before(&ctx);
        aspect.after(&ctx, &42);

        let calls = aspect.called.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0], "before:test_function");
        assert_eq!(calls[1], "after:test_function");
    }
}
