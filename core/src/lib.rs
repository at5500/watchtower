//! Core types and traits for Watchtower notification system

/// Conditional debug logging macros
/// These macros only compile in code when the `debug-logging` feature is enabled
#[cfg(feature = "debug-logging")]
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        tracing::debug!($($arg)*)
    };
}

#[cfg(not(feature = "debug-logging"))]
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "debug-logging")]
#[macro_export]
macro_rules! trace_log {
    ($($arg:tt)*) => {
        tracing::trace!($($arg)*)
    };
}

#[cfg(not(feature = "debug-logging"))]
#[macro_export]
macro_rules! trace_log {
    ($($arg:tt)*) => {};
}

pub mod backpressure;
pub mod circuit_breaker;
pub mod config;
pub mod event;
pub mod handler;
pub mod subscriber;
pub mod transport;
pub mod errors;
pub mod prelude;

pub use backpressure::{BackpressureController, BackpressureStats};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStats, CircuitState};
pub use config::{BackpressureConfig, BackpressureStrategy};
pub use event::{Event, EventMetadata};
pub use handler::HandlerInfo;
pub use subscriber::{Subscriber, SubscriptionHandle};
pub use transport::{Transport, TransportInfo, TransportSubscription};
pub use errors::WatchtowerError;
