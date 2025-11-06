//! WebSocket Server Transport for Watchtower
//!
//! This transport provides a WebSocket server implementation that allows multiple clients
//! to connect and receive real-time event notifications via WebSocket connections.
//!
//! # Features
//!
//! - **Broadcast Events**: Publish events to all connected WebSocket clients
//! - **Connection Management**: Track and manage multiple concurrent connections
//! - **Axum Integration**: Ready-to-use handlers for Axum web framework
//! - **Client Metadata**: Associate custom metadata with each connection
//! - **Configurable Limits**: Control max connections, message size, etc.
//!
//! # Example
//!
//! ```rust,no_run
//! use watchtower_websocket_server::prelude::*;
//! use watchtower_core::{Event, Transport};
//! use axum::{routing::get, Router};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create transport
//!     let config = WebSocketServerConfig::default()
//!         .with_max_connections(1000)
//!         .with_broadcast_buffer(500);
//!
//!     let transport = Arc::new(WebSocketServerTransport::new(config));
//!
//!     // Create Axum router
//!     let app = Router::new()
//!         .route("/ws", get(websocket_handler))
//!         .with_state(transport.clone());
//!
//!     // Start server
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
//!     axum::serve(listener, app).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Publishing Events
//!
//! ```rust,no_run
//! use watchtower_websocket_server::prelude::*;
//! use watchtower_core::{Event, EventMetadata, Transport};
//! use std::sync::Arc;
//! use serde_json::json;
//!
//! async fn publish_example(transport: Arc<WebSocketServerTransport>) {
//!     let event = Event::new(
//!         "user.created",
//!         json!({"user_id": 123, "name": "Alice"}),
//!         EventMetadata::default(),
//!     );
//!
//!     // Broadcast to all connected clients
//!     transport.publish(event).await.unwrap();
//! }
//! ```

pub mod config;
pub mod connection;
pub mod handlers;
pub mod prelude;
pub mod transport;

// Re-export watchtower-core for users
pub use watchtower_core;

pub use config::WebSocketServerConfig;
pub use connection::{ClientId, ClientMetadata, ConnectionManager};
pub use handlers::{websocket_handler, websocket_handler_with_metadata};
pub use transport::WebSocketServerTransport;