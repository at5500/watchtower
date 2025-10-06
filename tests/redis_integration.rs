//! Redis Streams integration tests
//!
//! These tests require a running Redis server on localhost:6379
//! Run with: make services-up && cargo test --test redis_integration

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::time::{timeout, Duration};
use watchtower_core::prelude::*;
use watchtower_redis::prelude::*;

#[tokio::test]
#[ignore]
async fn test_redis_publish_subscribe() {
    let config = RedisConfig {
        url: "redis://localhost:6379".to_string(),
        stream_prefix: "test:events".to_string(),
        consumer_group: "test-group".to_string(),
        consumer_name: "test-consumer-1".to_string(),
        max_stream_length: 1000,
        backpressure: BackpressureConfig::default(),
    };

    let mut subscriber = RedisSubscriber::new(config)
        .await
        .expect("Failed to connect to Redis");

    let received_count = Arc::new(AtomicU32::new(0));
    let count_clone = received_count.clone();

    let handle = subscriber
        .subscribe(
            vec!["payment.processed".to_string()],
            Arc::new(move |event| {
                let count = count_clone.clone();
                Box::pin(async move {
                    assert_eq!(event.event_type(), "payment.processed");
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            }),
        )
        .await
        .expect("Failed to subscribe");

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Publish events
    for i in 0..5 {
        let event = Event::new(
            "payment.processed",
            serde_json::json!({
                "payment_id": format!("PAY-{}", i),
                "amount": 100.0 * (i as f64 + 1.0)
            }),
        );
        subscriber.publish(event).await.expect("Failed to publish");
    }

    // Wait for events
    let result = timeout(Duration::from_secs(5), async {
        while received_count.load(Ordering::SeqCst) < 5 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    assert!(result.is_ok(), "Not all events were received");
    assert_eq!(received_count.load(Ordering::SeqCst), 5);

    subscriber.unsubscribe(&handle).await.expect("Failed to unsubscribe");
}

#[tokio::test]
#[ignore]
async fn test_redis_consumer_groups() {
    let config1 = RedisConfig {
        url: "redis://localhost:6379".to_string(),
        stream_prefix: "test:cg".to_string(),
        consumer_group: "cg-test".to_string(),
        consumer_name: "consumer-1".to_string(),
        max_stream_length: 1000,
        backpressure: BackpressureConfig::default(),
    };

    let config2 = RedisConfig {
        consumer_name: "consumer-2".to_string(),
        ..config1.clone()
    };

    let mut sub1 = RedisSubscriber::new(config1)
        .await
        .expect("Failed to connect consumer 1");

    let mut sub2 = RedisSubscriber::new(config2)
        .await
        .expect("Failed to connect consumer 2");

    let count1 = Arc::new(AtomicU32::new(0));
    let count2 = Arc::new(AtomicU32::new(0));

    let c1 = count1.clone();
    let c2 = count2.clone();

    let handle1 = sub1
        .subscribe(
            vec!["cg.event".to_string()],
            Arc::new(move |_| {
                let c = c1.clone();
                Box::pin(async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    Ok(())
                })
            }),
        )
        .await
        .expect("Failed to subscribe 1");

    let handle2 = sub2
        .subscribe(
            vec!["cg.event".to_string()],
            Arc::new(move |_| {
                let c = c2.clone();
                Box::pin(async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    Ok(())
                })
            }),
        )
        .await
        .expect("Failed to subscribe 2");

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Publish 10 events - should be distributed between consumers
    for i in 0..10 {
        let event = Event::new("cg.event", serde_json::json!({"id": i}));
        sub1.publish(event).await.expect("Failed to publish");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    tokio::time::sleep(Duration::from_secs(2)).await;

    let total = count1.load(Ordering::SeqCst) + count2.load(Ordering::SeqCst);
    assert_eq!(total, 10, "Total events should be 10");

    // Both consumers should have received some events (load balancing)
    assert!(count1.load(Ordering::SeqCst) > 0, "Consumer 1 should receive events");
    assert!(count2.load(Ordering::SeqCst) > 0, "Consumer 2 should receive events");

    sub1.unsubscribe(&handle1).await.expect("Failed to unsubscribe 1");
    sub2.unsubscribe(&handle2).await.expect("Failed to unsubscribe 2");
}

#[tokio::test]
#[ignore]
async fn test_redis_stream_persistence() {
    let config = RedisConfig {
        url: "redis://localhost:6379".to_string(),
        stream_prefix: "test:persist".to_string(),
        consumer_group: "persist-group".to_string(),
        consumer_name: "persist-consumer".to_string(),
        max_stream_length: 100,
        backpressure: BackpressureConfig::default(),
    };

    let subscriber = RedisSubscriber::new(config.clone())
        .await
        .expect("Failed to connect");

    // Publish events
    for i in 0..5 {
        let event = Event::new(
            "persist.test",
            serde_json::json!({"id": i}),
        );
        subscriber.publish(event).await.expect("Failed to publish");
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Create new subscriber (simulating restart)
    let mut new_subscriber = RedisSubscriber::new(config)
        .await
        .expect("Failed to reconnect");

    let received = Arc::new(AtomicU32::new(0));
    let r = received.clone();

    let handle = new_subscriber
        .subscribe(
            vec!["persist.test".to_string()],
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

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should receive persisted events
    assert!(
        received.load(Ordering::SeqCst) > 0,
        "Should receive persisted events"
    );

    new_subscriber.unsubscribe(&handle).await.expect("Failed to unsubscribe");
}
