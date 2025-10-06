//! Webhook notification module

pub mod client;
pub mod config;
pub mod subscriber;
pub mod prelude;

pub use client::WebhookClient;
pub use config::WebhookConfig;
pub use subscriber::WebhookSubscriber;
