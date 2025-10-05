//! Unified notification system with support for webhooks, NATS, Redis, RabbitMQ, and WebSocket

use async_trait::async_trait;
use watchtower_core::{Event, Subscriber, SubscriptionHandle, WatchtowerError};
use watchtower_core::subscriber::EventCallback;
use watchtower_nats::NatsSubscriber;
use watchtower_rabbitmq::RabbitMQSubscriber;
use watchtower_redis::RedisSubscriber;
use watchtower_webhook::WebhookSubscriber;
use watchtower_websocket::WebSocketSubscriber;

/// Unified notifier that can use webhooks, NATS, Redis, RabbitMQ, WebSocket, or any combination
pub struct UnifiedNotifier {
    webhook: Option<WebhookSubscriber>,
    nats: Option<NatsSubscriber>,
    redis: Option<RedisSubscriber>,
    rabbitmq: Option<RabbitMQSubscriber>,
    websocket: Option<WebSocketSubscriber>,
}

impl UnifiedNotifier {
    /// Create a new unified notifier with webhook
    pub fn with_webhook(config: watchtower_webhook::WebhookConfig) -> Result<Self, WatchtowerError> {
        Ok(Self {
            webhook: Some(WebhookSubscriber::new(config)?),
            nats: None,
            redis: None,
            rabbitmq: None,
            websocket: None,
        })
    }

    /// Create a new unified notifier with NATS
    pub async fn with_nats(config: watchtower_nats::NatsConfig) -> Result<Self, WatchtowerError> {
        Ok(Self {
            webhook: None,
            nats: Some(NatsSubscriber::new(config).await?),
            redis: None,
            rabbitmq: None,
            websocket: None,
        })
    }

    /// Create a new unified notifier with Redis
    pub async fn with_redis(config: watchtower_redis::RedisConfig) -> Result<Self, WatchtowerError> {
        Ok(Self {
            webhook: None,
            nats: None,
            redis: Some(RedisSubscriber::new(config).await?),
            rabbitmq: None,
            websocket: None,
        })
    }

    /// Create a new unified notifier with RabbitMQ
    pub async fn with_rabbitmq(config: watchtower_rabbitmq::RabbitMQConfig) -> Result<Self, WatchtowerError> {
        Ok(Self {
            webhook: None,
            nats: None,
            redis: None,
            rabbitmq: Some(RabbitMQSubscriber::new(config).await?),
            websocket: None,
        })
    }

    /// Create a new unified notifier with WebSocket
    pub async fn with_websocket(config: watchtower_websocket::WebSocketConfig) -> Result<Self, WatchtowerError> {
        Ok(Self {
            webhook: None,
            nats: None,
            redis: None,
            rabbitmq: None,
            websocket: Some(WebSocketSubscriber::new(config).await?),
        })
    }

    /// Create a new unified notifier with both webhook and NATS
    pub async fn with_webhook_and_nats(
        webhook_config: watchtower_webhook::WebhookConfig,
        nats_config: watchtower_nats::NatsConfig,
    ) -> Result<Self, WatchtowerError> {
        Ok(Self {
            webhook: Some(WebhookSubscriber::new(webhook_config)?),
            nats: Some(NatsSubscriber::new(nats_config).await?),
            redis: None,
            rabbitmq: None,
            websocket: None,
        })
    }

    /// Get mutable reference to webhook subscriber
    pub fn webhook_mut(&mut self) -> Option<&mut WebhookSubscriber> {
        self.webhook.as_mut()
    }

    /// Get mutable reference to NATS subscriber
    pub fn nats_mut(&mut self) -> Option<&mut NatsSubscriber> {
        self.nats.as_mut()
    }

    /// Get mutable reference to Redis subscriber
    pub fn redis_mut(&mut self) -> Option<&mut RedisSubscriber> {
        self.redis.as_mut()
    }

    /// Get mutable reference to RabbitMQ subscriber
    pub fn rabbitmq_mut(&mut self) -> Option<&mut RabbitMQSubscriber> {
        self.rabbitmq.as_mut()
    }

    /// Get mutable reference to WebSocket subscriber
    pub fn websocket_mut(&mut self) -> Option<&mut WebSocketSubscriber> {
        self.websocket.as_mut()
    }

    /// Start listening for subscriptions (NATS, RabbitMQ, and WebSocket if configured)
    pub async fn start_listening(&self) {
        if let Some(nats) = &self.nats {
            nats.start_listening().await;
        }
        if let Some(rabbitmq) = &self.rabbitmq {
            rabbitmq.start_listening().await;
        }
        if let Some(websocket) = &self.websocket {
            websocket.start_listening().await;
        }
    }

    /// Get backpressure statistics from webhook subscriber
    pub async fn webhook_backpressure_stats(&self) -> Option<watchtower_core::BackpressureStats> {
        if let Some(webhook) = &self.webhook {
            Some(webhook.backpressure_stats().await)
        } else {
            None
        }
    }

    /// Get backpressure statistics from NATS subscriber
    pub async fn nats_backpressure_stats(&self) -> Option<watchtower_core::BackpressureStats> {
        if let Some(nats) = &self.nats {
            Some(nats.backpressure_stats().await)
        } else {
            None
        }
    }

