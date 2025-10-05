//! Webhook configuration

use serde::{Deserialize, Serialize};
use watchtower_core::BackpressureConfig;

/// Webhook notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Maximum number of retry attempts for failed webhook deliveries
    pub retry_attempts: u32,
    /// Timeout for webhook requests in seconds
    pub timeout_seconds: u64,
    /// Whether to verify SSL certificates
    pub verify_ssl: bool,
    /// Backpressure configuration
    pub backpressure: BackpressureConfig,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            retry_attempts: 3,
            timeout_seconds: 30,
            verify_ssl: true,
            backpressure: BackpressureConfig::default(),
        }
    }
}
