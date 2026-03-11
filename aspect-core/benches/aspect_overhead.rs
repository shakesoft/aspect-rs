//! Benchmarks for aspect overhead measurement
//!
//! Measures the performance impact of using aspects compared to
//! hand-written code.

use aspect_core::{Aspect, AspectError, JoinPoint, Location, ProceedingJoinPoint};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::any::Any;

// ============================================================================
// Test Aspects
// ============================================================================

/// No-op aspect that does nothing
struct NoOpAspect;

impl Aspect for NoOpAspect {
    // All methods use default implementation (empty)
}

/// Simple aspect with minimal before/after logic
struct SimpleAspect {
    counter: std::sync::atomic::AtomicUsize,
}

impl SimpleAspect {
    fn new() -> Self {
        Self {
            counter: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl Aspect for SimpleAspect {
    fn before(&self, _ctx: &JoinPoint) {
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn after(&self, _ctx: &JoinPoint, _result: &dyn Any) {
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Complex aspect with work in around advice
struct ComplexAspect;

impl Aspect for ComplexAspect {
    fn around(&self, pjp: ProceedingJoinPoint) -> Result<Box<dyn Any>, AspectError> {
        // Simulate some work
        let _sum: u64 = (0..10).sum();

        let result = pjp.proceed()?;

        // Simulate more work
        let _product: u64 = (1..5).product();

        Ok(result)
    }
}

// ============================================================================
// Test Functions
// ============================================================================

/// Baseline: function with no aspect
#[inline(never)]
fn baseline_function(x: i32) -> i32 {
    x * 2
}

/// Function with no-op aspect (simulated by calling aspect methods)
#[inline(never)]
fn noop_aspect_function(x: i32) -> i32 {
    let aspect = NoOpAspect;
    let ctx = JoinPoint {
        function_name: "noop_aspect_function",
        module_path: "benchmark",
        location: Location {
            file: "benches/aspect_overhead.rs",
            line: 0,
        },
        args: vec![],
    };

    aspect.before(&ctx);
    let result = baseline_function(x);
    aspect.after(&ctx, &result);
    result
}

/// Function with simple aspect
#[inline(never)]
fn simple_aspect_function(x: i32) -> i32 {
    let aspect = SimpleAspect::new();
    let ctx = JoinPoint {
        function_name: "simple_aspect_function",
        module_path: "benchmark",
        location: Location {
            file: "benches/aspect_overhead.rs",
            line: 0,
        },
        args: vec![],
    };

    aspect.before(&ctx);
    let result = baseline_function(x);
    aspect.after(&ctx, &result);
    result
}

/// Function with complex aspect using around advice
#[inline(never)]
fn complex_aspect_function(x: i32) -> Result<i32, AspectError> {
    let aspect = ComplexAspect;
    let ctx = JoinPoint {
        function_name: "complex_aspect_function",
        module_path: "benchmark",
        location: Location {
            file: "benches/aspect_overhead.rs",
            line: 0,
        },
        args: vec![],
    };

    let pjp = ProceedingJoinPoint::new(|| Ok(Box::new(baseline_function(x)) as Box<dyn Any>), ctx);

    let result = aspect.around(pjp)?;
    Ok(*result.downcast::<i32>().unwrap())
}

// ============================================================================
// Benchmarks
// ============================================================================

fn bench_baseline(c: &mut Criterion) {
    c.bench_function("baseline_no_aspect", |b| {
        b.iter(|| baseline_function(black_box(42)))
    });
}

fn bench_noop_aspect(c: &mut Criterion) {
    c.bench_function("noop_aspect", |b| {
        b.iter(|| noop_aspect_function(black_box(42)))
    });
}

fn bench_simple_aspect(c: &mut Criterion) {
    c.bench_function("simple_aspect", |b| {
        b.iter(|| simple_aspect_function(black_box(42)))
    });
}

fn bench_complex_aspect(c: &mut Criterion) {
    c.bench_function("complex_aspect", |b| {
        b.iter(|| complex_aspect_function(black_box(42)).unwrap())
    });
}

fn bench_aspect_overhead_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("aspect_overhead");

    group.bench_function("baseline", |b| b.iter(|| baseline_function(black_box(42))));

    group.bench_function("noop", |b| b.iter(|| noop_aspect_function(black_box(42))));

    group.bench_function("simple", |b| {
        b.iter(|| simple_aspect_function(black_box(42)))
    });

    group.bench_function("complex", |b| {
        b.iter(|| complex_aspect_function(black_box(42)).unwrap())
    });

    group.finish();
}

fn bench_joinpoint_creation(c: &mut Criterion) {
    c.bench_function("joinpoint_creation", |b| {
        b.iter(|| {
            black_box(JoinPoint {
                function_name: "test",
                module_path: "test::module",
                location: Location {
                    file: "test.rs",
                    line: 42,
                },
                args: vec![],
            })
        })
    });
}

fn bench_proceedingjoinpoint(c: &mut Criterion) {
    c.bench_function("proceedingjoinpoint_proceed", |b| {
        b.iter(|| {
            let ctx = JoinPoint {
                function_name: "test",
                module_path: "test::module",
                location: Location {
                    file: "test.rs",
                    line: 42,
                },
                args: vec![],
            };
            let pjp = ProceedingJoinPoint::new(|| Ok(Box::new(42) as Box<dyn Any>), ctx);
            black_box(pjp.proceed().unwrap())
        })
    });
}

criterion_group!(
    benches,
    bench_baseline,
    bench_noop_aspect,
    bench_simple_aspect,
    bench_complex_aspect,
    bench_aspect_overhead_comparison,
    bench_joinpoint_creation,
    bench_proceedingjoinpoint
);

criterion_main!(benches);
