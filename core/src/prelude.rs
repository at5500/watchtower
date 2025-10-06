//! Prelude module for convenient imports
//!
//! This module re-exports the most commonly used types and traits from the watchtower-core crate.
//!
//! # Examples
//!
//! ```
//! use watchtower_core::prelude::*;
//!
//! // Now you have access to all commonly used types:
//! // - Event, EventMetadata
//! // - Transport, Subscriber
//! // - WatchtowerError
//! // - BackpressureConfig, BackpressureStrategy
//! // - CircuitBreakerConfig, CircuitState
//! // - SubscriptionHandle, TransportInfo
//! ```

pub use crate::backpressure::{BackpressureController, BackpressureStats};
pub use crate::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStats, CircuitState,
};
pub use crate::config::{BackpressureConfig, BackpressureStrategy};
pub use crate::errors::WatchtowerError;
pub use crate::event::{Event, EventMetadata};
pub use crate::subscriber::{EventCallback, Subscriber, SubscriptionHandle};
pub use crate::transport::{Transport, TransportInfo, TransportSubscription};
