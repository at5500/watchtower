//! Prelude module for convenient imports
//!
//! This module re-exports the most commonly used types from the watchtower-nats crate.
//!
//! # Examples
//!
//! ```
//! use watchtower_nats::prelude::*;
//!
//! // Now you have access to:
//! // - NatsConfig
//! // - NatsClient
//! // - NatsSubscriber
//! ```

pub use crate::client::NatsClient;
pub use crate::config::NatsConfig;
pub use crate::subscriber::NatsSubscriber;
