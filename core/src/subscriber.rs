//! Subscriber trait and subscription management

use crate::event::Event;
use crate::errors::WatchtowerError;
use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;

/// Callback type for event handling
pub type EventCallback = std::sync::Arc<
    dyn Fn(Event) -> Pin<Box<dyn Future<Output = Result<(), WatchtowerError>> + Send>> + Send + Sync,
>;

/// Handle for managing subscription lifecycle
#[derive(Debug, Clone)]
pub struct SubscriptionHandle {
    pub id: String,
    pub event_types: Vec<String>,
}

impl SubscriptionHandle {
    pub fn new(id: impl Into<String>, event_types: Vec<String>) -> Self {
        Self {
            id: id.into(),
            event_types,
        }
    }
}

/// Trait for subscribing to events
#[async_trait]
pub trait Subscriber: Send + Sync {
    /// Subscribe to specific event types
    async fn subscribe(
        &mut self,
        event_types: Vec<String>,
        callback: EventCallback,
    ) -> Result<SubscriptionHandle, WatchtowerError>;

    /// Unsubscribe from events
    async fn unsubscribe(&mut self, handle: &SubscriptionHandle) -> Result<(), WatchtowerError>;

    /// Publish an event
    async fn publish(&self, event: Event) -> Result<(), WatchtowerError>;
}
