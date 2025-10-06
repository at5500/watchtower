//! WebSocket subscriber implementation

use async_trait::async_trait;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info};

use crate::config::WebSocketConfig;
use crate::transport::WebSocketTransport;
use watchtower_core::subscriber::EventCallback;
use watchtower_core::{Event, Subscriber, SubscriptionHandle, Transport, WatchtowerError};

/// Subscription metadata
struct SubscriptionMeta {
    task_handle: tokio::task::JoinHandle<()>,
    #[allow(dead_code)] // Stored for future use (logging, metrics, debugging)
    event_types: Vec<String>,
}

/// WebSocket subscriber that manages subscriptions
pub struct WebSocketSubscriber {
    transport: Arc<WebSocketTransport>,
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionMeta>>>,
}

impl WebSocketSubscriber {
    /// Create a new WebSocket subscriber
    pub async fn new(config: WebSocketConfig) -> Result<Self, WatchtowerError> {
        let transport = Arc::new(WebSocketTransport::new(config).await?);

        Ok(Self {
            transport,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Start listening for incoming WebSocket messages
    /// Note: WebSocket consumers start automatically when subscribe() is called
    pub async fn start_listening(&self) {
        // WebSocket consumers are already listening when created
        // This method is kept for API compatibility
        info!("WebSocket consumers are already active");
    }

    /// Get backpressure statistics
    pub async fn backpressure_stats(&self) -> watchtower_core::BackpressureStats {
        self.transport.backpressure_stats().await
    }
}

#[async_trait]
impl Subscriber for WebSocketSubscriber {
    async fn subscribe(
        &mut self,
        event_types: Vec<String>,
        callback: EventCallback,
    ) -> Result<SubscriptionHandle, WatchtowerError> {
        let id = uuid::Uuid::new_v4().to_string();
        let handle = SubscriptionHandle::new(id.clone(), event_types.clone());

        // Subscribe to each event type via WebSocket
        for event_type in &event_types {
            self.transport.subscribe(event_type).await?;
        }

        // Spawn task to consume messages from WebSocket
        let transport = self.transport.clone();
        let sub_id = id.clone();
        let event_types_clone = event_types.clone();

        let task_handle = tokio::spawn(async move {
            info!(
                subscription_id = %sub_id,
                event_types = ?event_types_clone,
                "Starting WebSocket consumer task"
            );

            loop {
                // Get reader from transport
                let mut reader_guard = transport.reader.lock().await;

                if let Some(reader) = reader_guard.as_mut() {
                    // Read next message
                    match reader.next().await {
                        Some(Ok(msg)) => {
                            drop(reader_guard);

                            match msg {
                                Message::Text(text) => {
                                    match serde_json::from_str::<Event>(&text) {
                                        Ok(event) => {
                                            // Check if event type matches subscription
                                            if event_types_clone.is_empty()
                                                || event_types_clone.contains(&event.event_type().to_string())
                                            {
                                                if let Err(e) = callback(event.clone()).await {
                                                    error!(
                                                        subscription_id = %sub_id,
                                                        event_id = %event.id(),
                                                        error = %e,
                                                        "Callback execution failed"
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!(
                                                subscription_id = %sub_id,
                                                error = %e,
                                                "Failed to deserialize event from WebSocket message"
                                            );
                                        }
                                    }
                                }
                                Message::Binary(data) => {
                                    match serde_json::from_slice::<Event>(&data) {
                                        Ok(event) => {
                                            // Check if event type matches subscription
                                            if event_types_clone.is_empty()
                                                || event_types_clone.contains(&event.event_type().to_string())
                                            {
                                                if let Err(e) = callback(event.clone()).await {
                                                    error!(
                                                        subscription_id = %sub_id,
                                                        event_id = %event.id(),
                                                        error = %e,
                                                        "Callback execution failed"
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!(
                                                subscription_id = %sub_id,
                                                error = %e,
                                                "Failed to deserialize event from WebSocket binary message"
                                            );
                                        }
                                    }
                                }
                                Message::Close(_) => {
                                    info!(
                                        subscription_id = %sub_id,
                                        "WebSocket connection closed"
                                    );
                                    break;
                                }
                                Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {
                                    // Ignore control frames
                                }
                            }
                        }
                        Some(Err(e)) => {
                            drop(reader_guard);
                            error!(
                                subscription_id = %sub_id,
                                error = %e,
                                "WebSocket read error"
                            );
                            // Wait a bit before retrying
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        }
                        None => {
                            drop(reader_guard);
                            error!(
                                subscription_id = %sub_id,
                                "WebSocket stream ended"
                            );
                            break;
                        }
                    }
                } else {
                    drop(reader_guard);
                    error!(
                        subscription_id = %sub_id,
                        "WebSocket reader not available"
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        });

        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.insert(
            id.clone(),
            SubscriptionMeta {
                task_handle,
                event_types: event_types.clone(),
            },
        );

        info!(
            subscription_id = %id,
            event_types = ?event_types,
            "Created WebSocket subscription"
        );

        Ok(handle)
    }

    async fn unsubscribe(&mut self, handle: &SubscriptionHandle) -> Result<(), WatchtowerError> {
        let mut subscriptions = self.subscriptions.write().await;

        if let Some(meta) = subscriptions.remove(&handle.id) {
            // Abort the consumer task
            meta.task_handle.abort();

            info!(
                subscription_id = %handle.id,
                "Unsubscribed from WebSocket and stopped consumer task"
            );

            Ok(())
        } else {
            Err(WatchtowerError::SubscriptionError(format!(
                "Subscription {} not found",
                handle.id
            )))
        }
    }

    async fn publish(&self, event: Event) -> Result<(), WatchtowerError> {
        self.transport.publish(event).await
    }
}
