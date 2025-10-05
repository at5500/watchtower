//! Webhook subscriber implementation

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::config::WebhookConfig;
use watchtower_core::{Event, Subscriber, SubscriptionHandle, WatchtowerError};
use watchtower_core::subscriber::EventCallback;
use watchtower_core::{BackpressureConfig, BackpressureController, BackpressureStrategy};

use crate::client::WebhookClient;

/// Webhook endpoint configuration
#[derive(Debug, Clone)]
pub struct WebhookEndpoint {
    pub url: String,
    pub secret: Option<String>,
    pub event_types: Vec<String>,
}

/// Webhook subscriber that manages webhook endpoints and event delivery
pub struct WebhookSubscriber {
    client: Arc<WebhookClient>,
    endpoints: Arc<RwLock<HashMap<String, WebhookEndpoint>>>,
    callbacks: Arc<RwLock<HashMap<String, EventCallback>>>,
    backpressure: Arc<BackpressureController>,
}

impl WebhookSubscriber {
    pub fn new(config: WebhookConfig) -> Result<Self, WatchtowerError> {
        let backpressure_strategy = config.backpressure.strategy;

        let backpressure = Arc::new(BackpressureController::new(
            config.backpressure.max_queue_size,
            backpressure_strategy,
            config.backpressure.warning_threshold,
        ));

        let client = Arc::new(WebhookClient::new(config)?);

        Ok(Self {
            client,
            endpoints: Arc::new(RwLock::new(HashMap::new())),
            callbacks: Arc::new(RwLock::new(HashMap::new())),
            backpressure,
        })
    }

    /// Register a webhook endpoint
    pub async fn register_endpoint(
        &mut self,
        id: impl Into<String>,
        endpoint: WebhookEndpoint,
    ) -> Result<(), WatchtowerError> {
        let mut endpoints = self.endpoints.write().await;
        endpoints.insert(id.into(), endpoint);
        Ok(())
    }

    /// Unregister a webhook endpoint
    pub async fn unregister_endpoint(&mut self, id: &str) -> Result<(), WatchtowerError> {
        let mut endpoints = self.endpoints.write().await;
        endpoints.remove(id);
        Ok(())
    }

    /// Get all registered endpoints
    pub async fn get_endpoints(&self) -> Vec<(String, WebhookEndpoint)> {
        let endpoints = self.endpoints.read().await;
        endpoints
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Get backpressure statistics
    pub async fn backpressure_stats(&self) -> watchtower_core::BackpressureStats {
        self.backpressure.stats().await
    }
}

#[async_trait]
impl Subscriber for WebhookSubscriber {
    async fn subscribe(
        &mut self,
        event_types: Vec<String>,
        callback: EventCallback,
    ) -> Result<SubscriptionHandle, WatchtowerError> {
        let id = uuid::Uuid::new_v4().to_string();
        let handle = SubscriptionHandle::new(id.clone(), event_types.clone());

        let mut callbacks = self.callbacks.write().await;
        callbacks.insert(id, callback);

        Ok(handle)
    }

    async fn unsubscribe(&mut self, handle: &SubscriptionHandle) -> Result<(), WatchtowerError> {
        let mut callbacks = self.callbacks.write().await;
        callbacks.remove(&handle.id);
        Ok(())
    }

    async fn publish(&self, event: Event) -> Result<(), WatchtowerError> {
        // Apply backpressure
        self.backpressure.send(event.clone()).await?;

        // Process event from queue
        if let Some(queued_event) = self.backpressure.receive().await {
            let endpoints = self.endpoints.read().await;
            let callbacks = self.callbacks.read().await;

            let mut errors = Vec::new();

            // Deliver to registered webhook endpoints
            for (endpoint_id, endpoint) in endpoints.iter() {
                if endpoint.event_types.contains(&queued_event.event_type().to_string())
                    || endpoint.event_types.is_empty()
                {
                    if let Err(e) = self
                        .client
                        .send(&endpoint.url, &queued_event, endpoint.secret.as_deref())
                        .await
                    {
                        errors.push(format!("Endpoint {}: {}", endpoint_id, e));
                    }
                }
            }

            // Execute callbacks for matching subscriptions
            for (callback_id, callback) in callbacks.iter() {
                if let Err(e) = callback(queued_event.clone()).await {
                    errors.push(format!("Callback {}: {}", callback_id, e));
                }
            }

            if !errors.is_empty() {
                return Err(WatchtowerError::PublicationError(errors.join("; ")));
            }
        }

        Ok(())
    }
}
