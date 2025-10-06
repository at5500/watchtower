//! NATS configuration

use serde::{Deserialize, Serialize};
use watchtower_core::BackpressureConfig;

/// NATS notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    /// NATS server URL
    pub url: String,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Reconnection delay in seconds
    pub reconnect_delay_seconds: u64,
    /// Backpressure configuration
    pub backpressure: BackpressureConfig,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: "nats://localhost:4222".to_string(),
            max_reconnect_attempts: 10,
            reconnect_delay_seconds: 2,
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
        let config = NatsConfig::default();

        assert_eq!(config.url, "nats://localhost:4222");
        assert_eq!(config.max_reconnect_attempts, 10);
        assert_eq!(config.reconnect_delay_seconds, 2);
        assert_eq!(config.backpressure.max_queue_size, 1000);
    }

    #[test]
    fn test_custom_config() {
        let config = NatsConfig {
            url: "nats://remote:4222".to_string(),
            max_reconnect_attempts: 5,
            reconnect_delay_seconds: 10,
            backpressure: BackpressureConfig {
                max_queue_size: 500,
                strategy: BackpressureStrategy::DropNewest,
                warning_threshold: 0.9,
            },
        };

        assert_eq!(config.url, "nats://remote:4222");
        assert_eq!(config.max_reconnect_attempts, 5);
        assert_eq!(config.reconnect_delay_seconds, 10);
        assert_eq!(config.backpressure.max_queue_size, 500);
    }

    #[test]
    fn test_config_serialization() {
        let config = NatsConfig::default();

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: NatsConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.url, deserialized.url);
        assert_eq!(config.max_reconnect_attempts, deserialized.max_reconnect_attempts);
    }
}
