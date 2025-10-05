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
