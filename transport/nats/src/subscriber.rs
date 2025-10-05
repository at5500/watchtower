//! NATS subscriber implementation

use async_trait::async_trait;
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use crate::config::NatsConfig;
use watchtower_core::subscriber::EventCallback;
use watchtower_core::{BackpressureController, BackpressureStrategy, Event, Subscriber, SubscriptionHandle, WatchtowerError};

use crate::client::NatsClient;

/// Subscription metadata
struct SubscriptionMeta {
    task_handle: tokio::task::JoinHandle<()>,
    event_types: Vec<String>,
}

/// NATS subscriber that manages subscriptions and event handling
pub struct NatsSubscriber {
    client: Arc<NatsClient>,
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionMeta>>>,
    backpressure: Arc<BackpressureController>,
}

impl NatsSubscriber {
    /// Create a new NATS subscriber
    pub async fn new(config: NatsConfig) -> Result<Self, WatchtowerError> {
        let backpressure_strategy = config.backpressure.strategy;

        let backpressure = Arc::new(BackpressureController::new(
            config.backpressure.max_queue_size,
            backpressure_strategy,
            config.backpressure.warning_threshold,
        ));

        let client = Arc::new(NatsClient::new(config).await?);

        Ok(Self {
            client,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            backpressure,
        })
    }

    /// Start listening to all subscriptions
    /// This should be called after creating subscriptions
    pub async fn start_listening(&self) {
        // Note: NATS subscriptions are already listening when created
        // This method is kept for API compatibility but doesn't need to do anything
        // The actual message handling happens in the subscribe() method
    }

    /// Get subject name for event type
    pub fn subject_for_event_type(event_type: &str) -> String {
        format!("watchtower.events.{}", event_type)
    }

    /// Get wildcard subject for all events
    pub fn subject_all_events() -> String {
        "watchtower.events.*".to_string()
    }

    /// Get backpressure statistics
    pub async fn backpressure_stats(&self) -> watchtower_core::BackpressureStats {
        self.backpressure.stats().await
    }
}

#[async_trait]
impl Subscriber for NatsSubscriber {
    async fn subscribe(
        &mut self,
        event_types: Vec<String>,
        callback: EventCallback,
    ) -> Result<SubscriptionHandle, WatchtowerError> {
        let id = uuid::Uuid::new_v4().to_string();

        // Determine subject to subscribe to
        let subject = if event_types.is_empty() {
            Self::subject_all_events()
        } else if event_types.len() == 1 {
            Self::subject_for_event_type(&event_types[0])
        } else {
            // For multiple event types, subscribe to wildcard and filter in callback
            Self::subject_all_events()
        };

        let mut subscription = self
            .client
            .client()
            .subscribe(subject.clone())
            .await
            .map_err(|e| {
                WatchtowerError::SubscriptionError(format!("Failed to subscribe: {}", e))
            })?;

        info!(
            subscription_id = %id,
            subject = %subject,
            event_types = ?event_types,
            "Created NATS subscription"
        );

        let handle = SubscriptionHandle::new(id.clone(), event_types.clone());

        // Spawn task to handle incoming messages
        let sub_id = id.clone();
        let event_types_filter = event_types.clone();
        let task_handle = tokio::spawn(async move {
            while let Some(message) = subscription.next().await {
                match serde_json::from_slice::<Event>(&message.payload) {
                    Ok(event) => {
                        // Check if event type matches subscription
                        if event_types_filter.is_empty() || event_types_filter.contains(&event.event_type().to_string()) {
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
                            "Failed to deserialize event from NATS message"
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

        Ok(handle)
    }

    async fn unsubscribe(&mut self, handle: &SubscriptionHandle) -> Result<(), WatchtowerError> {
        let mut subscriptions = self.subscriptions.write().await;

        if let Some(meta) = subscriptions.remove(&handle.id) {
            // Abort the consumer task
            meta.task_handle.abort();

            info!(
                subscription_id = %handle.id,
                "Unsubscribed from NATS and stopped consumer task"
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
        // Apply backpressure
        self.backpressure.send(event.clone()).await?;

        // Process event from queue
        if let Some(queued_event) = self.backpressure.receive().await {
            let subject = Self::subject_for_event_type(queued_event.event_type());
            self.client.publish(&subject, &queued_event).await?;
        }

        Ok(())
    }
}
