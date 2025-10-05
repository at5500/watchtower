//! RabbitMQ subscriber implementation

use async_trait::async_trait;
use futures_util::stream::StreamExt;
use lapin::options::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::config::RabbitMQConfig;
use crate::transport::RabbitMQTransport;
use watchtower_core::subscriber::EventCallback;
use watchtower_core::{Event, Subscriber, SubscriptionHandle, Transport, WatchtowerError};

/// Subscription metadata
struct SubscriptionMeta {
    task_handle: tokio::task::JoinHandle<()>,
    event_types: Vec<String>,
}

/// RabbitMQ subscriber that manages subscriptions
pub struct RabbitMQSubscriber {
    transport: Arc<RabbitMQTransport>,
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionMeta>>>,
}

impl RabbitMQSubscriber {
    /// Create a new RabbitMQ subscriber
    pub async fn new(config: RabbitMQConfig) -> Result<Self, WatchtowerError> {
        let transport = Arc::new(RabbitMQTransport::new(config).await?);

        Ok(Self {
            transport,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Start listening for messages (should be called after subscriptions are set up)
    /// Note: RabbitMQ consumers start automatically when subscribe() is called
    pub async fn start_listening(&self) {
        // RabbitMQ consumers are already listening when created
        // This method is kept for API compatibility
        info!("RabbitMQ consumers are already active");
    }

    /// Get backpressure statistics
    pub async fn backpressure_stats(&self) -> watchtower_core::BackpressureStats {
        self.transport.backpressure_stats().await
    }
}

#[async_trait]
impl Subscriber for RabbitMQSubscriber {
    async fn subscribe(
        &mut self,
        event_types: Vec<String>,
        callback: EventCallback,
    ) -> Result<SubscriptionHandle, WatchtowerError> {
        let id = uuid::Uuid::new_v4().to_string();
        let handle = SubscriptionHandle::new(id.clone(), event_types.clone());

        // Subscribe to each event type in RabbitMQ (creates queues and bindings)
        for event_type in &event_types {
            self.transport.subscribe(event_type).await?;
        }

        // Spawn consumer task for each event type
        let transport = self.transport.clone();
        let sub_id = id.clone();
        let event_types_clone = event_types.clone();

        let task_handle = tokio::spawn(async move {
            // Create a new channel for consuming
            let channel = match transport.connection.create_channel().await {
                Ok(ch) => ch,
                Err(e) => {
                    error!(
                        subscription_id = %sub_id,
                        error = %e,
                        "Failed to create RabbitMQ channel for consumer"
                    );
                    return;
                }
            };

            // Set QoS
            if let Err(e) = channel
                .basic_qos(transport.config.prefetch_count, BasicQosOptions::default())
                .await
            {
                error!(
                    subscription_id = %sub_id,
                    error = %e,
                    "Failed to set QoS"
                );
                return;
            }

            for event_type in &event_types_clone {
                let queue_name = format!("{}.{}", transport.config.queue_prefix, event_type);
                let callback_clone = callback.clone();
                let sub_id_clone = sub_id.clone();
                let channel_clone = channel.clone();

                // Create consumer for this queue
                match channel_clone
                    .basic_consume(
                        &queue_name,
                        &format!("consumer-{}", sub_id_clone),
                        BasicConsumeOptions::default(),
                        lapin::types::FieldTable::default(),
                    )
                    .await
                {
                    Ok(mut consumer) => {
                        let queue_name_clone = queue_name.clone();
                        tokio::spawn(async move {
                            info!(
                                subscription_id = %sub_id_clone,
                                queue = %queue_name_clone,
                                "Started RabbitMQ consumer"
                            );

                            while let Some(delivery_result) = consumer.next().await {
                                match delivery_result {
                                    Ok(delivery) => {
                                        match serde_json::from_slice::<Event>(&delivery.data) {
                                            Ok(event) => {
                                                if let Err(e) = callback_clone(event.clone()).await {
                                                    error!(
                                                        subscription_id = %sub_id_clone,
                                                        event_id = %event.id(),
                                                        error = %e,
                                                        "Callback execution failed"
                                                    );
                                                    // Negative acknowledge on callback error
                                                    let _ = delivery
                                                        .nack(BasicNackOptions {
                                                            requeue: true,
                                                            ..Default::default()
                                                        })
                                                        .await;
                                                } else {
                                                    // Acknowledge successful processing
                                                    let _ = delivery.ack(BasicAckOptions::default()).await;
                                                }
                                            }
                                            Err(e) => {
                                                error!(
                                                    subscription_id = %sub_id_clone,
                                                    error = %e,
                                                    "Failed to deserialize event from RabbitMQ message"
                                                );
                                                // Negative acknowledge on deserialization error
                                                let _ = delivery
                                                    .nack(BasicNackOptions {
                                                        requeue: false,
                                                        ..Default::default()
                                                    })
                                                    .await;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!(
                                            subscription_id = %sub_id_clone,
                                            error = %e,
                                            "RabbitMQ consumer error"
                                        );
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!(
                            subscription_id = %sub_id,
                            queue = %queue_name,
                            error = %e,
                            "Failed to create RabbitMQ consumer"
                        );
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
            "Created RabbitMQ subscription"
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
                "Unsubscribed from RabbitMQ and stopped consumer task"
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
