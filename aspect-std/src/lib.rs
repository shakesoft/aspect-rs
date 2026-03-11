//! # aspect-std
//!
//! Standard aspects library providing common, production-ready aspects.
//!
//! This crate provides a collection of reusable aspects for common cross-cutting concerns:
//! - **Logging**: Structured logging with configurable levels
//! - **Timing**: Performance monitoring with statistics
//! - **Caching**: Generic memoization with TTL
//! - **Metrics**: Counters, gauges, and histograms
//! - **Rate Limiting**: Token bucket algorithm for throttling
//! - **Circuit Breaker**: Fault tolerance and failure prevention
//! - **Authorization**: Role-based access control
//! - **Validation**: Pre/post condition checking
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use aspect_std::prelude::*;
//! use aspect_macros::aspect;
//!
//! // Use the standard logging aspect
//! #[aspect(LoggingAspect::new())]
//! fn my_function(x: i32) -> i32 {
//!     x * 2
//! }
//! ```

pub mod authorization;
pub mod caching;
pub mod circuitbreaker;
pub mod logging;
pub mod metrics;
pub mod ratelimit;
pub mod timing;
pub mod validation;

// Re-export commonly used types
pub use authorization::{AuthMode, AuthorizationAspect};
pub use caching::CachingAspect;
pub use circuitbreaker::{CircuitBreakerAspect, CircuitState};
pub use logging::LoggingAspect;
pub use metrics::MetricsAspect;
pub use ratelimit::RateLimitAspect;
pub use timing::TimingAspect;
pub use validation::{ValidationAspect, ValidationRule};

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::authorization::{AuthMode, AuthorizationAspect};
    pub use crate::caching::CachingAspect;
    pub use crate::circuitbreaker::{CircuitBreakerAspect, CircuitState};
    pub use crate::logging::LoggingAspect;
    pub use crate::metrics::MetricsAspect;
    pub use crate::ratelimit::RateLimitAspect;
    pub use crate::timing::TimingAspect;
    pub use crate::validation::{ValidationAspect, ValidationRule};
}
