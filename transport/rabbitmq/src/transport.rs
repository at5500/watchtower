//! RabbitMQ transport implementation

use async_trait::async_trait;
use lapin::{
    options::*,
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind,
};
use tracing::{error, info, warn};

use crate::config::{ExchangeType, RabbitMQConfig};
use watchtower_core::{
    BackpressureController, Event, Transport, TransportInfo, TransportSubscription,
    WatchtowerError,
};

/// RabbitMQ transport
pub struct RabbitMQTransport {
    pub(crate) connection: Connection,
    channel: Channel,
    pub(crate) config: RabbitMQConfig,
    backpressure: BackpressureController,
}

impl RabbitMQTransport {
    /// Create a new RabbitMQ transport
    pub async fn new(config: RabbitMQConfig) -> Result<Self, WatchtowerError> {
        let mut retry_count = 0;
        let connection = loop {
            match Connection::connect(&config.url, ConnectionProperties::default()).await {
                Ok(conn) => break conn,
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= config.retry_attempts {
                        return Err(WatchtowerError::ConnectionError(format!(
                            "RabbitMQ connection failed after {} attempts: {}",
                            retry_count, e
                        )));
                    }
                    warn!(
                        attempt = retry_count,
                        max_attempts = config.retry_attempts,
                        "RabbitMQ connection failed, retrying..."
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(
                        config.retry_delay_seconds,
                    ))
                    .await;
                }
            }
        };

        let channel = connection
            .create_channel()
            .await
            .map_err(|e| WatchtowerError::ConnectionError(format!("Channel creation failed: {}", e)))?;

        // Declare exchange
        let exchange_kind = match config.exchange_type {
            ExchangeType::Direct => ExchangeKind::Direct,
            ExchangeType::Topic => ExchangeKind::Topic,
            ExchangeType::Fanout => ExchangeKind::Fanout,
            ExchangeType::Headers => ExchangeKind::Headers,
        };

        channel
            .exchange_declare(
                &config.exchange,
                exchange_kind,
                ExchangeDeclareOptions {
                    durable: config.durable,
                    auto_delete: config.auto_delete,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .map_err(|e| {
                WatchtowerError::ConnectionError(format!("Exchange declaration failed: {}", e))
            })?;

        // Declare dead letter exchange if configured
        if let Some(dlx) = &config.dead_letter_exchange {
            channel
                .exchange_declare(
                    dlx,
                    ExchangeKind::Topic,
                    ExchangeDeclareOptions {
                        durable: config.durable,
                        ..Default::default()
                    },
                    FieldTable::default(),
                )
                .await
                .map_err(|e| {
                    WatchtowerError::ConnectionError(format!(
                        "Dead letter exchange declaration failed: {}",
                        e
                    ))
                })?;
        }

        let backpressure = BackpressureController::new(
            config.backpressure.max_queue_size,
            config.backpressure.strategy,
            config.backpressure.warning_threshold,
        );

        info!(
            url = %config.url,
            exchange = %config.exchange,
            exchange_type = %config.exchange_type.as_str(),
            "Connected to RabbitMQ"
        );

        Ok(Self {
            connection,
            channel,
            config,
            backpressure,
        })
    }

    /// Get routing key for event type
    fn routing_key(&self, event_type: &str) -> String {
        event_type.to_string()
    }

    /// Get backpressure statistics
    pub async fn backpressure_stats(&self) -> watchtower_core::BackpressureStats {
        self.backpressure.stats().await
    }

    /// Build message properties
    fn build_properties(&self, event: &Event) -> BasicProperties {
        let mut properties = BasicProperties::default();

        if self.config.persistent {
            properties = properties.with_delivery_mode(2); // persistent
        }

        properties = properties
            .with_content_type("application/json".into())
            .with_message_id(event.id().to_string().into());

        properties
    }
}

#[async_trait]
impl Transport for RabbitMQTransport {
    fn info(&self) -> TransportInfo {
        TransportInfo {
            name: "rabbitmq".to_string(),
            version: "0.1.0".to_string(),
            supports_subscriptions: true,
            supports_backpressure: true,
        }
    }

