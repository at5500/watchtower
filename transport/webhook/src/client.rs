//! HTTP client for webhook delivery

use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use crate::config::WebhookConfig;
use watchtower_core::{CircuitBreaker, CircuitBreakerConfig, Event, WatchtowerError};

type HmacSha256 = Hmac<Sha256>;

/// Dead Letter Queue entry for webhook
#[derive(Clone)]
struct DlqEntry {
    event: Event,
    error: String,
}

/// HTTP client for delivering webhooks
pub struct WebhookClient {
    client: Client,
    config: WebhookConfig,
    dlq: Arc<Mutex<VecDeque<DlqEntry>>>,
    circuit_breakers: Arc<Mutex<HashMap<String, Arc<CircuitBreaker>>>>,
}

impl WebhookClient {
    pub fn new(config: WebhookConfig) -> Result<Self, WatchtowerError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .danger_accept_invalid_certs(!config.verify_ssl)
            .build()
            .map_err(|e| WatchtowerError::HttpError(e.to_string()))?;

        Ok(Self {
            client,
            config,
            dlq: Arc::new(Mutex::new(VecDeque::new())),
            circuit_breakers: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get or create circuit breaker for URL
    async fn get_circuit_breaker(&self, url: &str) -> Arc<CircuitBreaker> {
        let mut breakers = self.circuit_breakers.lock().await;

        if let Some(breaker) = breakers.get(url) {
            return breaker.clone();
        }

        let breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
        breakers.insert(url.to_string(), breaker.clone());
        breaker
    }

    /// Send event to webhook URL
    pub async fn send(
        &self,
        url: &str,
        event: &Event,
        secret: Option<&str>,
    ) -> Result<(), WatchtowerError> {
        let circuit_breaker = self.get_circuit_breaker(url).await;

        // Check circuit breaker
        if !circuit_breaker.should_allow_request().await {
            let stats = circuit_breaker.stats().await;
            warn!(
                url = %url,
                state = ?stats.state,
                "Circuit breaker is open, rejecting request"
            );
            return Err(WatchtowerError::HttpError(
                "Circuit breaker is open".to_string()
            ));
        }

        let json_payload =
            serde_json::to_string(event).map_err(WatchtowerError::SerializationError)?;

        for attempt in 1..=self.config.retry_attempts {
            let mut request = self
                .client
                .post(url)
                .header("Content-Type", "application/json")
                .header("User-Agent", "Watchtower/0.1.0")
                .body(json_payload.clone());

            // Add HMAC signature if secret is provided
            if let Some(secret) = secret {
                let signature = self.generate_signature(secret, &json_payload)?;
                request = request.header("X-Watchtower-Signature", signature);
            }

            match request.send().await {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        info!(
                            url = %url,
                            event_id = %event.id(),
                            attempt = attempt,
                            "Webhook delivered successfully"
                        );
                        circuit_breaker.record_success().await;
                        return Ok(());
                    } else {
                        let body = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unable to read response".to_string());

                        warn!(
                            url = %url,
                            event_id = %event.id(),
                            status = %status,
                            attempt = attempt,
                            body = %body,
                            "Webhook delivery failed"
                        );

                        if attempt == self.config.retry_attempts {
                            circuit_breaker.record_failure().await;
                            return Err(WatchtowerError::HttpError(format!(
                                "Webhook delivery failed: status {}, body: {}",
                                status, body
                            )));
                        }
                    }
                }
                Err(err) => {
                    error!(
                        url = %url,
                        event_id = %event.id(),
                        attempt = attempt,
                        error = %err,
                        "Webhook request failed"
                    );

                    if attempt == self.config.retry_attempts {
                        circuit_breaker.record_failure().await;
                        return Err(WatchtowerError::HttpError(err.to_string()));
                    }
                }
            }

            // Exponential backoff: 1s, 2s, 4s...
            let delay = tokio::time::Duration::from_secs(2_u64.pow(attempt - 1));
            tokio::time::sleep(delay).await;
        }

        Err(WatchtowerError::HttpError(
            "Max retry attempts reached".to_string(),
        ))
    }

    /// Generate HMAC-SHA256 signature for webhook payload
    fn generate_signature(&self, secret: &str, payload: &str) -> Result<String, WatchtowerError> {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| WatchtowerError::InternalError(e.to_string()))?;

        mac.update(payload.as_bytes());
        let hash = mac.finalize().into_bytes();

        Ok(format!("sha256={:x}", hash))
    }

    /// Publish event to Dead Letter Queue
    pub async fn publish_to_dlq(&self, event: Event, error: &WatchtowerError) -> Result<(), WatchtowerError> {
        let mut dlq = self.dlq.lock().await;

        dlq.push_back(DlqEntry {
            event: event.clone(),
            error: error.to_string(),
        });

        // Limit DLQ size to prevent memory overflow
        const MAX_DLQ_SIZE: usize = 10000;
        while dlq.len() > MAX_DLQ_SIZE {
            dlq.pop_front();
        }

        info!(
            event_id = %event.id(),
            event_type = %event.event_type(),
            dlq_size = dlq.len(),
            "Webhook event added to Dead Letter Queue (in-memory)"
        );

        Ok(())
    }

    /// Consume events from Dead Letter Queue with retry
    pub async fn consume_dlq(
        &self,
        callback: watchtower_core::subscriber::EventCallback,
    ) -> Result<(), WatchtowerError> {
        let dlq = self.dlq.clone();

        info!("Started consuming from Webhook Dead Letter Queue (in-memory)");

        tokio::spawn(async move {
            loop {
                let entry = {
                    let mut queue = dlq.lock().await;
                    queue.pop_front()
                };

                if let Some(dlq_entry) = entry {
                    if let Err(e) = callback(dlq_entry.event.clone()).await {
                        error!(
                            event_id = %dlq_entry.event.id(),
                            error = %e,
                            "DLQ callback execution failed, re-queuing with exponential backoff"
                        );

                        // Re-queue failed event
                        let mut queue = dlq.lock().await;
                        queue.push_back(dlq_entry);

                        // Exponential backoff before retrying
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                } else {
                    // No items in queue, wait before checking again
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        });

        Ok(())
    }

    /// Get circuit breaker statistics for a URL
    pub async fn circuit_breaker_stats(&self, url: &str) -> Option<watchtower_core::CircuitBreakerStats> {
        let breakers = self.circuit_breakers.lock().await;
        if let Some(breaker) = breakers.get(url) {
            Some(breaker.stats().await)
        } else {
            None
        }
    }

    /// Get all circuit breaker statistics
    pub async fn all_circuit_breaker_stats(&self) -> Vec<(String, watchtower_core::CircuitBreakerStats)> {
        let breakers = self.circuit_breakers.lock().await;
        let mut stats = Vec::new();
        for (url, breaker) in breakers.iter() {
            stats.push((url.clone(), breaker.stats().await));
        }
        stats
    }
}
