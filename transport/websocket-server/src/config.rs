//! WebSocket server configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// WebSocket server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketServerConfig {
    /// Maximum number of concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Ping interval for keepalive (seconds)
    #[serde(default = "default_ping_interval")]
    pub ping_interval_secs: u64,

    /// Pong timeout (seconds)
    #[serde(default = "default_pong_timeout")]
    pub pong_timeout_secs: u64,

    /// Maximum message size in bytes
    #[serde(default = "default_max_message_size")]
    pub max_message_size: usize,

    /// Broadcast channel buffer size
    #[serde(default = "default_broadcast_buffer")]
    pub broadcast_buffer_size: usize,

    /// Enable authentication
    #[serde(default)]
    pub require_auth: bool,
}

impl Default for WebSocketServerConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            ping_interval_secs: default_ping_interval(),
            pong_timeout_secs: default_pong_timeout(),
            max_message_size: default_max_message_size(),
            broadcast_buffer_size: default_broadcast_buffer(),
            require_auth: false,
        }
    }
}

impl WebSocketServerConfig {
    /// Create new configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum connections
    pub fn with_max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    /// Set ping interval
    pub fn with_ping_interval(mut self, secs: u64) -> Self {
        self.ping_interval_secs = secs;
        self
    }

    /// Set pong timeout
    pub fn with_pong_timeout(mut self, secs: u64) -> Self {
        self.pong_timeout_secs = secs;
        self
    }

    /// Set max message size
    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = size;
        self
    }

    /// Set broadcast buffer size
    pub fn with_broadcast_buffer(mut self, size: usize) -> Self {
        self.broadcast_buffer_size = size;
        self
    }

    /// Enable authentication
    pub fn with_auth(mut self, require: bool) -> Self {
        self.require_auth = require;
        self
    }

    /// Get ping interval as Duration
    pub fn ping_interval(&self) -> Duration {
        Duration::from_secs(self.ping_interval_secs)
    }

    /// Get pong timeout as Duration
    pub fn pong_timeout(&self) -> Duration {
        Duration::from_secs(self.pong_timeout_secs)
    }
}

fn default_max_connections() -> usize {
    10000
}

fn default_ping_interval() -> u64 {
    30
}

fn default_pong_timeout() -> u64 {
    60
}

fn default_max_message_size() -> usize {
    1024 * 1024 // 1MB
}

fn default_broadcast_buffer() -> usize {
    1000
}