//! Redis Streams transport implementation

use async_trait::async_trait;
use redis::streams::StreamMaxlen;
use redis::{aio::MultiplexedConnection, AsyncCommands, Client};
use std::sync::Arc;
use tracing::{error, info, warn};
use crate::config::RedisConfig;
use watchtower_core::{
    BackpressureController, CircuitBreaker, CircuitBreakerConfig, Event, Transport, TransportInfo,
    TransportSubscription, WatchtowerError,
};

/// Redis Streams transport
pub struct RedisTransport {
    pub(crate) connection: MultiplexedConnection,
    pub(crate) config: RedisConfig,
    backpressure: BackpressureController,
    circuit_breaker: Arc<CircuitBreaker>,
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

        let circuit_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));

        Ok(Self {
            connection,
            config,
            backpressure,
            circuit_breaker,
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

    /// Get circuit breaker statistics
    pub async fn circuit_breaker_stats(&self) -> watchtower_core::CircuitBreakerStats {
        self.circuit_breaker.stats().await
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
        // Check circuit breaker
        if !self.circuit_breaker.should_allow_request().await {
            let stats = self.circuit_breaker.stats().await;
            warn!(
                stream_prefix = %self.config.stream_prefix,
                state = ?stats.state,
                "Circuit breaker is open, rejecting publish request"
            );
            return Err(WatchtowerError::PublicationError(
                "Circuit breaker is open".to_string(),
            ));
        }

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

            match result {
                Ok(_) => {
                    self.circuit_breaker.record_success().await;
                    info!(
                        stream = %stream_key,
                        event_id = %queued_event.id(),
                        event_type = %queued_event.event_type(),
                        "Event published to Redis stream"
                    );
                }
                Err(e) => {
                    self.circuit_breaker.record_failure().await;
                    error!(
                        stream = %stream_key,
                        event_id = %queued_event.id(),
                        error = %e,
                        "Failed to publish to Redis stream"
                    );
                    return Err(WatchtowerError::PublicationError(format!("Redis XADD failed: {}", e)));
                }
            }
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

    async fn publish_to_dlq(&self, event: Event, error: &WatchtowerError) -> Result<(), WatchtowerError> {
        let dlq_stream = format!("{}:dlq", self.config.stream_prefix);

        let payload = serde_json::to_string(&event)
            .map_err(WatchtowerError::SerializationError)?;

        let mut conn = self.connection.clone();

        let items = &[
            ("event", payload.as_str()),
            ("error", &error.to_string()),
            ("original_type", event.event_type()),
        ];

        // Add to DLQ stream with max length
        let result: Result<String, redis::RedisError> = if self.config.max_stream_length > 0 {
            conn.xadd_maxlen(
                &dlq_stream,
                StreamMaxlen::Approx(self.config.max_stream_length),
                "*",
                items,
            )
            .await
        } else {
            conn.xadd(&dlq_stream, "*", items).await
        };

        result.map_err(|e| {
            error!(
                stream = %dlq_stream,
                event_id = %event.id(),
                error = %e,
                "Failed to publish to DLQ stream"
            );
            WatchtowerError::PublicationError(format!("Redis DLQ XADD failed: {}", e))
        })?;

        info!(
            stream = %dlq_stream,
            event_id = %event.id(),
            event_type = %event.event_type(),
            "Event published to Dead Letter Queue"
        );

        Ok(())
    }

    async fn consume_dlq(&self, callback: watchtower_core::subscriber::EventCallback) -> Result<(), WatchtowerError> {
        use redis::streams::StreamReadOptions;

        let dlq_stream = format!("{}:dlq", self.config.stream_prefix);
        let dlq_group = format!("{}_dlq", self.config.consumer_group);

        // Create DLQ consumer group
        let mut conn = self.connection.clone();
        let _: Result<(), redis::RedisError> = conn
            .xgroup_create_mkstream(&dlq_stream, &dlq_group, "0")
            .await;

        info!(
            stream = %dlq_stream,
            group = %dlq_group,
            "Started consuming from Dead Letter Queue"
        );

        tokio::spawn(async move {
            loop {
                let opts = StreamReadOptions::default()
                    .group(&dlq_group, format!("{}_dlq_consumer", &dlq_group))
                    .count(10)
                    .block(1000);

                let result: Result<redis::streams::StreamReadReply, redis::RedisError> =
                    conn.xread_options(&[&dlq_stream], &[">"], &opts).await;

                match result {
                    Ok(stream_reply) => {
                        for stream_key in stream_reply.keys {
                            for stream_id in stream_key.ids {
                                if let Some(event_data) = stream_id.map.get("event") {
                                    let event_result = match event_data {
                                        redis::Value::BulkString(bytes) => {
                                            serde_json::from_slice::<Event>(bytes.as_slice())
                                        }
                                        redis::Value::VerbatimString { format: _, text } => {
                                            serde_json::from_slice::<Event>(text.as_bytes())
                                        }
                                        _ => {
                                            error!("Unexpected Redis value type for DLQ event data");
                                            continue;
                                        }
                                    };

                                    match event_result {
                                        Ok(event) => {
                                            if let Err(e) = callback(event.clone()).await {
                                                error!(
                                                    event_id = %event.id(),
                                                    error = %e,
                                                    "DLQ callback execution failed"
                                                );
                                            }

                                            // Acknowledge DLQ message
                                            let _: Result<(), redis::RedisError> = conn
                                                .xack(&stream_key.key, &dlq_group, &[&stream_id.id])
                                                .await;
                                        }
                                        Err(e) => {
                                            error!(
                                                error = %e,
                                                "Failed to deserialize event from DLQ stream"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to read from DLQ stream");
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
        });

        Ok(())
    }
}
