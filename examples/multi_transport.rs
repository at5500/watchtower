//! Multi-transport integration example
//!
//! This example demonstrates using multiple transports together:
//! - NATS for internal service communication
//! - Redis Streams for event sourcing
//! - Webhook for external API notifications
//! - RabbitMQ for task queues

use std::sync::Arc;
use watchtower_core::prelude::*;
use watchtower_nats::prelude::*;
use watchtower_redis::prelude::*;
use watchtower_webhook::prelude::*;
use watchtower_webhook::subscriber::WebhookEndpoint;
use watchtower_rabbitmq::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Watchtower Multi-Transport Example ===\n");

    // 1. Setup NATS for real-time internal messaging
    println!("📡 Setting up NATS for internal messaging...");
    let nats_config = NatsConfig {
        url: "nats://localhost:4222".to_string(),
        max_reconnect_attempts: 5,
        reconnect_delay_seconds: 2,
        backpressure: BackpressureConfig::default(),
    };
    let mut nats_sub = NatsSubscriber::new(nats_config).await?;
    println!("✅ NATS ready\n");

    // 2. Setup Redis Streams for event sourcing
    println!("📡 Setting up Redis Streams for event sourcing...");
    let redis_config = RedisConfig {
        url: "redis://localhost:6379".to_string(),
        stream_prefix: "app:events".to_string(),
        consumer_group: "app-processors".to_string(),
        consumer_name: "processor-1".to_string(),
        max_stream_length: 10000,
        backpressure: BackpressureConfig::default(),
    };
    let mut redis_sub = RedisSubscriber::new(redis_config).await?;
    println!("✅ Redis Streams ready\n");

    // 3. Setup RabbitMQ for task queues
    println!("📡 Setting up RabbitMQ for task processing...");
    let rabbitmq_config = RabbitMQConfig {
        url: "amqp://guest:guest@localhost:5672/%2f".to_string(),
        exchange: "app.tasks".to_string(),
        exchange_type: ExchangeType::Direct,
        queue_prefix: "tasks".to_string(),
        durable: true,
        persistent: true,
        auto_delete: false,
        dead_letter_exchange: Some("app.tasks.dlx".to_string()),
        message_ttl: 0,
        max_priority: 10,
        prefetch_count: 10,
        retry_attempts: 3,
        retry_delay_seconds: 2,
        backpressure: BackpressureConfig::default(),
    };
    let mut rabbitmq_sub = RabbitMQSubscriber::new(rabbitmq_config).await?;
    println!("✅ RabbitMQ ready\n");

    // 4. Setup Webhook for external notifications
    println!("📡 Setting up Webhook for external APIs...");
    let webhook_config = WebhookConfig {
        retry_attempts: 3,
        timeout_seconds: 30,
        verify_ssl: true,
        backpressure: BackpressureConfig::default(),
    };
    let mut webhook_sub = WebhookSubscriber::new(webhook_config)?;

    let external_api = WebhookEndpoint {
        url: "http://localhost:3000/webhooks/external".to_string(),
        secret: Some("external-api-secret".to_string()),
        event_types: vec!["order.completed".to_string(), "payment.confirmed".to_string()],
    };
    webhook_sub.register_endpoint("external-api", external_api).await?;
    println!("✅ Webhook ready\n");

    // Setup event handlers
    println!("🔔 Setting up event handlers...\n");

    // NATS: Handle real-time user events
    let nats_handle = nats_sub
        .subscribe(
            vec!["user.logged_in".to_string(), "user.action".to_string()],
            Arc::new(|event| {
                Box::pin(async move {
                    println!("⚡ [NATS] Real-time event: {}", event.event_type());
                    println!("   Payload: {}", serde_json::to_string_pretty(&event.payload)?);
                    Ok(())
                })
            }),
        )
        .await?;

    // Redis: Store all events for event sourcing
    let redis_handle = redis_sub
        .subscribe(
            vec!["order.created".to_string(), "order.completed".to_string()],
            Arc::new(|event| {
                Box::pin(async move {
                    println!("💾 [Redis] Persisting event: {}", event.event_type());
                    println!("   Event ID: {}", event.id());
                    Ok(())
                })
            }),
        )
        .await?;

    // RabbitMQ: Process background tasks
    let rabbitmq_handle = rabbitmq_sub
        .subscribe(
            vec!["task.process_order".to_string(), "task.send_email".to_string()],
            Arc::new(|event| {
                Box::pin(async move {
                    println!("⚙️  [RabbitMQ] Processing task: {}", event.event_type());
                    println!("   Task data: {}", serde_json::to_string_pretty(&event.payload)?);
                    // Simulate task processing
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    println!("   ✅ Task completed\n");
                    Ok(())
                })
            }),
        )
        .await?;

    println!("✅ All handlers ready!\n");

    // Simulate a complete order workflow
    println!("📋 Simulating order workflow...\n");

    // 1. User logs in (NATS - real-time)
    println!("Step 1: User logs in");
    nats_sub
        .publish(Event::new(
            "user.logged_in",
            serde_json::json!({
                "user_id": 12345,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        ))
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // 2. User creates order (Redis - event sourcing)
    println!("\nStep 2: Order created");
    redis_sub
        .publish(Event::new(
            "order.created",
            serde_json::json!({
                "order_id": "ORD-001",
                "user_id": 12345,
                "items": [
                    {"product": "Widget", "price": 29.99},
                    {"product": "Gadget", "price": 49.99}
                ],
                "total": 79.98
            }),
        ))
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // 3. Queue task to process order (RabbitMQ - task queue)
    println!("\nStep 3: Queueing order processing task");
    rabbitmq_sub
        .publish(Event::new(
            "task.process_order",
            serde_json::json!({
                "order_id": "ORD-001",
                "priority": 5
            }),
        ))
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 4. Order completed (Redis - event sourcing)
    println!("\nStep 4: Order completed");
    redis_sub
        .publish(Event::new(
            "order.completed",
            serde_json::json!({
                "order_id": "ORD-001",
                "status": "shipped",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        ))
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // 5. Notify external API (Webhook)
    println!("\nStep 5: Notifying external API");
    webhook_sub
        .publish(Event::new(
            "order.completed",
            serde_json::json!({
                "order_id": "ORD-001",
                "external_reference": "EXT-12345"
            }),
        ))
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 6. Queue email task (RabbitMQ)
    println!("\nStep 6: Queueing email notification");
    rabbitmq_sub
        .publish(Event::new(
            "task.send_email",
            serde_json::json!({
                "to": "user@example.com",
                "template": "order_confirmation",
                "order_id": "ORD-001"
            }),
        ))
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Cleanup
    println!("\n🧹 Cleaning up...");
    nats_sub.unsubscribe(&nats_handle).await?;
    redis_sub.unsubscribe(&redis_handle).await?;
    rabbitmq_sub.unsubscribe(&rabbitmq_handle).await?;

    println!("\n✅ Multi-transport example completed!\n");
    println!("Key patterns demonstrated:");
    println!("  • NATS: Real-time internal messaging");
    println!("  • Redis Streams: Event sourcing and persistence");
    println!("  • RabbitMQ: Reliable task queue with DLX");
    println!("  • Webhook: External API integration");
    println!("\nEach transport serves a specific purpose:");
    println!("  • Use NATS for speed and simplicity");
    println!("  • Use Redis for persistence and replay");
    println!("  • Use RabbitMQ for guaranteed delivery");
    println!("  • Use Webhook for external integration");

    Ok(())
}
