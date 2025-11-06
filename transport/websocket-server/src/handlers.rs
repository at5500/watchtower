//! Axum WebSocket handlers

use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::IntoResponse,
};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::connection::{handle_websocket_connection, ClientMetadata};
use crate::transport::WebSocketServerTransport;

/// WebSocket upgrade handler
///
/// This handler upgrades HTTP connections to WebSocket and manages the connection lifecycle.
///
/// # Example
///
/// ```rust,no_run
/// use axum::{routing::get, Router};
/// use watchtower_websocket_server::prelude::*;
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() {
///     let config = WebSocketServerConfig::default();
///     let transport = Arc::new(WebSocketServerTransport::new(config));
///
///     let app = Router::new()
///         .route("/ws", get(websocket_handler))
///         .with_state(transport);
///
///     // Run server...
/// }
/// ```
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(transport): State<Arc<WebSocketServerTransport>>,
) -> impl IntoResponse {
    let client_id = Uuid::new_v4();
    let manager = transport.connection_manager();

    info!(
        client_id = %client_id,
        "WebSocket upgrade request"
    );

    // Check connection limit - note: we accept upgrade and close gracefully
    // Axum doesn't provide a way to reject upgrade before handshake
    let can_accept = manager.can_accept_connection().await;
    if !can_accept {
        warn!(
            client_id = %client_id,
            max_connections = transport.config().max_connections,
            "Connection rejected: max connections reached"
        );
    }

    let metadata = ClientMetadata::new(client_id);

    ws.on_upgrade(move |socket| {
        handle_websocket_connection(socket, client_id, metadata, manager)
    })
}

/// WebSocket upgrade handler with custom client metadata
///
/// This variant allows injecting custom metadata (e.g., from authentication middleware).
///
/// # Example
///
/// ```rust,no_run
/// use axum::{routing::get, Router, Extension};
/// use watchtower_websocket_server::prelude::*;
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() {
///     let config = WebSocketServerConfig::default();
///     let transport = Arc::new(WebSocketServerTransport::new(config));
///
///     let app = Router::new()
///         .route("/ws", get(websocket_handler_with_metadata))
///         .with_state(transport);
///
///     // Add auth middleware that injects Extension<ClientMetadata>
/// }
/// ```
pub async fn websocket_handler_with_metadata(
    ws: WebSocketUpgrade,
    State(transport): State<Arc<WebSocketServerTransport>>,
    axum::Extension(metadata): axum::Extension<ClientMetadata>,
) -> impl IntoResponse {
    let client_id = metadata.id;
    let manager = transport.connection_manager();

    info!(
        client_id = %client_id,
        attributes = ?metadata.attributes,
        "WebSocket upgrade request with metadata"
    );

    // Check connection limit - note: we accept upgrade and close gracefully
    // Axum doesn't provide a way to reject upgrade before handshake
    let can_accept = manager.can_accept_connection().await;
    if !can_accept {
        warn!(
            client_id = %client_id,
            max_connections = transport.config().max_connections,
            "Connection rejected: max connections reached"
        );
    }

    ws.on_upgrade(move |socket| {
        handle_websocket_connection(socket, client_id, metadata, manager)
    })
}