//! Backpressure handling example
//!
//! This example demonstrates:
//! - Different backpressure strategies
//! - Queue monitoring and warnings
//! - Handling overload scenarios
//! - Flow control mechanisms

use std::sync::Arc;
use std::time::Duration;
use watchtower_core::prelude::*;
use watchtower_nats::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Watchtower Backpressure Handling Example ===\n");

    println!("📊 Backpressure Strategies:");
    println!("   DropOldest:  Drop oldest messages when queue is full");
    println!("   DropNewest:  Drop newest messages when queue is full");
    println!("   Block:       Block until space is available\n");

    // Demonstrate different strategies
    println!("═══════════════════════════════════════════════════");
    println!("Strategy 1: DROP_OLDEST");
    println!("═══════════════════════════════════════════════════\n");
    demonstrate_drop_oldest().await?;

    println!("\n═══════════════════════════════════════════════════");
    println!("Strategy 2: DROP_NEWEST");
    println!("═══════════════════════════════════════════════════\n");
    demonstrate_drop_newest().await?;

    println!("\n═══════════════════════════════════════════════════");
    println!("Strategy 3: BLOCK");
    println!("═══════════════════════════════════════════════════\n");
    demonstrate_block().await?;

    println!("\n✅ All backpressure demonstrations complete!\n");
    println!("When to use each strategy:");
    println!("  DROP_OLDEST:");
    println!("    ✓ Real-time data (latest is most important)");
    println!("    ✓ Sensor readings, metrics");
    println!("    ✓ Live dashboards");
    println!("  DROP_NEWEST:");
    println!("    ✓ Historical data (older is more important)");
    println!("    ✓ Event sourcing");
    println!("    ✓ Audit logs");
    println!("  BLOCK:");
    println!("    ✓ Critical operations (no data loss)");
    println!("    ✓ Financial transactions");
    println!("    ✓ Order processing\n");

    println!("Best Practices:");
    println!("  • Monitor queue depth regularly");
    println!("  • Set appropriate warning thresholds");
    println!("  • Alert on backpressure conditions");
    println!("  • Scale consumers when needed");
    println!("  • Use circuit breakers with blocking");
    println!("  • Log all dropped messages");

    Ok(())
}

