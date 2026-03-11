//! Metrics collection aspect (counters, gauges, histograms).

use aspect_core::{Aspect, AspectError, ProceedingJoinPoint};
use parking_lot::Mutex;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Metrics aspect for collecting function call statistics.
///
/// # Example
///
/// ```rust,ignore
/// use aspect_std::MetricsAspect;
/// use aspect_macros::aspect;
///
/// let metrics = MetricsAspect::new();
///
/// #[aspect(metrics.clone())]
/// fn api_handler() -> Result<(), String> {
///     Ok(())
/// }
///
/// // Print metrics
/// metrics.print();
/// ```
#[derive(Clone)]
pub struct MetricsAspect {
    counters: Arc<Mutex<HashMap<String, u64>>>,
    histograms: Arc<Mutex<HashMap<String, Vec<Duration>>>>,
}

impl MetricsAspect {
    /// Create a new metrics aspect.
    pub fn new() -> Self {
        Self {
            counters: Arc::new(Mutex::new(HashMap::new())),
            histograms: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get call count for a function.
    pub fn get_count(&self, function_name: &str) -> u64 {
        self.counters
            .lock()
            .get(function_name)
            .copied()
            .unwrap_or(0)
    }

    /// Get duration histogram for a function.
    pub fn get_histogram(&self, function_name: &str) -> Vec<Duration> {
        self.histograms
            .lock()
            .get(function_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Print all metrics.
    pub fn print(&self) {
        println!("\n=== Metrics ===");

        let counters = self.counters.lock();
        println!("\nCall Counts:");
        for (name, count) in counters.iter() {
            println!("  {}: {}", name, count);
        }

        drop(counters);

        let histograms = self.histograms.lock();
        println!("\nDuration Histograms:");
        for (name, durations) in histograms.iter() {
            if !durations.is_empty() {
                let total: Duration = durations.iter().sum();
                let avg = total / durations.len() as u32;
                println!("  {}: avg={:?}, count={}", name, avg, durations.len());
            }
        }
        println!();
    }

    /// Clear all metrics.
    pub fn clear(&self) {
        self.counters.lock().clear();
        self.histograms.lock().clear();
    }
}

impl Default for MetricsAspect {
    fn default() -> Self {
        Self::new()
    }
}

impl Aspect for MetricsAspect {
    fn around(&self, pjp: ProceedingJoinPoint) -> Result<Box<dyn Any>, AspectError> {
        let function_name = pjp.context().function_name.to_string();
        let start = Instant::now();

        // Increment counter
        *self
            .counters
            .lock()
            .entry(function_name.clone())
            .or_insert(0) += 1;

        let result = pjp.proceed();

        // Record duration
        let duration = start.elapsed();
        self.histograms
            .lock()
            .entry(function_name)
            .or_default()
            .push(duration);

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_aspect() {
        let metrics = MetricsAspect::new();

        assert_eq!(metrics.get_count("test"), 0);
        assert!(metrics.get_histogram("test").is_empty());
    }

    #[test]
    fn test_metrics_increment() {
        let metrics = MetricsAspect::new();

        *metrics
            .counters
            .lock()
            .entry("test".to_string())
            .or_insert(0) += 1;
        *metrics
            .counters
            .lock()
            .entry("test".to_string())
            .or_insert(0) += 1;

        assert_eq!(metrics.get_count("test"), 2);
    }
}
