//! WebSocket configuration

use serde::{Deserialize, Serialize};
use watchtower_core::BackpressureConfig;

/// WebSocket connection and behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// WebSocket server URL (e.g., "ws://localhost:8080/events")
    pub url: String,

    /// Enable automatic reconnection on disconnect
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,

    /// Reconnection retry attempts (0 = infinite)
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,

    /// Reconnection delay in seconds
    #[serde(default = "default_retry_delay")]
    pub retry_delay_seconds: u64,

    /// Ping interval in seconds (0 = disabled)
    #[serde(default = "default_ping_interval")]
    pub ping_interval_seconds: u64,

    /// Pong timeout in seconds
    #[serde(default = "default_pong_timeout")]
    pub pong_timeout_seconds: u64,

    /// Maximum message size in bytes
    #[serde(default = "default_max_message_size")]
    pub max_message_size: usize,

    /// Custom headers for WebSocket handshake
    #[serde(default)]
    pub headers: Vec<(String, String)>,

    /// Backpressure configuration
    #[serde(default)]
    pub backpressure: BackpressureConfig,
}

fn default_true() -> bool {
    true
}

fn default_retry_attempts() -> u32 {
    5
}

fn default_retry_delay() -> u64 {
    2
}

fn default_ping_interval() -> u64 {
    30
}

fn default_pong_timeout() -> u64 {
    10
}

fn default_max_message_size() -> usize {
    64 * 1024 * 1024 // 64MB
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            url: "ws://localhost:8080/events".to_string(),
            auto_reconnect: true,
            retry_attempts: 5,
            retry_delay_seconds: 2,
            ping_interval_seconds: 30,
            pong_timeout_seconds: 10,
            max_message_size: 64 * 1024 * 1024,
            headers: Vec::new(),
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
        let config = WebSocketConfig::default();

        assert_eq!(config.url, "ws://localhost:8080/events");
        assert!(config.auto_reconnect);
        assert_eq!(config.retry_attempts, 5);
        assert_eq!(config.retry_delay_seconds, 2);
        assert_eq!(config.ping_interval_seconds, 30);
        assert_eq!(config.pong_timeout_seconds, 10);
        assert_eq!(config.max_message_size, 64 * 1024 * 1024);
        assert!(config.headers.is_empty());
    }

    #[test]
    fn test_custom_config() {
        let config = WebSocketConfig {
            url: "wss://remote:8080/custom".to_string(),
            auto_reconnect: false,
            retry_attempts: 10,
            retry_delay_seconds: 5,
            ping_interval_seconds: 60,
            pong_timeout_seconds: 20,
            max_message_size: 10 * 1024 * 1024,
            headers: vec![
                ("Authorization".to_string(), "Bearer token".to_string()),
                ("X-Custom".to_string(), "value".to_string()),
            ],
            backpressure: BackpressureConfig {
                max_queue_size: 200,
                strategy: BackpressureStrategy::Block,
                warning_threshold: 0.7,
            },
        };

        assert_eq!(config.url, "wss://remote:8080/custom");
        assert!(!config.auto_reconnect);
        assert_eq!(config.retry_attempts, 10);
        assert_eq!(config.max_message_size, 10 * 1024 * 1024);
        assert_eq!(config.headers.len(), 2);
    }

    #[test]
    fn test_config_serialization() {
        let config = WebSocketConfig::default();

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: WebSocketConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.url, deserialized.url);
        assert_eq!(config.auto_reconnect, deserialized.auto_reconnect);
        assert_eq!(config.retry_attempts, deserialized.retry_attempts);
        assert_eq!(config.max_message_size, deserialized.max_message_size);
    }
}
