//! Core types and traits for Watchtower notification system

pub mod backpressure;
pub mod config;
pub mod event;
pub mod subscriber;
pub mod transport;
pub mod errors;

pub use backpressure::{BackpressureController, BackpressureStats};
pub use config::{BackpressureConfig, BackpressureStrategy};
pub use event::{Event, EventMetadata};
pub use subscriber::{Subscriber, SubscriptionHandle};
pub use transport::{Transport, TransportInfo, TransportSubscription};
pub use errors::WatchtowerError;
