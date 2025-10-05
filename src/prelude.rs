//! Prelude module for convenient imports

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
