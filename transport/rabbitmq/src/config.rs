//! RabbitMQ configuration

use serde::{Deserialize, Serialize};
use watchtower_core::BackpressureConfig;

/// RabbitMQ connection and behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RabbitMQConfig {
    /// RabbitMQ connection URL (e.g., "amqp://user:pass@localhost:5672/%2f")
    pub url: String,

    /// Exchange name for publishing events
    pub exchange: String,

    /// Exchange type (direct, topic, fanout, headers)
    #[serde(default = "default_exchange_type")]
    pub exchange_type: ExchangeType,

    /// Queue name prefix for consumers
    #[serde(default = "default_queue_prefix")]
    pub queue_prefix: String,

    /// Enable durable exchanges and queues
    #[serde(default = "default_true")]
    pub durable: bool,

    /// Enable message persistence
    #[serde(default = "default_true")]
    pub persistent: bool,

    /// Auto-delete queue when no consumers
    #[serde(default)]
    pub auto_delete: bool,

    /// Dead letter exchange for failed messages
    pub dead_letter_exchange: Option<String>,

    /// Message TTL in milliseconds (0 = no TTL)
    #[serde(default)]
    pub message_ttl: u32,

    /// Maximum message priority (0-255, 0 = disabled)
    #[serde(default)]
    pub max_priority: u8,

    /// Prefetch count for consumers
    #[serde(default = "default_prefetch_count")]
    pub prefetch_count: u16,

    /// Connection retry attempts
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,

    /// Connection retry delay in seconds
    #[serde(default = "default_retry_delay")]
    pub retry_delay_seconds: u64,

    /// Backpressure configuration
    #[serde(default)]
    pub backpressure: BackpressureConfig,
}

/// RabbitMQ exchange types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExchangeType {
    Direct,
    Topic,
    Fanout,
    Headers,
}

impl ExchangeType {
    pub fn as_str(&self) -> &str {
        match self {
            ExchangeType::Direct => "direct",
            ExchangeType::Topic => "topic",
            ExchangeType::Fanout => "fanout",
            ExchangeType::Headers => "headers",
        }
    }
}

fn default_exchange_type() -> ExchangeType {
    ExchangeType::Topic
}

fn default_queue_prefix() -> String {
    "watchtower".to_string()
}

fn default_true() -> bool {
    true
}

fn default_prefetch_count() -> u16 {
    10
}

fn default_retry_attempts() -> u32 {
    5
}

fn default_retry_delay() -> u64 {
    2
}

impl Default for RabbitMQConfig {
    fn default() -> Self {
        Self {
            url: "amqp://guest:guest@localhost:5672/%2f".to_string(),
            exchange: "watchtower.events".to_string(),
            exchange_type: ExchangeType::Topic,
            queue_prefix: "watchtower".to_string(),
            durable: true,
            persistent: true,
            auto_delete: false,
            dead_letter_exchange: None,
            message_ttl: 0,
            max_priority: 0,
            prefetch_count: 10,
            retry_attempts: 5,
            retry_delay_seconds: 2,
            backpressure: BackpressureConfig::default(),
        }
    }
}
