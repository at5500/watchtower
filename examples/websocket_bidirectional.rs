//! WebSocket bidirectional communication example
//!
//! This example demonstrates WebSocket usage with Watchtower:
//! - Bidirectional real-time communication
//! - Auto-reconnect on connection loss
//! - Text and binary message support
//! - Split read/write streams

use std::sync::Arc;
use watchtower_core::prelude::*;
use watchtower_websocket::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Watchtower WebSocket Bidirectional Example ===\n");

    // Configure WebSocket transport
    let config = WebSocketConfig {
        url: "ws://localhost:8080".to_string(),
        auto_reconnect: true,
        retry_attempts: 5,
        retry_delay_seconds: 3,
        ping_interval_seconds: 30,
        pong_timeout_seconds: 10,
        max_message_size: 64 * 1024 * 1024, // 64MB
        headers: Vec::new(),
        backpressure: BackpressureConfig {
            max_queue_size: 500,
            strategy: BackpressureStrategy::DropOldest,
            warning_threshold: 0.8,
        },
    };

    println!("📡 Connecting to WebSocket server at {}", config.url);
    let mut subscriber = WebSocketSubscriber::new(config).await?;
    println!("✅ Connected!\n");

    // Subscribe to chat events
    println!("🔔 Setting up subscriptions...");
    let chat_handle = subscriber
        .subscribe(
            vec!["chat.message".to_string(), "chat.user_joined".to_string()],
            Arc::new(|event| {
                Box::pin(async move {
                    println!("💬 Chat event received:");
                    println!("   Type: {}", event.event_type());
                    println!("   Payload: {}", serde_json::to_string_pretty(&event.payload)?);
                    println!();
                    Ok(())
                })
            }),
        )
        .await?;

    // Subscribe to system events
    let system_handle = subscriber
        .subscribe(
            vec!["system.notification".to_string()],
            Arc::new(|event| {
                Box::pin(async move {
                    println!("🔔 System notification:");
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

    // Simulate bidirectional communication
    println!("📤 Sending messages (bidirectional)...\n");

    // Send chat message
    let chat1 = Event::new(
        "chat.message",
        serde_json::json!({
            "user": "Alice",
            "message": "Hello, everyone!",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
    );
    println!("Sending: chat.message (Alice)");
    subscriber.publish(chat1).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Send user joined event
    let join_event = Event::new(
        "chat.user_joined",
        serde_json::json!({
            "user": "Bob",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
    );
    println!("Sending: chat.user_joined (Bob)");
    subscriber.publish(join_event).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Send another chat message
    let chat2 = Event::new(
        "chat.message",
        serde_json::json!({
            "user": "Bob",
            "message": "Hi Alice!",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
    );
    println!("Sending: chat.message (Bob)");
    subscriber.publish(chat2).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Send system notification
    let notification = Event::new(
        "system.notification",
        serde_json::json!({
            "type": "info",
            "message": "Server maintenance scheduled for tonight",
            "priority": "normal"
        }),
    );
    println!("Sending: system.notification");
    subscriber.publish(notification).await?;

    // Wait for messages to be echoed back
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test reconnection (optional - commented out)
    // println!("\n🔄 Testing auto-reconnect...");
    // println!("   (Manually stop/restart WebSocket server to see reconnection)");
    // tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // Cleanup
    println!("\n🧹 Cleaning up...");
    subscriber.unsubscribe(&chat_handle).await?;
    subscriber.unsubscribe(&system_handle).await?;

    println!("✅ Example completed successfully!");
    println!("\nKey WebSocket features demonstrated:");
    println!("  • Bidirectional real-time communication");
    println!("  • Split read/write streams");
    println!("  • Text message format (JSON)");
    println!("  • Auto-reconnect on disconnect");
    println!("  • Multiple concurrent subscriptions");
    println!("\nWebSocket server echoes messages back!");
    println!("Try: websocat ws://localhost:8080");

    Ok(())
}
