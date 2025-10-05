//! Webhook notification module

pub mod client;
pub mod config;
pub mod subscriber;

pub use client::WebhookClient;
pub use config::WebhookConfig;
pub use subscriber::WebhookSubscriber;
