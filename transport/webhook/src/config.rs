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

#[cfg(test)]
mod tests {
    use super::*;
    use watchtower_core::BackpressureStrategy;

    #[test]
    fn test_default_config() {
        let config = WebhookConfig::default();

        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.timeout_seconds, 30);
        assert!(config.verify_ssl);
        assert_eq!(config.backpressure.max_queue_size, 1000);
    }

    #[test]
    fn test_custom_config() {
        let config = WebhookConfig {
            retry_attempts: 5,
            timeout_seconds: 60,
            verify_ssl: false,
            backpressure: BackpressureConfig {
                max_queue_size: 500,
                strategy: BackpressureStrategy::DropOldest,
                warning_threshold: 0.9,
            },
        };

        assert_eq!(config.retry_attempts, 5);
        assert_eq!(config.timeout_seconds, 60);
        assert!(!config.verify_ssl);
        assert_eq!(config.backpressure.max_queue_size, 500);
    }

    #[test]
    fn test_config_serialization() {
        let config = WebhookConfig::default();

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: WebhookConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.retry_attempts, deserialized.retry_attempts);
        assert_eq!(config.timeout_seconds, deserialized.timeout_seconds);
        assert_eq!(config.verify_ssl, deserialized.verify_ssl);
    }
}
