//! Prelude module for convenient imports
//!
//! This module re-exports the most commonly used types from the watchtower-webhook crate.
//!
//! # Examples
//!
//! ```
//! use watchtower_webhook::prelude::*;
//!
//! // Now you have access to:
//! // - WebhookConfig
//! // - WebhookClient
//! // - WebhookSubscriber
//! ```

pub use crate::client::WebhookClient;
pub use crate::config::WebhookConfig;
pub use crate::subscriber::WebhookSubscriber;