    /// Get backpressure statistics from Redis subscriber
    pub async fn redis_backpressure_stats(&self) -> Option<watchtower_core::BackpressureStats> {
        if let Some(redis) = &self.redis {
            Some(redis.backpressure_stats().await)
        } else {
            None
        }
    }

    /// Get backpressure statistics from RabbitMQ subscriber
    pub async fn rabbitmq_backpressure_stats(&self) -> Option<watchtower_core::BackpressureStats> {
        if let Some(rabbitmq) = &self.rabbitmq {
            Some(rabbitmq.backpressure_stats().await)
        } else {
            None
        }
    }

    /// Get backpressure statistics from WebSocket subscriber
    pub async fn websocket_backpressure_stats(&self) -> Option<watchtower_core::BackpressureStats> {
        if let Some(websocket) = &self.websocket {
            Some(websocket.backpressure_stats().await)
        } else {
            None
        }
    }
}

#[async_trait]
impl Subscriber for UnifiedNotifier {
    async fn subscribe(
        &mut self,
        event_types: Vec<String>,
        callback: EventCallback,
    ) -> Result<SubscriptionHandle, WatchtowerError> {
        let mut handles = Vec::new();

        // Subscribe to webhook if configured
        if let Some(webhook) = &mut self.webhook {
            let handle = webhook.subscribe(event_types.clone(), callback.clone()).await?;
            handles.push(handle);
        }

        // Subscribe to NATS if configured
        if let Some(nats) = &mut self.nats {
            let handle = nats.subscribe(event_types.clone(), callback.clone()).await?;
            handles.push(handle);
        }

        // Subscribe to Redis if configured
        if let Some(redis) = &mut self.redis {
            let handle = redis.subscribe(event_types.clone(), callback.clone()).await?;
            handles.push(handle);
        }

        // Subscribe to RabbitMQ if configured
        if let Some(rabbitmq) = &mut self.rabbitmq {
            let handle = rabbitmq.subscribe(event_types.clone(), callback.clone()).await?;
            handles.push(handle);
        }

        // Subscribe to WebSocket if configured
        if let Some(websocket) = &mut self.websocket {
            let handle = websocket.subscribe(event_types.clone(), callback).await?;
            handles.push(handle);
        }

        // Return the first handle (all backends will be notified)
        handles
            .into_iter()
            .next()
            .ok_or_else(|| WatchtowerError::InternalError("No subscription created".to_string()))
    }

    async fn unsubscribe(&mut self, handle: &SubscriptionHandle) -> Result<(), WatchtowerError> {
        let mut errors = Vec::new();

        if let Some(webhook) = &mut self.webhook {
            if let Err(e) = webhook.unsubscribe(handle).await {
                errors.push(format!("Webhook: {}", e));
            }
        }

        if let Some(nats) = &mut self.nats {
            if let Err(e) = nats.unsubscribe(handle).await {
                errors.push(format!("NATS: {}", e));
            }
        }

        if let Some(redis) = &mut self.redis {
            if let Err(e) = redis.unsubscribe(handle).await {
                errors.push(format!("Redis: {}", e));
            }
        }

        if let Some(rabbitmq) = &mut self.rabbitmq {
            if let Err(e) = rabbitmq.unsubscribe(handle).await {
                errors.push(format!("RabbitMQ: {}", e));
            }
        }

        if let Some(websocket) = &mut self.websocket {
            if let Err(e) = websocket.unsubscribe(handle).await {
                errors.push(format!("WebSocket: {}", e));
            }
        }

        if !errors.is_empty() {
            return Err(WatchtowerError::SubscriptionError(errors.join("; ")));
        }

        Ok(())
    }

    async fn publish(&self, event: Event) -> Result<(), WatchtowerError> {
        let mut errors = Vec::new();

        // Publish to webhook if configured
        if let Some(webhook) = &self.webhook {
            if let Err(e) = webhook.publish(event.clone()).await {
                errors.push(format!("Webhook: {}", e));
            }
        }

        // Publish to NATS if configured
        if let Some(nats) = &self.nats {
            if let Err(e) = nats.publish(event.clone()).await {
                errors.push(format!("NATS: {}", e));
            }
        }

        // Publish to Redis if configured
        if let Some(redis) = &self.redis {
            if let Err(e) = redis.publish(event.clone()).await {
                errors.push(format!("Redis: {}", e));
            }
        }

        // Publish to RabbitMQ if configured
        if let Some(rabbitmq) = &self.rabbitmq {
            if let Err(e) = rabbitmq.publish(event.clone()).await {
                errors.push(format!("RabbitMQ: {}", e));
            }
        }

        // Publish to WebSocket if configured
        if let Some(websocket) = &self.websocket {
            if let Err(e) = websocket.publish(event.clone()).await {
                errors.push(format!("WebSocket: {}", e));
            }
        }

        if !errors.is_empty() {
            return Err(WatchtowerError::PublicationError(errors.join("; ")));
        }

        Ok(())
    }
}
