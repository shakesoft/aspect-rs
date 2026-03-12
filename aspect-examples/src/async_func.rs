use std::any::Any;
use std::future::Future;
use aspect_core::{Aspect, AspectError, AsyncAspect, AsyncJoinPoint, AsyncProceedingJoinPoint, JoinPoint, ProceedingJoinPoint};
use aspect_macros::aspect;
use aspect_std::{LoggingAspect, TimingAspect};

#[tokio::main]
async fn main() {
    println!("=== Timing Aspect Example ===\n");

    let result = add(5, 3).await;
    println!("Result of add: {}\n", result);
    let result = sub(10, 4).await;
    println!("Result of sub: {}\n", result);
}

#[aspect(Logger)]
async fn add(a: i32, b: i32) -> i32 {
    println!("  [APP] Adding {} + {}", a, b);
    a + b
}

#[aspect(Logger1)]
async fn sub(a: i32, b: i32) -> i32 {
    println!("  [APP] Subtracting {} - {}", a, b);
    a - b
}

#[derive(Default)]
pub struct Logger;

impl Aspect for Logger {
    fn before(&self, ctx: &JoinPoint) {
        let args = ctx
            .args
            .iter()
            .map(|arg| {
                if let Some(v) = arg.downcast_ref::<i32>() {
                    format!("{:?}", v)
                } else {
                    format!("{:?}", "<non-debug-arg>")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        println!(
            "before {}: {},{},{},[{}]",
            ctx.function_name, ctx.module_path, ctx.location.file, ctx.location.line, args
        );
    }

    fn after(&self, _ctx: &JoinPoint, _result: &dyn Any) {
        let args = _ctx
            .args
            .iter()
            .map(|arg| {
                if let Some(v) = arg.downcast_ref::<i32>() {
                    format!("{:?}", v)
                } else {
                    format!("{:?}", "<non-debug-arg>")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "alter {}: {},{},{},[{}]",
            _ctx.function_name, _ctx.module_path, _ctx.location.file, _ctx.location.line, args
        );
    }

    fn around(&self, pjp: ProceedingJoinPoint) -> Result<Box<dyn Any>, AspectError> {
        self.before(pjp.context());
        pjp.proceed()
    }
}


#[derive(Default)]
pub struct Logger1;

impl AsyncAspect for Logger1 {
    async fn before(&self, ctx: &AsyncJoinPoint) {
        let args = ctx
            .args
            .iter()
            .map(|arg| {
                if let Some(v) = arg.downcast_ref::<i32>() {
                    format!("{:?}", v)
                } else {
                    format!("{:?}", "<non-debug-arg>")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        println!(
            "before {}: {},{},{},[{}]",
            ctx.function_name, ctx.module_path, ctx.location.file, ctx.location.line, args
        );
    }

    async fn after(&self, _ctx: &AsyncJoinPoint, _result: &(dyn Any + Send + Sync)) {
        let args = _ctx
            .args
            .iter()
            .map(|arg| {
                if let Some(v) = arg.downcast_ref::<i32>() {
                    format!("{:?}", v)
                } else {
                    format!("{:?}", "<non-debug-arg>")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        println!(
            "after {}: {},{},{},[{}]",
            _ctx.function_name, _ctx.module_path, _ctx.location.file, _ctx.location.line, args
        );
    }

    async fn around(&self, pjp: AsyncProceedingJoinPoint<'_>) -> Result<Box<dyn Any + Send + Sync>, AspectError> {
        self.before(pjp.context()).await;
        Ok((pjp.proceed().await?))
    }

}
