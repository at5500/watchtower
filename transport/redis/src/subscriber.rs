//! Redis subscriber implementation

use async_trait::async_trait;
use redis::{streams::StreamReadOptions, AsyncCommands};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::config::RedisConfig;
use watchtower_core::subscriber::EventCallback;
use watchtower_core::{Event, Subscriber, SubscriptionHandle, Transport, WatchtowerError};

use crate::transport::RedisTransport;

/// Subscription metadata
struct SubscriptionMeta {
    task_handle: tokio::task::JoinHandle<()>,
    event_types: Vec<String>,
}

/// Redis subscriber that manages subscriptions
pub struct RedisSubscriber {
    transport: Arc<RedisTransport>,
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionMeta>>>,
}

impl RedisSubscriber {
    /// Create a new Redis subscriber
    pub async fn new(config: RedisConfig) -> Result<Self, WatchtowerError> {
        let transport = Arc::new(RedisTransport::new(config).await?);

        Ok(Self {
            transport,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get backpressure statistics
    pub async fn backpressure_stats(&self) -> watchtower_core::BackpressureStats {
        self.transport.backpressure_stats().await
    }
}

#[async_trait]
impl Subscriber for RedisSubscriber {
    async fn subscribe(
        &mut self,
        event_types: Vec<String>,
        callback: EventCallback,
    ) -> Result<SubscriptionHandle, WatchtowerError> {
        let id = uuid::Uuid::new_v4().to_string();
        let handle = SubscriptionHandle::new(id.clone(), event_types.clone());

        // Subscribe to each event type in Redis (creates consumer groups)
        for event_type in &event_types {
            self.transport.subscribe(event_type).await?;
        }

        // Spawn task to consume messages from Redis Streams
        let transport = self.transport.clone();
        let sub_id = id.clone();
        let event_types_clone = event_types.clone();

        let task_handle = tokio::spawn(async move {
            // Get connection from transport for reading
            let mut conn = transport.connection.clone();

            // Build stream keys
            let stream_keys: Vec<String> = event_types_clone
                .iter()
                .map(|et| format!("{}:{}", transport.config.stream_prefix, et))
                .collect();

            info!(
                subscription_id = %sub_id,
                streams = ?stream_keys,
                "Starting Redis consumer task"
            );

            loop {
                // Read from consumer group using XREADGROUP
                let opts = StreamReadOptions::default()
                    .group(&transport.config.consumer_group, &transport.config.consumer_name)
                    .count(10) // Read up to 10 messages at a time
                    .block(1000); // Block for 1 second

                let streams: Vec<&str> = stream_keys.iter().map(|s| s.as_str()).collect();
                let ids: Vec<&str> = vec![">"; streams.len()]; // ">" means undelivered messages

                let result: Result<redis::streams::StreamReadReply, redis::RedisError> = conn.xread_options(&streams, &ids, &opts).await;
                match result {
                    Ok(stream_reply) => {
                        for stream_key in stream_reply.keys {
                            for stream_id in stream_key.ids {
                                // Extract event data from stream entry
                                if let Some(event_data) = stream_id.map.get("event") {
                                    match event_data {
                                        redis::Value::BulkString(bytes) => {
                                            match serde_json::from_slice::<Event>(bytes.as_slice()) {
                                                Ok(event) => {
                                                    if let Err(e) = callback(event.clone()).await {
                                                        error!(
                                                            subscription_id = %sub_id,
                                                            event_id = %event.id(),
                                                            error = %e,
                                                            "Callback execution failed"
                                                        );
                                                    }

                                                    // Acknowledge message
                                                    let _: Result<(), redis::RedisError> = conn
                                                        .xack(&stream_key.key, &transport.config.consumer_group, &[&stream_id.id])
                                                        .await;
                                                }
                                                Err(e) => {
                                                    error!(
                                                        subscription_id = %sub_id,
                                                        error = %e,
                                                        "Failed to deserialize event from Redis stream"
                                                    );
                                                }
                                            }
                                        }
                                        redis::Value::VerbatimString { format: _, text } => {
                                            match serde_json::from_slice::<Event>(text.as_bytes()) {
                                                Ok(event) => {
                                                    if let Err(e) = callback(event.clone()).await {
                                                        error!(
                                                            subscription_id = %sub_id,
                                                            event_id = %event.id(),
                                                            error = %e,
                                                            "Callback execution failed"
                                                        );
                                                    }

                                                    // Acknowledge message
                                                    let _: Result<(), redis::RedisError> = conn
                                                        .xack(&stream_key.key, &transport.config.consumer_group, &[&stream_id.id])
                                                        .await;
                                                }
                                                Err(e) => {
                                                    error!(
                                                        subscription_id = %sub_id,
                                                        error = %e,
                                                        "Failed to deserialize event from Redis stream"
                                                    );
                                                }
                                            }
                                        }
                                        _ => {
                                            error!("Unexpected Redis value type for event data");
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            subscription_id = %sub_id,
                            error = %e,
                            "Failed to read from Redis stream"
                        );
                        // Wait a bit before retrying
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
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
            "Created Redis subscription"
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
                "Unsubscribed from Redis and stopped consumer task"
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
