//! Prelude module for convenient imports
//!
//! This module re-exports the most commonly used types from the watchtower-websocket crate.
//!
//! # Examples
//!
//! ```
//! use watchtower_websocket::prelude::*;
//!
//! // Now you have access to:
//! // - WebSocketConfig
//! // - WebSocketTransport
//! // - WebSocketSubscriber
//! ```

pub use crate::config::WebSocketConfig;
pub use crate::subscriber::WebSocketSubscriber;
pub use crate::transport::WebSocketTransport;
