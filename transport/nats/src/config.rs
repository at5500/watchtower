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
