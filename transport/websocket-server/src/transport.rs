//! WebSocket server transport implementation

use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, warn};

use watchtower_core::{Event, Transport, TransportInfo, TransportSubscription, WatchtowerError};

use crate::config::WebSocketServerConfig;
use crate::connection::ConnectionManager;

/// WebSocket server transport
pub struct WebSocketServerTransport {
    config: WebSocketServerConfig,
    connection_manager: Arc<ConnectionManager>,
}

impl WebSocketServerTransport {
    /// Create new WebSocket server transport
    pub fn new(config: WebSocketServerConfig) -> Self {
        let connection_manager = Arc::new(ConnectionManager::new(
            config.max_connections,
            config.broadcast_buffer_size,
        ));

        Self {
            config,
            connection_manager,
        }
    }

    /// Get connection manager (for use in handlers)
    pub fn connection_manager(&self) -> Arc<ConnectionManager> {
        self.connection_manager.clone()
    }

    /// Get active connection count
    pub async fn active_connections(&self) -> usize {
        self.connection_manager.connection_count().await
    }

    /// Get configuration
    pub fn config(&self) -> &WebSocketServerConfig {
        &self.config
    }

    /// Handle a new WebSocket connection
    ///
    /// This is a convenience method for Axum integration.
    /// It handles the WebSocket connection lifecycle:
    /// - Registers the client with metadata
    /// - Forwards broadcast events to the client
    /// - Handles disconnection cleanup
    ///
    /// # Arguments
    /// * `socket` - The Axum WebSocket
    /// * `metadata` - Optional client metadata (can include user ID, etc.)
    pub async fn handle_connection(
        &self,
        socket: axum::extract::ws::WebSocket,
        metadata: Option<std::collections::HashMap<String, String>>,
    ) {
        use axum::extract::ws::Message;
        use futures_util::{SinkExt, StreamExt};
        use tokio::sync::mpsc;
        use tracing::{error, info};
        use uuid::Uuid;

        eprintln!("!!! TRANSPORT.HANDLE_CONNECTION CALLED !!!");
        info!("=== WATCHTOWER handle_connection ENTRY POINT ===");
        let client_id = Uuid::new_v4();
        info!("handle_connection called for new client {}", client_id);

        let connection_manager = self.connection_manager.clone();

        // Create metadata for this client
        let mut client_metadata = crate::connection::ClientMetadata::new(client_id);
        if let Some(attrs) = metadata {
            for (key, value) in attrs {
                info!("Adding metadata: {} = {}", key, value);
                client_metadata = client_metadata.with_attribute(key, value);
            }
        }

        info!("Creating channel for client {}", client_id);
        // Create channel for sending messages to this client
        let (tx, mut rx) = mpsc::unbounded_channel();

        info!("Registering client {}", client_id);
        // Register the connection
        if let Err(e) = connection_manager.register(client_id, client_metadata, tx).await {
            error!("Failed to register WebSocket client {}: {}", client_id, e);
            return;
        }

        info!(client_id = %client_id, "WebSocket client connected");

        // Split the socket into sender and receiver
        let (mut ws_sender, mut ws_receiver) = socket.split();

        // Task 1: Handle messages from client
        let client_id_clone = client_id;
        let connection_manager_clone = connection_manager.clone();
        let receive_task = tokio::spawn(async move {
            info!("Receive task started for client {}", client_id_clone);
            while let Some(result) = ws_receiver.next().await {
                match result {
                    Ok(msg) => {
                        info!("Received message from client {}: {:?}", client_id_clone, msg);
                        if matches!(msg, Message::Close(_)) {
                            info!("Close message received from client {}", client_id_clone);
                            break;
                        }
                        // Clients in broadcast mode don't send events, only receive
                        // Ping/Pong handled automatically by Axum
                    }
                    Err(e) => {
                        error!("WebSocket receive error for client {}: {}", client_id_clone, e);
                        break;
                    }
                }
            }
            info!("Receive task ending for client {}", client_id_clone);
            connection_manager_clone.unregister(&client_id_clone).await;
        });

        // Task 2: Forward events from broadcast channel to client
        let client_id_for_send = client_id;
        let send_task = tokio::spawn(async move {
            info!("Send task started for client {}", client_id_for_send);
            while let Some(message) = rx.recv().await {
                info!("Sending message to client {}: {:?}", client_id_for_send, message);
                if ws_sender.send(message).await.is_err() {
                    error!("Failed to send message to client {}", client_id_for_send);
                    break;
                }
            }
            info!("Send task ending for client {}", client_id_for_send);
        });

        // Wait for either task to complete
        tokio::select! {
            _ = receive_task => {},
            _ = send_task => {},
        }

        info!(client_id = %client_id, "WebSocket client disconnected");
    }
}

#[async_trait]
impl Transport for WebSocketServerTransport {
    fn info(&self) -> TransportInfo {
        TransportInfo {
            name: "websocket-server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            supports_subscriptions: true,
            supports_backpressure: false, // Handled by client-side buffering
        }
    }

    async fn publish(&self, event: Event) -> Result<(), WatchtowerError> {
        let sent_count = self
            .connection_manager
            .broadcast(&event)
            .await
            .map_err(|e| WatchtowerError::PublicationError(e))?;

        if sent_count == 0 {
            warn!(
                event_type = %event.event_type(),
                event_id = %event.id(),
                "Event published but no clients connected"
            );
        }

        Ok(())
    }

    async fn subscribe(&self, pattern: &str) -> Result<TransportSubscription, WatchtowerError> {
        // WebSocket server doesn't need traditional subscriptions
        // Clients automatically receive all published events
        info!(pattern = %pattern, "WebSocket server subscription (all clients receive events)");

        Ok(TransportSubscription::new(
            uuid::Uuid::new_v4().to_string(),
            pattern.to_string(),
            "websocket-server",
        ))
    }

    async fn health_check(&self) -> Result<(), WatchtowerError> {
        let count = self.connection_manager.connection_count().await;

        info!(
            active_connections = count,
            max_connections = self.config.max_connections,
            "WebSocket server health check"
        );

        Ok(())
    }

    async fn shutdown(&self) -> Result<(), WatchtowerError> {
        info!("Shutting down WebSocket server transport");

        // Connections will be closed when ConnectionManager is dropped
        // or when the HTTP server shuts down

        Ok(())
    }

    async fn publish_to_dlq(
        &self,
        event: Event,
        error: &WatchtowerError,
    ) -> Result<(), WatchtowerError> {
        // WebSocket server doesn't maintain a DLQ
        // Failed deliveries are handled by client reconnection
        warn!(
            event_id = %event.id(),
            event_type = %event.event_type(),
            error = %error,
            "Event failed to publish (no DLQ for WebSocket server)"
        );

        Ok(())
    }

    async fn consume_dlq(
        &self,
        _callback: watchtower_core::subscriber::EventCallback,
    ) -> Result<(), WatchtowerError> {
        // WebSocket server doesn't maintain a DLQ
        Ok(())
    }
}