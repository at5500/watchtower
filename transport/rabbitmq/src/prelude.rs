//! Prelude module for convenient imports
//!
//! This module re-exports the most commonly used types from the watchtower-rabbitmq crate.
//!
//! # Examples
//!
//! ```
//! use watchtower_rabbitmq::prelude::*;
//!
//! // Now you have access to:
//! // - RabbitMQConfig
//! // - RabbitMQTransport
//! // - RabbitMQSubscriber
//! // - ExchangeType
//! ```

pub use crate::config::{ExchangeType, RabbitMQConfig};
pub use crate::subscriber::RabbitMQSubscriber;
pub use crate::transport::RabbitMQTransport;
