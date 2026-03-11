use aspect_core::{Aspect, AsyncAspect, AsyncJoinPoint, JoinPoint};
use aspect_macros::{aspect, async_aspect};
use aspect_std::{LoggingAspect, TimingAspect};

#[tokio::main]
async fn main() {
    println!("=== Timing Aspect Example ===\n");

    let result = add(5, 3).await;
    println!("Result of add: {}\n", result);
    let result = sub(10, 4).await;
    println!("Result of sub: {}\n", result);
}

#[async_aspect(Logger)]
async fn add(a: i32, b: i32) -> i32 {
    println!("  [APP] Adding {} + {}", a, b);
    a + b
}

#[async_aspect(Logger)]
async fn sub(a: i32, b: i32) -> i32 {
    println!("  [APP] Subtracting {} - {}", a, b);
    a - b
}

#[derive(Default)]
pub struct Logger;

impl AsyncAspect for Logger {
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
            "{}: {},{},{},[{}]",
            ctx.function_name, ctx.module_path, ctx.location.file, ctx.location.line, args
        );
    }
}
