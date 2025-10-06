//! Redis Streams example
//!
//! This example demonstrates Redis Streams usage with Watchtower:
//! - Consumer groups for load balancing
//! - Stream persistence
//! - ACK/NACK message acknowledgment
//! - Stream trimming

use std::sync::Arc;
use watchtower_core::prelude::*;
use watchtower_redis::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Watchtower Redis Streams Example ===\n");

    // Configure Redis transport with consumer group
    let config = RedisConfig {
        url: "redis://localhost:6379".to_string(),
        stream_prefix: "events".to_string(),
        consumer_group: "watchtower-consumers".to_string(),
        consumer_name: "consumer-1".to_string(),
        max_stream_length: 1000, // Keep last 1000 messages
        backpressure: BackpressureConfig {
            max_queue_size: 500,
            strategy: BackpressureStrategy::DropOldest,
            warning_threshold: 0.8,
        },
    };

    println!("📡 Connecting to Redis at {}", config.url);
    println!("   Consumer Group: {}", config.consumer_group);
    println!("   Consumer Name: {}", config.consumer_name);
    let mut subscriber = RedisSubscriber::new(config).await?;
    println!("✅ Connected!\n");

    // Subscribe to payment events
    println!("🔔 Setting up subscriptions...");
    let payment_handle = subscriber
        .subscribe(
            vec!["payment.processed".to_string(), "payment.failed".to_string()],
            Arc::new(|event| {
                Box::pin(async move {
                    println!("💳 Payment event received:");
                    println!("   Type: {}", event.event_type());
                    println!("   ID: {}", event.id());
                    println!("   Payload: {}", serde_json::to_string_pretty(&event.payload)?);
                    println!("   ✅ Message acknowledged (ACK)");
                    println!();
                    Ok(())
                })
            }),
        )
        .await?;

    // Subscribe to notification events
    let notification_handle = subscriber
        .subscribe(
            vec!["notification.sent".to_string()],
            Arc::new(|event| {
                Box::pin(async move {
                    println!("📧 Notification event received:");
                    println!("   Type: {}", event.event_type());
                    println!("   Payload: {}", serde_json::to_string_pretty(&event.payload)?);
                    println!("   ✅ Message acknowledged (ACK)");
                    println!();
                    Ok(())
                })
            }),
        )
        .await?;

    println!("✅ Subscriptions ready!\n");

    // Wait for subscriptions to be active
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Publish events to Redis Streams
    println!("📤 Publishing events...\n");

    let payment1 = Event::new(
        "payment.processed",
        serde_json::json!({
            "payment_id": "PAY-001",
            "order_id": "ORD-123",
            "amount": 149.99,
            "currency": "USD",
            "status": "completed"
        }),
    );
    println!("Publishing: payment.processed (stream: events:payment.processed)");
    subscriber.publish(payment1).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let payment2 = Event::new(
        "payment.failed",
        serde_json::json!({
            "payment_id": "PAY-002",
            "order_id": "ORD-124",
            "amount": 99.99,
            "currency": "USD",
            "error": "insufficient_funds"
        }),
    );
    println!("Publishing: payment.failed (stream: events:payment.failed)");
    subscriber.publish(payment2).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let notification = Event::new(
        "notification.sent",
        serde_json::json!({
            "notification_id": "NOT-001",
            "user_id": 123,
            "channel": "email",
            "subject": "Payment Confirmed"
        }),
    );
    println!("Publishing: notification.sent (stream: events:notification.sent)");
    subscriber.publish(notification).await?;

    // Wait for messages to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Cleanup
    println!("\n🧹 Cleaning up...");
    subscriber.unsubscribe(&payment_handle).await?;
    subscriber.unsubscribe(&notification_handle).await?;

    println!("✅ Example completed successfully!");
    println!("\nKey Redis Streams features demonstrated:");
    println!("  • Consumer groups for message distribution");
    println!("  • Persistent message storage");
    println!("  • Automatic ACK on successful processing");
    println!("  • Stream naming: {{stream_prefix}}:{{event_type}}");
    println!("  • Stream trimming (max 1000 messages)");
    println!("\nStream management commands:");
    println!("  redis-cli XLEN events:payment.processed");
    println!("  redis-cli XINFO GROUPS events:payment.processed");
    println!("  redis-cli XPENDING events:payment.processed watchtower-consumers");

    Ok(())
}
