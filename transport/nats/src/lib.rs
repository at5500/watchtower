//! NATS notification module

pub mod client;
pub mod config;
pub mod subscriber;

pub use client::NatsClient;
pub use config::NatsConfig;
pub use subscriber::NatsSubscriber;
