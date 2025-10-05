//! Redis Streams transport implementation

use async_trait::async_trait;
use redis::streams::StreamMaxlen;
use redis::{aio::MultiplexedConnection, AsyncCommands, Client};
use tracing::{error, info};
use crate::config::RedisConfig;
use watchtower_core::{
    BackpressureController, Event, Transport, TransportInfo,
    TransportSubscription, WatchtowerError,
};

/// Redis Streams transport
pub struct RedisTransport {
    pub(crate) connection: MultiplexedConnection,
    pub(crate) config: RedisConfig,
    backpressure: BackpressureController,
}

impl RedisTransport {
    /// Create a new Redis transport
    pub async fn new(config: RedisConfig) -> Result<Self, WatchtowerError> {
        let client = Client::open(config.url.as_str())
            .map_err(|e| WatchtowerError::ConnectionError(format!("Redis connection failed: {}", e)))?;

        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| WatchtowerError::ConnectionError(format!("Redis connection failed: {}", e)))?;

        let backpressure_strategy = config.backpressure.strategy;

        let backpressure = BackpressureController::new(
            config.backpressure.max_queue_size,
            backpressure_strategy,
            config.backpressure.warning_threshold,
        );

        info!(url = %config.url, "Connected to Redis");

        Ok(Self {
            connection,
            config,
            backpressure,
        })
    }

    /// Get stream key for event type
    fn stream_key(&self, event_type: &str) -> String {
        format!("{}:{}", self.config.stream_prefix, event_type)
    }

    /// Get backpressure statistics
    pub async fn backpressure_stats(&self) -> watchtower_core::BackpressureStats {
        self.backpressure.stats().await
    }
}

#[async_trait]
impl Transport for RedisTransport {
    fn info(&self) -> TransportInfo {
        TransportInfo {
            name: "redis".to_string(),
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
            let stream_key = self.stream_key(queued_event.event_type());

            let payload = serde_json::to_string(&queued_event)
                .map_err(WatchtowerError::SerializationError)?;

            let mut conn = self.connection.clone();

            let items = &[("event", payload.as_str())];

            // Add to stream with optional max length
            let result: Result<String, redis::RedisError> = if self.config.max_stream_length > 0 {
                conn.xadd_maxlen(
                    &stream_key,
                    StreamMaxlen::Approx(self.config.max_stream_length),
                    "*",
                    items,
                )
                .await
            } else {
                conn.xadd(&stream_key, "*", items).await
            };

            result.map_err(|e| {
                error!(
                    stream = %stream_key,
                    event_id = %queued_event.id(),
                    error = %e,
                    "Failed to publish to Redis stream"
                );
                WatchtowerError::PublicationError(format!("Redis XADD failed: {}", e))
            })?;

            info!(
                stream = %stream_key,
                event_id = %queued_event.id(),
                event_type = %queued_event.event_type(),
                "Event published to Redis stream"
            );
        }

        Ok(())
    }

    async fn subscribe(&self, pattern: &str) -> Result<TransportSubscription, WatchtowerError> {
        let stream_key = if pattern.contains(':') {
            pattern.to_string()
        } else {
            self.stream_key(pattern)
        };

        // Create consumer group if it doesn't exist
        let mut conn = self.connection.clone();
        let _: Result<(), redis::RedisError> = conn
            .xgroup_create_mkstream(
                &stream_key,
                &self.config.consumer_group,
                "0",
            )
            .await;

        Ok(TransportSubscription::new(
            uuid::Uuid::new_v4().to_string(),
            stream_key,
            "redis",
        ))
    }

    async fn health_check(&self) -> Result<(), WatchtowerError> {
        let mut conn = self.connection.clone();
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .map_err(|e| WatchtowerError::ConnectionError(format!("Redis ping failed: {}", e)))?;
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), WatchtowerError> {
        info!("Shutting down Redis transport");
        Ok(())
    }
}
