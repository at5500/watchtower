//! Transport trait for pluggable notification backends

use crate::event::Event;
use crate::errors::WatchtowerError;
use async_trait::async_trait;

/// Transport metadata
#[derive(Debug, Clone)]
pub struct TransportInfo {
    /// Transport name (e.g., "webhook", "nats", "redis")
    pub name: String,
    /// Transport version
    pub version: String,
    /// Whether transport supports subscriptions
    pub supports_subscriptions: bool,
    /// Whether transport supports backpressure
    pub supports_backpressure: bool,
}

/// Generic transport trait for all notification backends
#[async_trait]
pub trait Transport: Send + Sync {
    /// Get transport information
    fn info(&self) -> TransportInfo;

    /// Publish an event to the transport
    async fn publish(&self, event: Event) -> Result<(), WatchtowerError>;

    /// Subscribe to events matching pattern (optional)
    /// Pattern format is transport-specific:
    /// - Webhook: not supported (returns error)
    /// - NATS: subject pattern (e.g., "events.*")
    /// - Redis: stream key pattern
    async fn subscribe(&self, _pattern: &str) -> Result<TransportSubscription, WatchtowerError> {
        Err(WatchtowerError::InternalError(
            format!("{} does not support subscriptions", self.info().name)
        ))
    }

    /// Check if transport is healthy/connected
    async fn health_check(&self) -> Result<(), WatchtowerError> {
        Ok(())
    }

    /// Gracefully shutdown the transport
    async fn shutdown(&self) -> Result<(), WatchtowerError> {
        Ok(())
    }
}

/// Subscription handle for transport subscriptions
pub struct TransportSubscription {
    pub id: String,
    pub pattern: String,
    pub transport_name: String,
}

impl TransportSubscription {
    pub fn new(id: impl Into<String>, pattern: impl Into<String>, transport_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            pattern: pattern.into(),
            transport_name: transport_name.into(),
        }
    }
}