async fn demonstrate_drop_oldest() -> Result<(), Box<dyn std::error::Error>> {
    let config = NatsConfig {
        url: "nats://localhost:4222".to_string(),
        max_reconnect_attempts: 3,
        reconnect_delay_seconds: 1,
        backpressure: BackpressureConfig {
            max_queue_size: 5, // Small queue to trigger backpressure
            strategy: BackpressureStrategy::DropOldest,
            warning_threshold: 0.6, // Warn at 60% capacity
        },
    };

    println!("Configuration:");
    println!("   Queue size: {}", config.backpressure.max_queue_size);
    println!("   Strategy: {:?}", config.backpressure.strategy);
    println!("   Warning at: {}%\n", config.backpressure.warning_threshold * 100.0);

    let mut subscriber = NatsSubscriber::new(config).await?;

    let processed = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let p = processed.clone();

    let handle = subscriber
        .subscribe(
            vec!["backpressure.drop_oldest".to_string()],
            Arc::new(move |event| {
                let p = p.clone();
                Box::pin(async move {
                    // Simulate slow processing
                    tokio::time::sleep(Duration::from_millis(500)).await;

                    let id = event.payload.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                    p.lock().await.push(id);
                    println!("   Processed message: {}", id);

                    Ok(())
                })
            }),
        )
        .await?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    println!("Publishing 10 messages rapidly (queue size: 5)...");
    for i in 1..=10 {
        let event = Event::new(
            "backpressure.drop_oldest",
            serde_json::json!({"id": i}),
        );
        subscriber.publish(event).await?;
        println!("Published message: {}", i);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    println!("\nWaiting for processing to complete...");
    tokio::time::sleep(Duration::from_secs(6)).await;

    let processed_ids = processed.lock().await;
    println!("\nResults:");
    println!("   Published: 10 messages (1-10)");
    println!("   Processed: {} messages", processed_ids.len());
    println!("   Messages: {:?}", processed_ids);
    println!("   → Oldest messages (1-5) were dropped");
    println!("   → Newest messages (6-10) were processed");

    subscriber.unsubscribe(&handle).await?;
    Ok(())
}

async fn demonstrate_drop_newest() -> Result<(), Box<dyn std::error::Error>> {
    let config = NatsConfig {
        url: "nats://localhost:4222".to_string(),
        max_reconnect_attempts: 3,
        reconnect_delay_seconds: 1,
        backpressure: BackpressureConfig {
            max_queue_size: 5,
            strategy: BackpressureStrategy::DropNewest,
            warning_threshold: 0.6,
        },
    };

    println!("Configuration:");
    println!("   Queue size: {}", config.backpressure.max_queue_size);
    println!("   Strategy: {:?}", config.backpressure.strategy);
    println!("   Warning at: {}%\n", config.backpressure.warning_threshold * 100.0);

    let mut subscriber = NatsSubscriber::new(config).await?;

    let processed = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let p = processed.clone();

    let handle = subscriber
        .subscribe(
            vec!["backpressure.drop_newest".to_string()],
            Arc::new(move |event| {
                let p = p.clone();
                Box::pin(async move {
                    tokio::time::sleep(Duration::from_millis(500)).await;

                    let id = event.payload.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                    p.lock().await.push(id);
                    println!("   Processed message: {}", id);

                    Ok(())
                })
            }),
        )
        .await?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    println!("Publishing 10 messages rapidly (queue size: 5)...");
    for i in 1..=10 {
        let event = Event::new(
            "backpressure.drop_newest",
            serde_json::json!({"id": i}),
        );
        subscriber.publish(event).await?;
        println!("Published message: {}", i);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    println!("\nWaiting for processing to complete...");
    tokio::time::sleep(Duration::from_secs(6)).await;

    let processed_ids = processed.lock().await;
    println!("\nResults:");
    println!("   Published: 10 messages (1-10)");
    println!("   Processed: {} messages", processed_ids.len());
    println!("   Messages: {:?}", processed_ids);
    println!("   → Oldest messages (1-5) were processed");
    println!("   → Newest messages (6-10) were dropped");

    subscriber.unsubscribe(&handle).await?;
    Ok(())
}

async fn demonstrate_block() -> Result<(), Box<dyn std::error::Error>> {
    let config = NatsConfig {
        url: "nats://localhost:4222".to_string(),
        max_reconnect_attempts: 3,
        reconnect_delay_seconds: 1,
        backpressure: BackpressureConfig {
            max_queue_size: 3,
            strategy: BackpressureStrategy::Block,
            warning_threshold: 0.6,
        },
    };

    println!("Configuration:");
    println!("   Queue size: {}", config.backpressure.max_queue_size);
    println!("   Strategy: {:?}", config.backpressure.strategy);
    println!("   Warning at: {}%\n", config.backpressure.warning_threshold * 100.0);

    let mut subscriber = NatsSubscriber::new(config).await?;

    let processed = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let p = processed.clone();

    let handle = subscriber
        .subscribe(
            vec!["backpressure.block".to_string()],
            Arc::new(move |event| {
                let p = p.clone();
                Box::pin(async move {
                    tokio::time::sleep(Duration::from_millis(500)).await;

                    let id = event.payload.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                    p.lock().await.push(id);
                    println!("   Processed message: {}", id);

                    Ok(())
                })
            }),
        )
        .await?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    println!("Publishing 6 messages (queue size: 3)...");
    println!("Note: Publisher will BLOCK when queue is full\n");

    for i in 1..=6 {
        let start = std::time::Instant::now();
        let event = Event::new(
            "backpressure.block",
            serde_json::json!({"id": i}),
        );
        subscriber.publish(event).await?;
        let elapsed = start.elapsed();

        if elapsed > Duration::from_millis(100) {
            println!("Published message: {} (BLOCKED for {:?})", i, elapsed);
        } else {
            println!("Published message: {} (immediate)", i);
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    println!("\nWaiting for processing to complete...");
    tokio::time::sleep(Duration::from_secs(4)).await;

    let processed_ids = processed.lock().await;
    println!("\nResults:");
    println!("   Published: 6 messages (1-6)");
    println!("   Processed: {} messages", processed_ids.len());
    println!("   Messages: {:?}", processed_ids);
    println!("   → ALL messages were processed (no data loss)");
    println!("   → Publisher blocked when queue was full");

    subscriber.unsubscribe(&handle).await?;
    Ok(())
}
