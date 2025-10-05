//! WebSocket transport for Watchtower notification system
//!
//! This module provides WebSocket integration for the Watchtower notification system,
//! supporting real-time bidirectional communication with features like:
//! - Automatic reconnection on disconnect
//! - Backpressure control
//! - Ping/pong keepalive
//! - Configurable message size limits

mod config;
mod subscriber;
mod transport;

pub use config::WebSocketConfig;
pub use subscriber::WebSocketSubscriber;
pub use transport::WebSocketTransport;
