//! WebSocket integration tests
//!
//! These tests require a running WebSocket server on localhost:8080
//! Run with: make services-up && cargo test --test websocket_integration

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::time::{timeout, Duration};
use watchtower_core::prelude::*;
use watchtower_websocket::prelude::*;

#[tokio::test]
#[ignore]
async fn test_websocket_bidirectional() {
    let config = WebSocketConfig {
        url: "ws://localhost:8080".to_string(),
        auto_reconnect: true,
        retry_attempts: 3,
        retry_delay_seconds: 2,
        ping_interval_seconds: 30,
        pong_timeout_seconds: 10,
        max_message_size: 64 * 1024 * 1024,
        headers: Vec::new(),
        backpressure: BackpressureConfig::default(),
    };

    let mut subscriber = WebSocketSubscriber::new(config)
        .await
        .expect("Failed to connect to WebSocket");

    let received_count = Arc::new(AtomicU32::new(0));
    let count_clone = received_count.clone();

    let handle = subscriber
        .subscribe(
            vec!["chat.message".to_string()],
            Arc::new(move |event| {
                let count = count_clone.clone();
                Box::pin(async move {
                    assert_eq!(event.event_type(), "chat.message");
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            }),
        )
        .await
        .expect("Failed to subscribe");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Publish events (WebSocket server should echo them back)
    for i in 0..5 {
        let event = Event::new(
            "chat.message",
            serde_json::json!({
                "user": "test-user",
                "message": format!("Message {}", i)
            }),
        );
        subscriber.publish(event).await.expect("Failed to publish");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let result = timeout(Duration::from_secs(5), async {
        while received_count.load(Ordering::SeqCst) < 5 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    assert!(result.is_ok(), "Not all events were echoed back");
    assert_eq!(received_count.load(Ordering::SeqCst), 5);

    subscriber.unsubscribe(&handle).await.expect("Failed to unsubscribe");
}

#[tokio::test]
#[ignore]
async fn test_websocket_multiple_subscriptions() {
    let config = WebSocketConfig::default();
    let mut subscriber = WebSocketSubscriber::new(config)
        .await
        .expect("Failed to connect");

    let chat_count = Arc::new(AtomicU32::new(0));
    let system_count = Arc::new(AtomicU32::new(0));

    let cc = chat_count.clone();
    let sc = system_count.clone();

    let chat_handle = subscriber
        .subscribe(
            vec!["chat.message".to_string()],
            Arc::new(move |event| {
                let c = cc.clone();
                Box::pin(async move {
                    assert_eq!(event.event_type(), "chat.message");
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            }),
        )
        .await
        .expect("Failed to subscribe to chat");

    let system_handle = subscriber
        .subscribe(
            vec!["system.notification".to_string()],
            Arc::new(move |event| {
                let c = sc.clone();
                Box::pin(async move {
                    assert_eq!(event.event_type(), "system.notification");
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            }),
        )
        .await
        .expect("Failed to subscribe to system");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Publish different event types
    subscriber
        .publish(Event::new("chat.message", serde_json::json!({"msg": "hello"})))
        .await
        .expect("Failed to publish");

    tokio::time::sleep(Duration::from_millis(200)).await;

    subscriber
        .publish(Event::new("system.notification", serde_json::json!({"type": "info"})))
        .await
        .expect("Failed to publish");

    tokio::time::sleep(Duration::from_secs(2)).await;

    assert_eq!(chat_count.load(Ordering::SeqCst), 1);
    assert_eq!(system_count.load(Ordering::SeqCst), 1);

    subscriber.unsubscribe(&chat_handle).await.expect("Failed to unsubscribe");
    subscriber.unsubscribe(&system_handle).await.expect("Failed to unsubscribe");
}

#[tokio::test]
#[ignore]
async fn test_websocket_reconnect() {
    let config = WebSocketConfig {
        url: "ws://localhost:8080".to_string(),
        auto_reconnect: true,
        retry_attempts: 5,
        retry_delay_seconds: 1,
        ping_interval_seconds: 5,
        pong_timeout_seconds: 3,
        max_message_size: 64 * 1024 * 1024,
        headers: Vec::new(),
        backpressure: BackpressureConfig::default(),
    };

    let mut subscriber = WebSocketSubscriber::new(config)
        .await
        .expect("Failed to connect");

    let received = Arc::new(AtomicU32::new(0));
    let r = received.clone();

    let handle = subscriber
        .subscribe(
            vec!["reconnect.test".to_string()],
            Arc::new(move |_| {
                let r = r.clone();
                Box::pin(async move {
                    r.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            }),
        )
        .await
        .expect("Failed to subscribe");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Send event before reconnection
    subscriber
        .publish(Event::new("reconnect.test", serde_json::json!({"before": true})))
        .await
        .expect("Failed to publish");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Note: To fully test reconnection, you would need to:
    // 1. Kill the WebSocket server
    // 2. Wait for reconnection attempt
    // 3. Restart the server
    // 4. Verify connection is restored
    // This is simplified for automated testing

    assert!(received.load(Ordering::SeqCst) > 0, "Should receive events");

    subscriber.unsubscribe(&handle).await.expect("Failed to unsubscribe");
}
