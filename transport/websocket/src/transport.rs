//! WebSocket transport implementation

use async_trait::async_trait;
use futures_util::{stream::SplitStream, SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{error, info, warn};

use crate::config::WebSocketConfig;
use watchtower_core::{
    BackpressureController, Event, Transport, TransportInfo, TransportSubscription,
    WatchtowerError,
};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsWriter = futures_util::stream::SplitSink<WsStream, Message>;
type WsReader = SplitStream<WsStream>;

/// WebSocket transport
pub struct WebSocketTransport {
    pub(crate) config: WebSocketConfig,
    writer: Arc<Mutex<Option<WsWriter>>>,
    pub(crate) reader: Arc<Mutex<Option<WsReader>>>,
    backpressure: BackpressureController,
}

impl WebSocketTransport {
    /// Create a new WebSocket transport
    pub async fn new(config: WebSocketConfig) -> Result<Self, WatchtowerError> {
        let backpressure = BackpressureController::new(
            config.backpressure.max_queue_size,
            config.backpressure.strategy,
            config.backpressure.warning_threshold,
        );

        let mut transport = Self {
            config,
            writer: Arc::new(Mutex::new(None)),
            reader: Arc::new(Mutex::new(None)),
            backpressure,
        };

        transport.connect().await?;

        Ok(transport)
    }

    /// Connect to WebSocket server
    async fn connect(&mut self) -> Result<(), WatchtowerError> {
        let mut retry_count = 0;

        loop {
            match connect_async(&self.config.url).await {
                Ok((ws_stream, _)) => {
                    info!(url = %self.config.url, "Connected to WebSocket server");

                    // Split the stream into read and write halves
                    let (write, read) = ws_stream.split();

                    *self.writer.lock().await = Some(write);
                    *self.reader.lock().await = Some(read);

                    return Ok(());
                }
                Err(e) => {
                    retry_count += 1;
                    if self.config.retry_attempts > 0 && retry_count >= self.config.retry_attempts {
                        return Err(WatchtowerError::ConnectionError(format!(
                            "WebSocket connection failed after {} attempts: {}",
                            retry_count, e
                        )));
                    }
                    warn!(
                        attempt = retry_count,
                        max_attempts = self.config.retry_attempts,
                        error = %e,
                        "WebSocket connection failed, retrying..."
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(
                        self.config.retry_delay_seconds,
                    ))
                    .await;
                }
            }
        }
    }

    /// Reconnect to WebSocket server
    async fn reconnect(&self) -> Result<(), WatchtowerError> {
        if !self.config.auto_reconnect {
            return Err(WatchtowerError::ConnectionError(
                "Auto-reconnect is disabled".to_string(),
            ));
        }

        info!("Attempting to reconnect to WebSocket server");

        let mut retry_count = 0;
        loop {
            match connect_async(&self.config.url).await {
                Ok((ws_stream, _)) => {
                    info!(url = %self.config.url, "Reconnected to WebSocket server");
                    let (write, read) = ws_stream.split();
                    *self.writer.lock().await = Some(write);
                    *self.reader.lock().await = Some(read);
                    return Ok(());
                }
                Err(e) => {
                    retry_count += 1;
                    if self.config.retry_attempts > 0 && retry_count >= self.config.retry_attempts {
                        return Err(WatchtowerError::ConnectionError(format!(
                            "WebSocket reconnection failed after {} attempts: {}",
                            retry_count, e
                        )));
                    }
                    warn!(
                        attempt = retry_count,
                        error = %e,
                        "WebSocket reconnection failed, retrying..."
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(
                        self.config.retry_delay_seconds,
                    ))
                    .await;
                }
            }
        }
    }

    /// Get backpressure statistics
    pub async fn backpressure_stats(&self) -> watchtower_core::BackpressureStats {
        self.backpressure.stats().await
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    fn info(&self) -> TransportInfo {
        TransportInfo {
            name: "websocket".to_string(),
            version: "0.1.0".to_string(),
            supports_subscriptions: true,
            supports_backpressure: true,
        }
    }

    async fn publish(&self, event: Event) -> Result<(), WatchtowerError> {
        // Apply backpressure
        self.backpressure.send(event.clone()).await?;

        // Process event from queue
        if let Some(queued_event) = self.backpressure.receive().await {
            let payload = serde_json::to_string(&queued_event)
                .map_err(WatchtowerError::SerializationError)?;

            let mut writer_guard = self.writer.lock().await;

            if let Some(writer) = writer_guard.as_mut() {
                if let Err(e) = writer.send(Message::Text(payload.clone().into())).await {
                    error!(
                        event_id = %queued_event.id(),
                        error = %e,
                        "Failed to send message via WebSocket"
                    );

                    // Drop the lock before reconnecting
                    drop(writer_guard);

                    // Try to reconnect
                    if let Err(reconnect_err) = self.reconnect().await {
                        return Err(WatchtowerError::PublicationError(format!(
                            "WebSocket send failed and reconnection failed: {}",
                            reconnect_err
                        )));
                    }

                    // Retry send after reconnection
                    let mut writer_guard = self.writer.lock().await;
                    if let Some(writer) = writer_guard.as_mut() {
                        writer.send(Message::Text(payload.into())).await.map_err(|e| {
                            WatchtowerError::PublicationError(format!(
                                "WebSocket send failed after reconnection: {}",
                                e
                            ))
                        })?;
                    }
                }

                info!(
                    event_id = %queued_event.id(),
                    event_type = %queued_event.event_type(),
                    "Event published via WebSocket"
                );
            } else {
                return Err(WatchtowerError::PublicationError(
                    "WebSocket connection is not established".to_string(),
                ));
            }
        }

        Ok(())
    }

    async fn subscribe(&self, pattern: &str) -> Result<TransportSubscription, WatchtowerError> {
        // WebSocket subscription is typically handled by sending a subscribe message
        // and then receiving messages that match the pattern
        Ok(TransportSubscription::new(
            uuid::Uuid::new_v4().to_string(),
            pattern.to_string(),
            "websocket",
        ))
    }

    async fn health_check(&self) -> Result<(), WatchtowerError> {
        let writer = self.writer.lock().await;
        if writer.is_some() {
            Ok(())
        } else {
            Err(WatchtowerError::ConnectionError(
                "WebSocket connection is not established".to_string(),
            ))
        }
    }

    async fn shutdown(&self) -> Result<(), WatchtowerError> {
        info!("Shutting down WebSocket transport");

        let mut writer = self.writer.lock().await;
        if let Some(mut ws_writer) = writer.take() {
            if let Err(e) = ws_writer.close().await {
                warn!(error = %e, "Failed to close WebSocket connection gracefully");
            }
        }

        Ok(())
    }
}
