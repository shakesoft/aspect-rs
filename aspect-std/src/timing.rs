//! Performance monitoring aspect with statistics.

use aspect_core::{Aspect, AspectError, ProceedingJoinPoint};
use parking_lot::Mutex;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Timing aspect that measures function execution time and collects statistics.
///
/// # Example
///
/// ```rust,ignore
/// use aspect_std::TimingAspect;
/// use aspect_macros::aspect;
///
/// let timing = TimingAspect::new();
///
/// #[aspect(timing.clone())]
/// fn expensive_operation() -> Result<(), String> {
///     std::thread::sleep(std::time::Duration::from_millis(100));
///     Ok(())
/// }
///
/// // Later, print statistics
/// timing.print_stats();
/// ```
#[derive(Clone)]
pub struct TimingAspect {
    stats: Arc<Mutex<HashMap<String, FunctionStats>>>,
    threshold_ms: Option<u64>,
    print_on_complete: bool,
}

/// Statistics for a single function.
#[derive(Debug, Clone)]
pub struct FunctionStats {
    /// Function name
    pub name: String,
    /// Number of calls
    pub count: u64,
    /// Total execution time
    pub total_duration: Duration,
    /// Minimum execution time
    pub min_duration: Duration,
    /// Maximum execution time
    pub max_duration: Duration,
}

impl FunctionStats {
    fn new(name: String) -> Self {
        Self {
            name,
            count: 0,
            total_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
        }
    }

    fn record(&mut self, duration: Duration) {
        self.count += 1;
        self.total_duration += duration;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);
    }

    /// Get average execution time.
    pub fn average_duration(&self) -> Duration {
        if self.count > 0 {
            self.total_duration / self.count as u32
        } else {
            Duration::ZERO
        }
    }
}

impl TimingAspect {
    /// Create a new timing aspect.
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(HashMap::new())),
            threshold_ms: None,
            print_on_complete: false,
        }
    }

    /// Set a threshold in milliseconds. Only log functions exceeding this duration.
    pub fn with_threshold(mut self, threshold_ms: u64) -> Self {
        self.threshold_ms = Some(threshold_ms);
        self
    }

    /// Print timing after each function completes.
    pub fn print_on_complete(mut self) -> Self {
        self.print_on_complete = true;
        self
    }

    /// Get statistics for a specific function.
    pub fn get_stats(&self, function_name: &str) -> Option<FunctionStats> {
        self.stats.lock().get(function_name).cloned()
    }

    /// Get all function statistics.
    pub fn all_stats(&self) -> Vec<FunctionStats> {
        self.stats.lock().values().cloned().collect()
    }

    /// Print statistics for all functions.
    pub fn print_stats(&self) {
        let stats = self.stats.lock();
        if stats.is_empty() {
            println!("No timing data collected.");
            return;
        }

        println!("\n=== Timing Statistics ===");
        println!(
            "{:<30} {:>10} {:>15} {:>15} {:>15} {:>15}",
            "Function", "Calls", "Total", "Average", "Min", "Max"
        );
        println!("{:-<100}", "");

        for stat in stats.values() {
            println!(
                "{:<30} {:>10} {:>15.3?} {:>15.3?} {:>15.3?} {:>15.3?}",
                stat.name,
                stat.count,
                stat.total_duration,
                stat.average_duration(),
                stat.min_duration,
                stat.max_duration
            );
        }
        println!();
    }

    /// Clear all statistics.
    pub fn clear(&self) {
        self.stats.lock().clear();
    }

    fn record_timing(&self, function_name: &str, duration: Duration) {
        let mut stats = self.stats.lock();
        stats
            .entry(function_name.to_string())
            .or_insert_with(|| FunctionStats::new(function_name.to_string()))
            .record(duration);
    }
}

impl Default for TimingAspect {
    fn default() -> Self {
        Self::new()
    }
}

impl Aspect for TimingAspect {
    fn around(&self, pjp: ProceedingJoinPoint) -> Result<Box<dyn Any>, AspectError> {
        let function_name = pjp.context().function_name.to_string();
        let start = Instant::now();

        let result = pjp.proceed();

        let duration = start.elapsed();
        self.record_timing(&function_name, duration);

        // Check threshold
        if let Some(threshold_ms) = self.threshold_ms {
            if duration.as_millis() > threshold_ms as u128 {
                println!(
                    "[SLOW] {} took {:?} (threshold: {}ms)",
                    function_name, duration, threshold_ms
                );
            }
        }

        // Print if requested
        if self.print_on_complete {
            println!("[TIMING] {} took {:?}", function_name, duration);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspect_core::JoinPoint;

    #[test]
    fn test_timing_aspect_creation() {
        let aspect = TimingAspect::new();
        assert!(aspect.threshold_ms.is_none());
        assert!(!aspect.print_on_complete);
        assert!(aspect.all_stats().is_empty());
    }

    #[test]
    fn test_timing_aspect_builder() {
        let aspect = TimingAspect::new().with_threshold(100).print_on_complete();

        assert_eq!(aspect.threshold_ms, Some(100));
        assert!(aspect.print_on_complete);
    }

    #[test]
    fn test_function_stats() {
        let mut stats = FunctionStats::new("test_func".to_string());

        stats.record(Duration::from_millis(10));
        stats.record(Duration::from_millis(20));
        stats.record(Duration::from_millis(30));

        assert_eq!(stats.count, 3);
        assert_eq!(stats.min_duration, Duration::from_millis(10));
        assert_eq!(stats.max_duration, Duration::from_millis(30));
        assert_eq!(stats.average_duration(), Duration::from_millis(20));
    }

    #[test]
    fn test_timing_aspect_record() {
        let aspect = TimingAspect::new();

        aspect.record_timing("func1", Duration::from_millis(10));
        aspect.record_timing("func1", Duration::from_millis(20));
        aspect.record_timing("func2", Duration::from_millis(30));

        let stats1 = aspect.get_stats("func1").unwrap();
        assert_eq!(stats1.count, 2);

        let stats2 = aspect.get_stats("func2").unwrap();
        assert_eq!(stats2.count, 1);

        assert_eq!(aspect.all_stats().len(), 2);
    }
}
