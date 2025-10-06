//! Event types for notification system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Event metadata containing common information about the event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Unique event identifier
    pub id: Uuid,
    /// Event type/name
    pub event_type: String,
    /// Timestamp when the event was created
    pub timestamp: DateTime<Utc>,
    /// Optional source identifier (e.g., service name, user id)
    pub source: Option<String>,
    /// Optional correlation ID for tracking related events
    pub correlation_id: Option<Uuid>,
}

impl EventMetadata {
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: event_type.into(),
            timestamp: Utc::now(),
            source: None,
            correlation_id: None,
        }
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn set_source(&mut self, source: impl Into<String>) {
        self.source = Some(source.into());
    }

    pub fn with_correlation_id(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
}

/// Generic event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event metadata
    pub metadata: EventMetadata,
    /// Event payload (business data)
    pub payload: Value,
}

impl Event {
    pub fn new(event_type: impl Into<String>, payload: Value) -> Self {
        Self {
            metadata: EventMetadata::new(event_type),
            payload,
        }
    }

    pub fn with_metadata(event_type: impl Into<String>, payload: Value, mut metadata: EventMetadata) -> Self {
        // Override event_type in metadata with the provided one
        metadata.event_type = event_type.into();
        Self { metadata, payload }
    }

    /// Get event type
    pub fn event_type(&self) -> &str {
        &self.metadata.event_type
    }

    /// Get event ID
    pub fn id(&self) -> Uuid {
        self.metadata.id
    }

    /// Get event timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.metadata.timestamp
    }
}
