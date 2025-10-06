//! RabbitMQ integration tests
//!
//! These tests require a running RabbitMQ server on localhost:5672
//! Run with: make services-up && cargo test --test rabbitmq_integration

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::time::{timeout, Duration};
use watchtower_core::prelude::*;
use watchtower_rabbitmq::prelude::*;

#[tokio::test]
#[ignore]
async fn test_rabbitmq_publish_subscribe() {
    let config = RabbitMQConfig {
        url: "amqp://guest:guest@localhost:5672/%2f".to_string(),
        exchange: "test.events".to_string(),
        exchange_type: ExchangeType::Topic,
        queue_prefix: "test".to_string(),
        durable: false,
        persistent: false,
        auto_delete: true,
        dead_letter_exchange: None,
        message_ttl: 0,
        max_priority: 0,
        prefetch_count: 10,
        retry_attempts: 3,
        retry_delay_seconds: 1,
        backpressure: BackpressureConfig::default(),
    };

    let mut subscriber = RabbitMQSubscriber::new(config)
        .await
        .expect("Failed to connect to RabbitMQ");

    let received_count = Arc::new(AtomicU32::new(0));
    let count_clone = received_count.clone();

    let handle = subscriber
        .subscribe(
            vec!["order.created".to_string()],
            Arc::new(move |event| {
                let count = count_clone.clone();
                Box::pin(async move {
                    assert_eq!(event.event_type(), "order.created");
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            }),
        )
        .await
        .expect("Failed to subscribe");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Publish events
    for i in 0..5 {
        let event = Event::new(
            "order.created",
            serde_json::json!({
                "order_id": format!("ORD-{}", i),
                "total": 100.0 * (i as f64 + 1.0)
            }),
        );
        subscriber.publish(event).await.expect("Failed to publish");
    }

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
async fn test_rabbitmq_topic_routing() {
    let config = RabbitMQConfig {
        url: "amqp://guest:guest@localhost:5672/%2f".to_string(),
        exchange: "test.topic".to_string(),
        exchange_type: ExchangeType::Topic,
        queue_prefix: "test-topic".to_string(),
        durable: false,
        persistent: false,
        auto_delete: true,
        dead_letter_exchange: None,
        message_ttl: 0,
        max_priority: 0,
        prefetch_count: 10,
        retry_attempts: 3,
        retry_delay_seconds: 1,
        backpressure: BackpressureConfig::default(),
    };

    let mut subscriber = RabbitMQSubscriber::new(config)
        .await
        .expect("Failed to connect");

    let user_count = Arc::new(AtomicU32::new(0));
    let product_count = Arc::new(AtomicU32::new(0));

    let uc = user_count.clone();
    let pc = product_count.clone();

    // Subscribe to user events
    let user_handle = subscriber
        .subscribe(
            vec!["user.created".to_string(), "user.updated".to_string()],
            Arc::new(move |event| {
                let c = uc.clone();
                Box::pin(async move {
                    assert!(event.event_type().starts_with("user."));
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            }),
        )
        .await
        .expect("Failed to subscribe to user events");

    // Subscribe to product events
    let product_handle = subscriber
        .subscribe(
            vec!["product.created".to_string()],
            Arc::new(move |event| {
                let c = pc.clone();
                Box::pin(async move {
                    assert_eq!(event.event_type(), "product.created");
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            }),
        )
        .await
        .expect("Failed to subscribe to product events");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Publish different event types
    subscriber
        .publish(Event::new("user.created", serde_json::json!({"id": 1})))
        .await
        .expect("Failed to publish");

    subscriber
        .publish(Event::new("user.updated", serde_json::json!({"id": 1})))
        .await
        .expect("Failed to publish");

    subscriber
        .publish(Event::new("product.created", serde_json::json!({"id": 1})))
        .await
        .expect("Failed to publish");

    tokio::time::sleep(Duration::from_secs(2)).await;

    assert_eq!(user_count.load(Ordering::SeqCst), 2);
    assert_eq!(product_count.load(Ordering::SeqCst), 1);

    subscriber.unsubscribe(&user_handle).await.expect("Failed to unsubscribe");
    subscriber.unsubscribe(&product_handle).await.expect("Failed to unsubscribe");
}

#[tokio::test]
#[ignore]
async fn test_rabbitmq_fanout_exchange() {
    let config = RabbitMQConfig {
        url: "amqp://guest:guest@localhost:5672/%2f".to_string(),
        exchange: "test.fanout".to_string(),
        exchange_type: ExchangeType::Fanout,
        queue_prefix: "test-fanout".to_string(),
        durable: false,
        persistent: false,
        auto_delete: true,
        dead_letter_exchange: None,
        message_ttl: 0,
        max_priority: 0,
        prefetch_count: 10,
        retry_attempts: 3,
        retry_delay_seconds: 1,
        backpressure: BackpressureConfig::default(),
    };

    let mut sub1 = RabbitMQSubscriber::new(config.clone())
        .await
        .expect("Failed to connect subscriber 1");

    let mut sub2 = RabbitMQSubscriber::new(config)
        .await
        .expect("Failed to connect subscriber 2");

    let count1 = Arc::new(AtomicU32::new(0));
    let count2 = Arc::new(AtomicU32::new(0));

    let c1 = count1.clone();
    let c2 = count2.clone();

    let handle1 = sub1
        .subscribe(
            vec!["broadcast.message".to_string()],
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
            vec!["broadcast.message".to_string()],
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

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Publish to fanout - all subscribers should receive
    for i in 0..3 {
        let event = Event::new(
            "broadcast.message",
            serde_json::json!({"index": i}),
        );
        sub1.publish(event).await.expect("Failed to publish");
    }

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Both should receive all messages in fanout
    assert_eq!(count1.load(Ordering::SeqCst), 3);
    assert_eq!(count2.load(Ordering::SeqCst), 3);

    sub1.unsubscribe(&handle1).await.expect("Failed to unsubscribe 1");
    sub2.unsubscribe(&handle2).await.expect("Failed to unsubscribe 2");
}
