//! # Watchtower
//!
//! A modular notification system library for webhooks and NATS.
//!
//! ## Features
//!
//! - **Modular architecture**: Use webhooks, NATS, or both
//! - **Unified API**: Single interface for all notification backends
//! - **Event-driven**: Subscribe to events and publish notifications
//! - **Extensible**: Easy to add custom notification backends
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use watchtower::prelude::*;
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), WatchtowerError> {
//!     // Configure notification system
//!     let config = NotificationConfig {
//!         webhook: Some(WebhookConfig::default()),
//!         nats: None,
//!     };
//!
//!     // Create unified notifier
//!     let mut notifier = UnifiedNotifier::new(config).await?;
//!
//!     // Create and publish an event
//!     let event = Event::new("user.created", json!({
//!         "user_id": "123",
//!         "email": "user@example.com"
//!     }));
//!
//!     notifier.publish(event).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod prelude;

// Re-export all public types from submodules
pub use watchtower_config::NotificationConfig;

pub use watchtower_core::{
    BackpressureConfig, BackpressureController, BackpressureStats, BackpressureStrategy, Event,
    EventMetadata, Subscriber, SubscriptionHandle, Transport, TransportInfo,
    TransportSubscription, WatchtowerError,
};

pub use watchtower_nats::{NatsClient, NatsConfig, NatsSubscriber};
pub use watchtower_rabbitmq::{ExchangeType, RabbitMQConfig, RabbitMQSubscriber, RabbitMQTransport};
pub use watchtower_redis::{RedisConfig, RedisSubscriber, RedisTransport};
pub use watchtower_unified::UnifiedNotifier;
pub use watchtower_webhook::{WebhookClient, WebhookConfig, WebhookSubscriber};
pub use watchtower_websocket::{WebSocketConfig, WebSocketSubscriber, WebSocketTransport};
pub use watchtower_websocket_server::{
    ClientId, ClientMetadata, ConnectionManager, WebSocketServerConfig, WebSocketServerTransport,
    websocket_handler, websocket_handler_with_metadata,
};
