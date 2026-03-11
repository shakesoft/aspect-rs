use aspect_core::{Aspect, JoinPoint};
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

#[aspect(Logger)]
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
                if let Some(v) = arg.downcast_ref::<String>() {
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
