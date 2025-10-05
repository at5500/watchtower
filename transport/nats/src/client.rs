//! NATS client for event publishing

use async_nats::Client;
use futures_util::StreamExt;
use tracing::{error, info};
use crate::config::NatsConfig;
use watchtower_core::{Event, WatchtowerError};

/// NATS client for publishing events
pub struct NatsClient {
    client: Client,
    config: NatsConfig,
}

impl NatsClient {
    /// Create a new NATS client
    pub async fn new(config: NatsConfig) -> Result<Self, WatchtowerError> {
        let client = async_nats::connect(&config.url)
            .await
            .map_err(|e| WatchtowerError::NatsError(format!("Connection failed: {}", e)))?;

        info!(url = %config.url, "Connected to NATS server");

        Ok(Self { client, config })
    }

    /// Publish event to a NATS subject
    pub async fn publish(
        &self,
        subject: &str,
        event: &Event,
    ) -> Result<(), WatchtowerError> {
        let payload = serde_json::to_vec(event)
            .map_err(WatchtowerError::SerializationError)?;

        self.client
            .publish(subject.to_string(), payload.into())
            .await
            .map_err(|e| {
                error!(
                    subject = %subject,
                    event_id = %event.id(),
                    error = %e,
                    "Failed to publish event to NATS"
                );
                WatchtowerError::NatsError(format!("Publish failed: {}", e))
            })?;

        info!(
            subject = %subject,
            event_id = %event.id(),
            event_type = %event.event_type(),
            "Event published to NATS"
        );

        Ok(())
    }

    /// Publish event with retry logic
    pub async fn publish_with_retry(
        &self,
        subject: &str,
        event: &Event,
        max_attempts: u32,
    ) -> Result<(), WatchtowerError> {
        for attempt in 1..=max_attempts {
            match self.publish(subject, event).await {
                Ok(()) => return Ok(()),
                Err(e) if attempt == max_attempts => return Err(e),
                Err(e) => {
                    error!(
                        subject = %subject,
                        event_id = %event.id(),
                        attempt = attempt,
                        error = %e,
                        "Retry publishing to NATS"
                    );

                    let delay = tokio::time::Duration::from_secs(
                        self.config.reconnect_delay_seconds * attempt as u64,
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(WatchtowerError::NatsError(
            "Max retry attempts reached".to_string(),
        ))
    }

    /// Get the underlying NATS client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Publish event to Dead Letter Queue
    pub async fn publish_to_dlq(
        &self,
        event: &Event,
        error: &WatchtowerError,
    ) -> Result<(), WatchtowerError> {
        let dlq_subject = format!("dlq.{}", event.event_type());

        // Create DLQ message with error information
        let dlq_payload = serde_json::json!({
            "event": event,
            "error": error.to_string(),
            "original_subject": event.event_type(),
        });

        let payload = serde_json::to_vec(&dlq_payload)
            .map_err(WatchtowerError::SerializationError)?;

        self.client
            .publish(dlq_subject.clone(), payload.into())
            .await
            .map_err(|e| {
                error!(
                    subject = %dlq_subject,
                    event_id = %event.id(),
                    error = %e,
                    "Failed to publish event to DLQ"
                );
                WatchtowerError::NatsError(format!("DLQ publish failed: {}", e))
            })?;

        info!(
            subject = %dlq_subject,
            event_id = %event.id(),
            event_type = %event.event_type(),
            "Event published to Dead Letter Queue"
        );

        Ok(())
    }

    /// Subscribe to Dead Letter Queue
    pub async fn subscribe_dlq(
        &self,
        callback: watchtower_core::subscriber::EventCallback,
    ) -> Result<(), WatchtowerError> {
        let dlq_subject = "dlq.*";

        let mut subscriber = self.client
            .subscribe(dlq_subject.to_string())
            .await
            .map_err(|e| WatchtowerError::NatsError(format!("DLQ subscription failed: {}", e)))?;

        info!(subject = %dlq_subject, "Started consuming from Dead Letter Queue");

        tokio::spawn(async move {
            while let Some(message) = subscriber.next().await {
                match serde_json::from_slice::<serde_json::Value>(&message.payload) {
                    Ok(dlq_message) => {
                        if let Some(event_value) = dlq_message.get("event") {
                            match serde_json::from_value::<Event>(event_value.clone()) {
                                Ok(event) => {
                                    if let Err(e) = callback(event.clone()).await {
                                        error!(
                                            event_id = %event.id(),
                                            error = %e,
                                            "DLQ callback execution failed"
                                        );
                                    }
                                }
                                Err(e) => {
                                    error!(error = %e, "Failed to deserialize event from DLQ");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to deserialize DLQ message");
                    }
                }
            }
        });

        Ok(())
    }
}
