//! Example demonstrating the use of derive macros for automatic event generation
//!
//! This example shows how to use #[derive(Event)] to automatically convert
//! structs into Watchtower events.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use watchtower_core::prelude::*;
use watchtower_derive::Event;
use watchtower_nats::prelude::*;

/// User registration event
#[derive(Event, Serialize, Deserialize, Debug, Clone)]
#[event(type = "user.created", source = "user-service")]
struct UserCreated {
    user_id: u64,
    email: String,
    username: String,
    created_at: String,
}

/// Order placement event
#[derive(Event, Serialize, Deserialize, Debug, Clone)]
#[event(type = "order.placed")]
struct OrderPlaced {
    order_id: String,
    user_id: u64,
    total_amount: f64,
    items_count: u32,
}

/// Payment processed event
#[derive(Event, Serialize, Deserialize, Debug, Clone)]
#[event(type = "payment.processed", source = "payment-service")]
struct PaymentProcessed {
    payment_id: String,
    order_id: String,
    amount: f64,
    currency: String,
    status: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Watchtower Derive Macro Example ===\n");

    // Configure NATS transport
    let config = NatsConfig::default();
    let mut subscriber = NatsSubscriber::new(config).await?;

    // Subscribe to user events
    let user_handle = subscriber
        .subscribe(
            vec!["user.created".to_string()],
            Arc::new(|event| {
                Box::pin(async move {
                    println!("📥 Received user event:");
                    println!("   Type: {}", event.event_type());
                    println!("   Payload: {}", serde_json::to_string_pretty(&event.payload)?);
                    println!("   Source: {:?}", event.metadata.source);
                    println!();
                    Ok(())
                })
            }),
        )
        .await?;

    // Subscribe to order events
    let order_handle = subscriber
        .subscribe(
            vec!["order.placed".to_string()],
            Arc::new(|event| {
                Box::pin(async move {
                    println!("📦 Received order event:");
                    println!("   Type: {}", event.event_type());
                    println!("   Payload: {}", serde_json::to_string_pretty(&event.payload)?);
                    println!();
                    Ok(())
                })
            }),
        )
        .await?;

    // Subscribe to payment events
    let payment_handle = subscriber
        .subscribe(
            vec!["payment.processed".to_string()],
            Arc::new(|event| {
                Box::pin(async move {
                    println!("💳 Received payment event:");
                    println!("   Type: {}", event.event_type());
                    println!("   Payload: {}", serde_json::to_string_pretty(&event.payload)?);
                    println!("   Source: {:?}", event.metadata.source);
                    println!();
                    Ok(())
                })
            }),
        )
        .await?;

    // Wait a bit for subscriptions to be ready
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    println!("Publishing events using derive macros...\n");

    // Create and publish user event using derive macro
    let user_event = UserCreated {
        user_id: 12345,
        email: "john.doe@example.com".to_string(),
        username: "johndoe".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    println!("✉️  Publishing UserCreated event...");
    println!("   Event type: {}", UserCreated::event_type());
    subscriber.publish(user_event.to_event()).await?;

    // Create and publish order event
    let order_event = OrderPlaced {
        order_id: "ORD-2024-001".to_string(),
        user_id: 12345,
        total_amount: 299.99,
        items_count: 3,
    };

    println!("✉️  Publishing OrderPlaced event...");
    println!("   Event type: {}", OrderPlaced::event_type());
    subscriber.publish(order_event.into()).await?; // Can use .into() or .to_event()

    // Create and publish payment event
    let payment_event = PaymentProcessed {
        payment_id: "PAY-2024-001".to_string(),
        order_id: "ORD-2024-001".to_string(),
        amount: 299.99,
        currency: "USD".to_string(),
        status: "completed".to_string(),
    };

    println!("✉️  Publishing PaymentProcessed event...");
    println!("   Event type: {}", PaymentProcessed::event_type());
    subscriber.publish(payment_event.to_event()).await?;

    // Wait for messages to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Cleanup
    subscriber.unsubscribe(&user_handle).await?;
    subscriber.unsubscribe(&order_handle).await?;
    subscriber.unsubscribe(&payment_handle).await?;

    println!("\n✅ Derive macro example completed!");
    println!("\nKey benefits of using #[derive(Event)]:");
    println!("  • Automatic Event conversion with .into() or .to_event()");
    println!("  • Type-safe event creation from structs");
    println!("  • Compile-time event type checking");
    println!("  • Less boilerplate code");
    println!("  • Source tracking for events");

    Ok(())
}
