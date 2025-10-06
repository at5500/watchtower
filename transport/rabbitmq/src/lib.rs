//! RabbitMQ transport for Watchtower notification system
//!
//! This module provides RabbitMQ integration for the Watchtower notification system,
//! supporting reliable message delivery with features like:
//! - Durable exchanges and queues
//! - Message persistence
//! - Dead letter exchanges for failed messages
//! - Message TTL
//! - Priority queues
//! - Publisher confirms
//! - Consumer acknowledgments

mod config;
mod subscriber;
mod transport;
pub mod prelude;

pub use config::{ExchangeType, RabbitMQConfig};
pub use subscriber::RabbitMQSubscriber;
pub use transport::RabbitMQTransport;
