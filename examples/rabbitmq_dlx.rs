//! RabbitMQ with Dead Letter Exchange example
//!
//! This example demonstrates RabbitMQ usage with Watchtower:
//! - Publishing to exchanges
//! - Topic-based routing
//! - Dead Letter Exchange (DLX) for failed messages
//! - Message persistence and durability

use std::sync::Arc;
use watchtower_core::prelude::*;
use watchtower_rabbitmq::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Watchtower RabbitMQ with DLX Example ===\n");

    // Configure RabbitMQ transport with DLX
    let config = RabbitMQConfig {
        url: "amqp://localhost:5672".to_string(),
        exchange: "watchtower.events".to_string(),
        exchange_type: ExchangeType::Topic,
        queue_prefix: "watchtower".to_string(),
        durable: true,
        persistent: true,
        auto_delete: false,
        dead_letter_exchange: Some("watchtower.dlx".to_string()),
        message_ttl: 0, // No TTL
        max_priority: 10,
        prefetch_count: 10,
        retry_attempts: 3,
        retry_delay_seconds: 5,
        backpressure: BackpressureConfig {
            max_queue_size: 1000,
            strategy: BackpressureStrategy::DropOldest,
            warning_threshold: 0.8,
        },
    };

    println!("📡 Connecting to RabbitMQ at {}", config.url);
    println!("   Exchange: {} ({:?})", config.exchange, config.exchange_type);
    println!("   Queue prefix: {}", config.queue_prefix);
    println!("   DLX: {:?}", config.dead_letter_exchange);
    let mut subscriber = RabbitMQSubscriber::new(config).await?;
    println!("✅ Connected!\n");

    // Subscribe to inventory events
    println!("🔔 Setting up subscriptions...");
    let inventory_handle = subscriber
        .subscribe(
            vec!["inventory.updated".to_string(), "inventory.depleted".to_string()],
            Arc::new(|event| {
                Box::pin(async move {
                    println!("📦 Inventory event received:");
                    println!("   Type: {}", event.event_type());
                    println!("   ID: {}", event.id());
                    println!("   Payload: {}", serde_json::to_string_pretty(&event.payload)?);
                    println!("   ✅ Message acknowledged");
                    println!();
                    Ok(())
                })
            }),
        )
        .await?;

    // Subscribe to shipping events (with simulated failures)
    let shipping_handle = subscriber
        .subscribe(
            vec!["shipping.created".to_string()],
            Arc::new(|event| {
                Box::pin(async move {
                    println!("🚚 Shipping event received:");
                    println!("   Type: {}", event.event_type());

                    // Simulate processing failure for demonstration
                    let payload = &event.payload;
                    if payload.get("simulate_failure").and_then(|v| v.as_bool()).unwrap_or(false) {
                        println!("   ❌ Simulated processing failure!");
                        println!("   → Message will be sent to DLX");
                        return Err(WatchtowerError::PublicationError(
                            "Simulated failure".to_string()
                        ));
                    }

                    println!("   Payload: {}", serde_json::to_string_pretty(&event.payload)?);
                    println!("   ✅ Message acknowledged");
                    println!();
                    Ok(())
                })
            }),
        )
        .await?;

    println!("✅ Subscriptions ready!\n");

    // Wait for subscriptions to be active
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Publish events to RabbitMQ
    println!("📤 Publishing events...\n");

    let inventory1 = Event::new(
        "inventory.updated",
        serde_json::json!({
            "product_id": "PROD-001",
            "quantity": 50,
            "warehouse": "WH-A"
        }),
    );
    println!("Publishing: inventory.updated");
    subscriber.publish(inventory1).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let inventory2 = Event::new(
        "inventory.depleted",
        serde_json::json!({
            "product_id": "PROD-002",
            "quantity": 0,
            "warehouse": "WH-B"
        }),
    );
    println!("Publishing: inventory.depleted");
    subscriber.publish(inventory2).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Publish shipping event that will succeed
    let shipping_ok = Event::new(
        "shipping.created",
        serde_json::json!({
            "shipping_id": "SHIP-001",
            "order_id": "ORD-123",
            "carrier": "UPS",
            "tracking_number": "1Z999AA10123456784"
        }),
    );
    println!("Publishing: shipping.created (will succeed)");
    subscriber.publish(shipping_ok).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Publish shipping event that will fail (goes to DLX)
    let shipping_fail = Event::new(
        "shipping.created",
        serde_json::json!({
            "shipping_id": "SHIP-002",
            "order_id": "ORD-124",
            "carrier": "FedEx",
            "simulate_failure": true
        }),
    );
    println!("Publishing: shipping.created (will fail → DLX)");
    subscriber.publish(shipping_fail).await?;

    // Wait for messages to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Cleanup
    println!("\n🧹 Cleaning up...");
    subscriber.unsubscribe(&inventory_handle).await?;
    subscriber.unsubscribe(&shipping_handle).await?;

    println!("✅ Example completed successfully!");
    println!("\nKey RabbitMQ features demonstrated:");
    println!("  • Topic exchange with routing patterns");
    println!("  • Durable queues and persistent messages");
    println!("  • Message acknowledgment (ACK/NACK)");
    println!("  • Dead Letter Exchange for failed messages");
    println!("  • Publisher confirms");
    println!("\nRabbitMQ Management UI:");
    println!("  http://localhost:15672 (guest/guest)");
    println!("\nCheck DLX queue:");
    println!("  rabbitmqctl list_queues name messages");
    println!("  # Look for watchtower.dlx related queues");

    Ok(())
}
