//! WebSocket connection management

use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use axum::extract::ws::{Message, WebSocket};
use watchtower_core::{debug_log, Event};

/// Client identifier
pub type ClientId = Uuid;

/// Client metadata
#[derive(Debug, Clone)]
pub struct ClientMetadata {
    pub id: ClientId,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub attributes: HashMap<String, String>,
}

impl ClientMetadata {
    /// Create new client metadata
    pub fn new(id: ClientId) -> Self {
        Self {
            id,
            connected_at: chrono::Utc::now(),
            attributes: HashMap::new(),
        }
    }

    /// Add attribute
    pub fn with_attribute(mut self, key: String, value: String) -> Self {
        self.attributes.insert(key, value);
        self
    }

    /// Get attribute
    pub fn get_attribute(&self, key: &str) -> Option<&String> {
        self.attributes.get(key)
    }
}

/// Connection handle for managing individual client
pub struct ConnectionHandle {
    pub metadata: ClientMetadata,
    pub sender: mpsc::UnboundedSender<Message>,
}

/// Connection manager
pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<ClientId, ConnectionHandle>>>,
    broadcast_tx: broadcast::Sender<Event>,
    max_connections: usize,
}

impl ConnectionManager {
    /// Create new connection manager
    pub fn new(max_connections: usize, broadcast_buffer: usize) -> Self {
        let (broadcast_tx, _) = broadcast::channel(broadcast_buffer);

        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            max_connections,
        }
    }

    /// Get active connection count
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Check if can accept new connection
    pub async fn can_accept_connection(&self) -> bool {
        self.connection_count().await < self.max_connections
    }

    /// Register new connection
    pub async fn register(
        &self,
        client_id: ClientId,
        metadata: ClientMetadata,
        sender: mpsc::UnboundedSender<Message>,
    ) -> Result<(), String> {
        if !self.can_accept_connection().await {
            return Err(format!(
                "Maximum connections ({}) reached",
                self.max_connections
            ));
        }

        let mut connections = self.connections.write().await;
        connections.insert(
            client_id,
            ConnectionHandle {
                metadata,
                sender,
            },
        );

        info!(
            client_id = %client_id,
            total_connections = connections.len(),
            "Client registered"
        );

        Ok(())
    }

    /// Unregister connection
    pub async fn unregister(&self, client_id: &ClientId) {
        let mut connections = self.connections.write().await;
        if connections.remove(client_id).is_some() {
            info!(
                client_id = %client_id,
                total_connections = connections.len(),
                "Client unregistered"
            );
        }
    }

    /// Broadcast event to all connected clients
    pub async fn broadcast(&self, event: &Event) -> Result<usize, String> {
        // Send only the payload, not the full Event structure with metadata
        // This allows clients to receive the actual business data directly
        let json = serde_json::to_string(&event.payload)
            .map_err(|e| format!("Failed to serialize event payload: {}", e))?;

        let message = Message::Text(json.clone());

        let connections = self.connections.read().await;
        let mut sent_count = 0;

        for (client_id, handle) in connections.iter() {
            if handle.sender.send(message.clone()).is_ok() {
                sent_count += 1;
                debug_log!(
                    client_id = %client_id,
                    event_type = %event.event_type(),
                    "Event sent to client"
                );
            } else {
                warn!(
                    client_id = %client_id,
                    "Failed to send event to client (channel closed)"
                );
            }
        }

        info!(
            event_type = %event.event_type(),
            event_id = %event.id(),
            sent_to = sent_count,
            total_connections = connections.len(),
            "Event broadcast completed"
        );

        Ok(sent_count)
    }

    /// Send event to specific client
    pub async fn send_to_client(&self, client_id: &ClientId, event: &Event) -> Result<(), String> {
        // Send only the payload, not the full Event structure with metadata
        let json = serde_json::to_string(&event.payload)
            .map_err(|e| format!("Failed to serialize event payload: {}", e))?;

        let connections = self.connections.read().await;

        if let Some(handle) = connections.get(client_id) {
            handle
                .sender
                .send(Message::Text(json))
                .map_err(|e| format!("Failed to send to client: {}", e))?;

            debug_log!(
                client_id = %client_id,
                event_type = %event.event_type(),
                "Event sent to specific client"
            );

            Ok(())
        } else {
            Err(format!("Client {} not found", client_id))
        }
    }

    /// Get client metadata
    pub async fn get_client_metadata(&self, client_id: &ClientId) -> Option<ClientMetadata> {
        let connections = self.connections.read().await;
        connections.get(client_id).map(|h| h.metadata.clone())
    }

    /// Get all connected clients
    pub async fn get_all_clients(&self) -> Vec<ClientMetadata> {
        let connections = self.connections.read().await;
        connections
            .values()
            .map(|h| h.metadata.clone())
            .collect()
    }

    /// Subscribe to broadcast channel
    pub fn subscribe_broadcast(&self) -> broadcast::Receiver<Event> {
        self.broadcast_tx.subscribe()
    }

    /// Publish to broadcast channel
    pub fn publish_broadcast(&self, event: Event) -> Result<usize, String> {
        self.broadcast_tx
            .send(event)
            .map_err(|e| format!("Failed to broadcast: {}", e))
    }
}

/// Handle individual WebSocket connection
pub async fn handle_websocket_connection(
    socket: WebSocket,
    client_id: ClientId,
    metadata: ClientMetadata,
    manager: Arc<ConnectionManager>,
) {
    info!("=== HANDLE_WEBSOCKET_CONNECTION ENTRY POINT ===");
    info!(client_id = %client_id, "WebSocket connection established");

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create channel for this client
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Register connection
    if let Err(e) = manager.register(client_id, metadata.clone(), tx).await {
        error!(client_id = %client_id, error = %e, "Failed to register client");
        return;
    }

    // Spawn task to send messages to client
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Spawn task to receive messages from client
    let manager_clone = manager.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg_result) = ws_receiver.next().await {
            match msg_result {
                Ok(msg) => match msg {
                    Message::Text(text) => {
                        debug_log!(client_id = %client_id, "Received text message");
                        // TODO: Handle incoming messages (e.g., subscription filters)
                        #[cfg(feature = "debug-logging")]
                        if let Ok(_event) = serde_json::from_str::<Event>(&text) {
                            // Client sending event back? Echo or process as needed
                            debug_log!(client_id = %client_id, event_type = %_event.event_type(), "Received event from client");
                        }
                        #[cfg(not(feature = "debug-logging"))]
                        let _ = text; // Suppress unused variable warning
                    }
                    Message::Binary(_data) => {
                        #[cfg(feature = "debug-logging")]
                        debug_log!(client_id = %client_id, size = _data.len(), "Received binary message");
                    }
                    Message::Ping(_) => {
                        debug_log!(client_id = %client_id, "Received ping");
                    }
                    Message::Pong(_) => {
                        debug_log!(client_id = %client_id, "Received pong");
                    }
                    Message::Close(frame) => {
                        info!(client_id = %client_id, frame = ?frame, "Client initiated close");
                        break;
                    }
                },
                Err(e) => {
                    error!(client_id = %client_id, error = %e, "WebSocket error");
                    break;
                }
            }
        }

        manager_clone.unregister(&client_id).await;
    });

    // Wait for either task to complete
    tokio::select! {
        _ = &mut send_task => {
            debug_log!(client_id = %client_id, "Send task completed");
            recv_task.abort();
        }
        _ = &mut recv_task => {
            debug_log!(client_id = %client_id, "Receive task completed");
            send_task.abort();
        }
    }

    info!(client_id = %client_id, "WebSocket connection closed");
}