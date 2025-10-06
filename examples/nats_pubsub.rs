//! NATS publish/subscribe example
//!
//! This example demonstrates basic NATS usage with Watchtower:
//! - Publishing events
//! - Subscribing to specific event types
//! - Subject-based routing
//! - Queue groups for load balancing

use std::sync::Arc;
use watchtower_core::prelude::*;
use watchtower_nats::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Watchtower NATS Pub/Sub Example ===\n");

    // Configure NATS transport
    let config = NatsConfig {
        url: "nats://localhost:4222".to_string(),
        max_reconnect_attempts: 10,
        reconnect_delay_seconds: 2,
        backpressure: BackpressureConfig {
            max_queue_size: 1000,
            strategy: BackpressureStrategy::DropOldest,
            warning_threshold: 0.8,
        },
    };

    println!("📡 Connecting to NATS server at {}", config.url);
    let mut subscriber = NatsSubscriber::new(config).await?;
    println!("✅ Connected!\n");

    // Subscribe to user events
    println!("🔔 Setting up subscriptions...");
    let user_handle = subscriber
        .subscribe(
            vec!["user.created".to_string(), "user.updated".to_string()],
            Arc::new(|event| {
                Box::pin(async move {
                    println!("👤 User event received:");
                    println!("   Type: {}", event.event_type());
                    println!("   ID: {}", event.id());
                    println!("   Payload: {}", serde_json::to_string_pretty(&event.payload)?);
                    println!();
                    Ok(())
                })
            }),
        )
        .await?;

    // Subscribe to order events
    let order_handle = subscriber
        .subscribe(
            vec!["order.*".to_string()], // Wildcard subscription
            Arc::new(|event| {
                Box::pin(async move {
                    println!("📦 Order event received:");
                    println!("   Type: {}", event.event_type());
                    println!("   Payload: {}", serde_json::to_string_pretty(&event.payload)?);
                    println!();
                    Ok(())
                })
            }),
        )
        .await?;

    println!("✅ Subscriptions ready!\n");

    // Wait for subscriptions to be active
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Publish user events
    println!("📤 Publishing events...\n");

    let user_created = Event::new(
        "user.created",
        serde_json::json!({
            "user_id": 123,
            "email": "alice@example.com",
            "name": "Alice"
        }),
    );
    println!("Publishing: user.created");
    subscriber.publish(user_created).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let user_updated = Event::new(
        "user.updated",
        serde_json::json!({
            "user_id": 123,
            "email": "alice.new@example.com",
            "updated_fields": ["email"]
        }),
    );
    println!("Publishing: user.updated");
    subscriber.publish(user_updated).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Publish order events
    let order_created = Event::new(
        "order.created",
        serde_json::json!({
            "order_id": "ORD-001",
            "user_id": 123,
            "total": 99.99
        }),
    );
    println!("Publishing: order.created");
    subscriber.publish(order_created).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let order_completed = Event::new(
        "order.completed",
        serde_json::json!({
            "order_id": "ORD-001",
            "status": "completed"
        }),
    );
    println!("Publishing: order.completed");
    subscriber.publish(order_completed).await?;

    // Wait for messages to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Cleanup
    println!("\n🧹 Cleaning up...");
    subscriber.unsubscribe(&user_handle).await?;
    subscriber.unsubscribe(&order_handle).await?;

    println!("✅ Example completed successfully!");
    println!("\nKey NATS features demonstrated:");
    println!("  • Subject-based pub/sub");
    println!("  • Wildcard subscriptions (order.*)");
    println!("  • Multiple event types per subscription");
    println!("  • Async message handling");

    Ok(())
}
