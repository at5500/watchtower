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
