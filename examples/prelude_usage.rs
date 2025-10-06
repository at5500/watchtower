//! Example demonstrating the use of prelude modules
//!
//! This example shows how to use the prelude module for convenient imports.

// Instead of importing each type individually:
// use watchtower_core::{Event, Subscriber, WatchtowerError};
// use watchtower_nats::{NatsConfig, NatsSubscriber};

// You can use the prelude:
use std::sync::Arc;
use watchtower_core::prelude::*;
use watchtower_nats::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // All types are now available without explicit imports

    // Configure NATS transport (you can also use NatsConfig::default())
    let config = NatsConfig {
        url: "nats://localhost:4222".to_string(),
        max_reconnect_attempts: 10,
        reconnect_delay_seconds: 5,
        backpressure: BackpressureConfig {
            max_queue_size: 1000,
            strategy: BackpressureStrategy::DropOldest,
            warning_threshold: 0.8,
        },
    };

    // Create subscriber
    let mut subscriber = NatsSubscriber::new(config).await?;

    // Subscribe to events
    let handle = subscriber
        .subscribe(
            vec!["example.event".to_string()],
            Arc::new(|event| {
                Box::pin(async move {
                    println!("Received event: {:?}", event);
                    Ok(())
                })
            }),
        )
        .await?;

    // Create and publish an event
    let event = Event::new(
        "example.event",
        serde_json::json!({
            "message": "Hello from prelude example!"
        }),
    );

    subscriber.publish(event).await?;

    // Wait a bit for the message to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Cleanup
    subscriber.unsubscribe(&handle).await?;

    println!("Prelude example completed!");

    Ok(())
}
