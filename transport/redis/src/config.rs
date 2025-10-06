//! Redis configuration

use serde::{Deserialize, Serialize};
use watchtower_core::BackpressureConfig;

/// Redis Streams configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,
    /// Stream name prefix
    pub stream_prefix: String,
    /// Consumer group name
    pub consumer_group: String,
    /// Consumer name
    pub consumer_name: String,
    /// Maximum stream length (0 = unlimited)
    pub max_stream_length: usize,
    /// Backpressure configuration
    pub backpressure: BackpressureConfig,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            stream_prefix: "watchtower:events".to_string(),
            consumer_group: "watchtower-group".to_string(),
            consumer_name: "watchtower-consumer".to_string(),
            max_stream_length: 10000,
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
        let config = RedisConfig::default();

        assert_eq!(config.url, "redis://localhost:6379");
        assert_eq!(config.stream_prefix, "watchtower:events");
        assert_eq!(config.consumer_group, "watchtower-group");
        assert_eq!(config.consumer_name, "watchtower-consumer");
        assert_eq!(config.max_stream_length, 10000);
        assert_eq!(config.backpressure.max_queue_size, 1000);
    }

    #[test]
    fn test_custom_config() {
        let config = RedisConfig {
            url: "redis://remote:6379".to_string(),
            stream_prefix: "custom:events".to_string(),
            consumer_group: "custom-group".to_string(),
            consumer_name: "custom-consumer".to_string(),
            max_stream_length: 5000,
            backpressure: BackpressureConfig {
                max_queue_size: 500,
                strategy: BackpressureStrategy::Block,
                warning_threshold: 0.9,
            },
        };

        assert_eq!(config.url, "redis://remote:6379");
        assert_eq!(config.stream_prefix, "custom:events");
        assert_eq!(config.consumer_group, "custom-group");
        assert_eq!(config.max_stream_length, 5000);
        assert_eq!(config.backpressure.max_queue_size, 500);
    }

    #[test]
    fn test_config_serialization() {
        let config = RedisConfig::default();

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RedisConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.url, deserialized.url);
        assert_eq!(config.stream_prefix, deserialized.stream_prefix);
        assert_eq!(config.consumer_group, deserialized.consumer_group);
        assert_eq!(config.max_stream_length, deserialized.max_stream_length);
    }
}
