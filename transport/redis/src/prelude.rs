//! Prelude module for convenient imports
//!
//! This module re-exports the most commonly used types from the watchtower-redis crate.
//!
//! # Examples
//!
//! ```
//! use watchtower_redis::prelude::*;
//!
//! // Now you have access to:
//! // - RedisConfig
//! // - RedisTransport
//! // - RedisSubscriber
//! ```

pub use crate::config::RedisConfig;
pub use crate::subscriber::RedisSubscriber;
pub use crate::transport::RedisTransport;
