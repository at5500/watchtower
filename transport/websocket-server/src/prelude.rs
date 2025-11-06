//! Prelude module for convenient imports

pub use crate::config::WebSocketServerConfig;
pub use crate::connection::{ClientId, ClientMetadata, ConnectionManager};
pub use crate::handlers::{websocket_handler, websocket_handler_with_metadata};
pub use crate::transport::WebSocketServerTransport;

// Re-export watchtower_core for convenience
pub use watchtower_core;
pub use watchtower_core::prelude::*;