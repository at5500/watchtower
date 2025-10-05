//! HTTP client for webhook delivery

use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use tracing::{error, info, warn};
use crate::config::WebhookConfig;
use watchtower_core::{Event, WatchtowerError};

type HmacSha256 = Hmac<Sha256>;

/// HTTP client for delivering webhooks
pub struct WebhookClient {
    client: Client,
    config: WebhookConfig,
}

impl WebhookClient {
    pub fn new(config: WebhookConfig) -> Result<Self, WatchtowerError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .danger_accept_invalid_certs(!config.verify_ssl)
            .build()
            .map_err(|e| WatchtowerError::HttpError(e.to_string()))?;

        Ok(Self { client, config })
    }

    /// Send event to webhook URL
    pub async fn send(
        &self,
        url: &str,
        event: &Event,
        secret: Option<&str>,
    ) -> Result<(), WatchtowerError> {
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
}
