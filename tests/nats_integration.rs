//! NATS integration tests
//!
//! These tests require a running NATS server on localhost:4222
//! Run with: make services-up && cargo test --test nats_integration

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::time::{timeout, Duration};
use watchtower_core::prelude::*;
use watchtower_nats::prelude::*;

#[tokio::test]
#[ignore] // Run with --ignored flag when NATS is available
async fn test_nats_publish_subscribe() {
    let config = NatsConfig {
        url: "nats://localhost:4222".to_string(),
        max_reconnect_attempts: 3,
        reconnect_delay_seconds: 1,
        backpressure: BackpressureConfig::default(),
    };

    let mut subscriber = NatsSubscriber::new(config)
        .await
        .expect("Failed to connect to NATS");

    let received_count = Arc::new(AtomicU32::new(0));
    let count_clone = received_count.clone();

    // Subscribe to test events
    let handle = subscriber
        .subscribe(
            vec!["test.event".to_string()],
            Arc::new(move |event| {
                let count = count_clone.clone();
                Box::pin(async move {
                    assert_eq!(event.event_type(), "test.event");
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            }),
        )
        .await
        .expect("Failed to subscribe");

    // Wait for subscription to be active
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Publish test events
    for i in 0..5 {
        let event = Event::new(
            "test.event",
            serde_json::json!({
                "id": i,
                "message": format!("Test message {}", i)
            }),
        );
        subscriber.publish(event).await.expect("Failed to publish");
    }

    // Wait for events to be received
    let result = timeout(Duration::from_secs(3), async {
        while received_count.load(Ordering::SeqCst) < 5 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    assert!(result.is_ok(), "Not all events were received in time");
    assert_eq!(received_count.load(Ordering::SeqCst), 5);

    // Cleanup
    subscriber.unsubscribe(&handle).await.expect("Failed to unsubscribe");
}

#[tokio::test]
#[ignore]
async fn test_nats_wildcard_subscription() {
    let config = NatsConfig::default();
    let mut subscriber = NatsSubscriber::new(config)
        .await
        .expect("Failed to connect to NATS");

    let received_types = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let types_clone = received_types.clone();

    // Subscribe with wildcard
    let handle = subscriber
        .subscribe(
            vec!["user.*".to_string()],
            Arc::new(move |event| {
                let types = types_clone.clone();
                Box::pin(async move {
                    types.lock().await.push(event.event_type().to_string());
                    Ok(())
                })
            }),
        )
        .await
        .expect("Failed to subscribe");

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Publish different event types
    for event_type in ["user.created", "user.updated", "user.deleted"] {
        let event = Event::new(event_type, serde_json::json!({"test": true}));
        subscriber.publish(event).await.expect("Failed to publish");
    }

    // Wait for events
    tokio::time::sleep(Duration::from_secs(1)).await;

    let types = received_types.lock().await;
    assert_eq!(types.len(), 3);
    assert!(types.contains(&"user.created".to_string()));
    assert!(types.contains(&"user.updated".to_string()));
    assert!(types.contains(&"user.deleted".to_string()));

    subscriber.unsubscribe(&handle).await.expect("Failed to unsubscribe");
}

#[tokio::test]
#[ignore]
async fn test_nats_multiple_subscribers() {
    let config = NatsConfig::default();

    let mut sub1 = NatsSubscriber::new(config.clone())
        .await
        .expect("Failed to connect subscriber 1");

    let mut sub2 = NatsSubscriber::new(config)
        .await
        .expect("Failed to connect subscriber 2");

    let count1 = Arc::new(AtomicU32::new(0));
    let count2 = Arc::new(AtomicU32::new(0));

    let c1 = count1.clone();
    let c2 = count2.clone();

    // Both subscribe to same event type
    let handle1 = sub1
        .subscribe(
            vec!["broadcast.event".to_string()],
            Arc::new(move |_| {
                let c = c1.clone();
                Box::pin(async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            }),
        )
        .await
        .expect("Failed to subscribe 1");

    let handle2 = sub2
        .subscribe(
            vec!["broadcast.event".to_string()],
            Arc::new(move |_| {
                let c = c2.clone();
                Box::pin(async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            }),
        )
        .await
        .expect("Failed to subscribe 2");

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Publish events
    for i in 0..3 {
        let event = Event::new(
            "broadcast.event",
            serde_json::json!({"index": i}),
        );
        sub1.publish(event).await.expect("Failed to publish");
    }

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Both subscribers should receive all events
    assert_eq!(count1.load(Ordering::SeqCst), 3);
    assert_eq!(count2.load(Ordering::SeqCst), 3);

    sub1.unsubscribe(&handle1).await.expect("Failed to unsubscribe 1");
    sub2.unsubscribe(&handle2).await.expect("Failed to unsubscribe 2");
}
