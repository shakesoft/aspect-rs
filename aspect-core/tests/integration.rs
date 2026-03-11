//! Integration tests for aspect-core.

use aspect_core::prelude::*;
use std::any::Any;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct TestAspect {
    log: Arc<Mutex<Vec<String>>>,
}

impl TestAspect {
    fn new() -> Self {
        Self {
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_log(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

impl Aspect for TestAspect {
    fn before(&self, ctx: &JoinPoint) {
        self.log
            .lock()
            .unwrap()
            .push(format!("before:{}", ctx.function_name));
    }

    fn after(&self, ctx: &JoinPoint, _result: &dyn Any) {
        self.log
            .lock()
            .unwrap()
            .push(format!("after:{}", ctx.function_name));
    }

    fn after_error(&self, ctx: &JoinPoint, _error: &AspectError) {
        self.log
            .lock()
            .unwrap()
            .push(format!("error:{}", ctx.function_name));
    }
}

#[test]
fn test_aspect_lifecycle() {
    let aspect = TestAspect::new();
    let ctx = JoinPoint {
        function_name: "test_fn",
        module_path: "test",
        location: Location {
            file: "test.rs",
            line: 1,
        },
        args: vec![],
    };

    aspect.before(&ctx);
    aspect.after(&ctx, &42);

    let log = aspect.get_log();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0], "before:test_fn");
    assert_eq!(log[1], "after:test_fn");
}

#[test]
fn test_aspect_error_handling() {
    let aspect = TestAspect::new();
    let ctx = JoinPoint {
        function_name: "failing_fn",
        module_path: "test",
        location: Location {
            file: "test.rs",
            line: 10,
        },
        args: vec![],
    };

    aspect.before(&ctx);
    aspect.after_error(&ctx, &AspectError::execution("test error"));

    let log = aspect.get_log();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0], "before:failing_fn");
    assert_eq!(log[1], "error:failing_fn");
}

#[test]
fn test_proceeding_joinpoint() {
    let ctx = JoinPoint {
        function_name: "wrapped_fn",
        module_path: "test",
        location: Location {
            file: "test.rs",
            line: 20,
        },
        args: vec![],
    };

    let executed = Arc::new(Mutex::new(false));
    let executed_clone = Arc::clone(&executed);

    let pjp = ProceedingJoinPoint::new(
        move || {
            *executed_clone.lock().unwrap() = true;
            Ok(Box::new(42) as Box<dyn Any>)
        },
        ctx,
    );

    assert!(!*executed.lock().unwrap());
    let result = pjp.proceed();
    assert!(*executed.lock().unwrap());
    assert!(result.is_ok());
}