    async fn publish(&self, event: Event) -> Result<(), WatchtowerError> {
        // Apply backpressure
        self.backpressure.send(event.clone()).await?;

        // Process event from queue
        if let Some(queued_event) = self.backpressure.receive().await {
            let routing_key = self.routing_key(queued_event.event_type());

            let payload =
                serde_json::to_vec(&queued_event).map_err(WatchtowerError::SerializationError)?;

            let properties = self.build_properties(&queued_event);

            self.channel
                .basic_publish(
                    &self.config.exchange,
                    &routing_key,
                    BasicPublishOptions::default(),
                    &payload,
                    properties,
                )
                .await
                .map_err(|e| {
                    error!(
                        exchange = %self.config.exchange,
                        routing_key = %routing_key,
                        event_id = %queued_event.id(),
                        error = %e,
                        "Failed to publish to RabbitMQ"
                    );
                    WatchtowerError::PublicationError(format!("RabbitMQ publish failed: {}", e))
                })?
                .await
                .map_err(|e| {
                    error!(
                        exchange = %self.config.exchange,
                        routing_key = %routing_key,
                        event_id = %queued_event.id(),
                        error = %e,
                        "Failed to confirm RabbitMQ publish"
                    );
                    WatchtowerError::PublicationError(format!(
                        "RabbitMQ publish confirmation failed: {}",
                        e
                    ))
                })?;

            info!(
                exchange = %self.config.exchange,
                routing_key = %routing_key,
                event_id = %queued_event.id(),
                event_type = %queued_event.event_type(),
                "Event published to RabbitMQ"
            );
        }

        Ok(())
    }

    async fn subscribe(&self, pattern: &str) -> Result<TransportSubscription, WatchtowerError> {
        let queue_name = format!("{}.{}", self.config.queue_prefix, pattern);

        // Build queue arguments
        let mut arguments = FieldTable::default();

        if let Some(dlx) = &self.config.dead_letter_exchange {
            arguments.insert("x-dead-letter-exchange".into(), lapin::types::AMQPValue::LongString(dlx.clone().into()));
        }

        if self.config.message_ttl > 0 {
            arguments.insert("x-message-ttl".into(), self.config.message_ttl.into());
        }

        if self.config.max_priority > 0 {
            arguments.insert("x-max-priority".into(), self.config.max_priority.into());
        }

        // Declare queue
        self.channel
            .queue_declare(
                &queue_name,
                QueueDeclareOptions {
                    durable: self.config.durable,
                    auto_delete: self.config.auto_delete,
                    ..Default::default()
                },
                arguments,
            )
            .await
            .map_err(|e| {
                WatchtowerError::SubscriptionError(format!("Queue declaration failed: {}", e))
            })?;

        // Bind queue to exchange with routing key pattern
        self.channel
            .queue_bind(
                &queue_name,
                &self.config.exchange,
                pattern,
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(|e| {
                WatchtowerError::SubscriptionError(format!("Queue bind failed: {}", e))
            })?;

        // Set QoS (prefetch count)
        self.channel
            .basic_qos(self.config.prefetch_count, BasicQosOptions::default())
            .await
            .map_err(|e| WatchtowerError::SubscriptionError(format!("QoS setup failed: {}", e)))?;

        info!(
            queue = %queue_name,
            pattern = %pattern,
            "Queue created and bound to exchange"
        );

        Ok(TransportSubscription::new(
            uuid::Uuid::new_v4().to_string(),
            queue_name,
            "rabbitmq",
        ))
    }

    async fn health_check(&self) -> Result<(), WatchtowerError> {
        if self.connection.status().connected() {
            Ok(())
        } else {
            Err(WatchtowerError::ConnectionError(
                "RabbitMQ connection is not active".to_string(),
            ))
        }
    }

    async fn shutdown(&self) -> Result<(), WatchtowerError> {
        info!("Shutting down RabbitMQ transport");

        if let Err(e) = self.channel.close(200, "Normal shutdown").await {
            warn!(error = %e, "Failed to close RabbitMQ channel gracefully");
        }

        if let Err(e) = self.connection.close(200, "Normal shutdown").await {
            warn!(error = %e, "Failed to close RabbitMQ connection gracefully");
        }

        Ok(())
    }
}
