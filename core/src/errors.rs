//! Error types for Watchtower

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WatchtowerError {
    #[error("Subscription error: {0}")]
    SubscriptionError(String),

    #[error("Publication error: {0}")]
    PublicationError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Callback execution error: {0}")]
    CallbackError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("NATS error: {0}")]
    NatsError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}
