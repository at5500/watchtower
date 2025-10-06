//! Webhook integration tests
//!
//! These tests require a running webhook receiver on localhost:3000
//! Run with: make services-up && cargo test --test webhook_integration

use tokio::time::Duration;
use watchtower_core::prelude::*;
use watchtower_webhook::prelude::*;
use watchtower_webhook::subscriber::WebhookEndpoint;

#[tokio::test]
#[ignore]
async fn test_webhook_delivery() {
    let config = WebhookConfig {
        retry_attempts: 3,
        timeout_seconds: 10,
        verify_ssl: true,
        backpressure: BackpressureConfig::default(),
    };

    let mut subscriber = WebhookSubscriber::new(config)
        .expect("Failed to create webhook subscriber");

    let endpoint = WebhookEndpoint {
        url: "http://localhost:3000/webhooks/test".to_string(),
        secret: Some("test-secret-key".to_string()),
        event_types: vec!["test.event".to_string()],
    };

    subscriber
        .register_endpoint("test", endpoint)
        .await
        .expect("Failed to register endpoint");

    // Publish event
    let event = Event::new(
        "test.event",
        serde_json::json!({
            "test_id": "TEST-001",
            "message": "Integration test"
        }),
    );

    let result = subscriber.publish(event).await;
    assert!(result.is_ok(), "Webhook delivery should succeed");

    tokio::time::sleep(Duration::from_millis(500)).await;
}

#[tokio::test]
#[ignore]
async fn test_webhook_multiple_endpoints() {
    let config = WebhookConfig::default();
    let mut subscriber = WebhookSubscriber::new(config)
        .expect("Failed to create subscriber");

    // Register multiple endpoints
    let payment_endpoint = WebhookEndpoint {
        url: "http://localhost:3000/webhooks/payments".to_string(),
        secret: Some("payment-secret".to_string()),
        event_types: vec!["payment.processed".to_string()],
    };

    let order_endpoint = WebhookEndpoint {
        url: "http://localhost:3000/webhooks/orders".to_string(),
        secret: Some("order-secret".to_string()),
        event_types: vec!["order.created".to_string()],
    };

    subscriber
        .register_endpoint("payments", payment_endpoint)
        .await
        .expect("Failed to register payment endpoint");

    subscriber
        .register_endpoint("orders", order_endpoint)
        .await
        .expect("Failed to register order endpoint");

    // Publish to payment endpoint
    let payment_event = Event::new(
        "payment.processed",
        serde_json::json!({
            "payment_id": "PAY-001",
            "amount": 100.0
        }),
    );

    subscriber
        .publish(payment_event)
        .await
        .expect("Failed to publish payment");

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Publish to order endpoint
    let order_event = Event::new(
        "order.created",
        serde_json::json!({
            "order_id": "ORD-001",
            "items": 3
        }),
    );

    subscriber
        .publish(order_event)
        .await
        .expect("Failed to publish order");

    tokio::time::sleep(Duration::from_millis(500)).await;
}

#[tokio::test]
#[ignore]
async fn test_webhook_signature_verification() {
    let config = WebhookConfig {
        retry_attempts: 1,
        timeout_seconds: 5,
        verify_ssl: true,
        backpressure: BackpressureConfig::default(),
    };

    let mut subscriber = WebhookSubscriber::new(config)
        .expect("Failed to create subscriber");

    let endpoint = WebhookEndpoint {
        url: "http://localhost:3000/webhooks/secure".to_string(),
        secret: Some("secure-secret-key".to_string()),
        event_types: vec!["secure.event".to_string()],
    };

    subscriber
        .register_endpoint("secure", endpoint)
        .await
        .expect("Failed to register endpoint");

    let event = Event::new(
        "secure.event",
        serde_json::json!({
            "sensitive_data": "important information"
        }),
    );

    // The webhook receiver should verify the HMAC signature
    let result = subscriber.publish(event).await;
    assert!(result.is_ok(), "Signed webhook should be accepted");

    tokio::time::sleep(Duration::from_millis(500)).await;
}

#[tokio::test]
#[ignore]
async fn test_webhook_retry_on_failure() {
    let config = WebhookConfig {
        retry_attempts: 3,
        timeout_seconds: 2,
        verify_ssl: true,
        backpressure: BackpressureConfig::default(),
    };

    let mut subscriber = WebhookSubscriber::new(config)
        .expect("Failed to create subscriber");

    // Register endpoint to non-existent server (will fail)
    let endpoint = WebhookEndpoint {
        url: "http://localhost:9999/webhooks/fail".to_string(),
        secret: None,
        event_types: vec!["fail.event".to_string()],
    };

    subscriber
        .register_endpoint("fail", endpoint)
        .await
        .expect("Failed to register endpoint");

    let event = Event::new(
        "fail.event",
        serde_json::json!({"test": "retry"}),
    );

    let start = std::time::Instant::now();
    let result = subscriber.publish(event).await;

    // Should fail after retries
    assert!(result.is_err(), "Should fail to non-existent endpoint");

    // Should have taken time for retries
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() >= 1,
        "Should have attempted retries (elapsed: {:?})",
        elapsed
    );
}

#[tokio::test]
#[ignore]
async fn test_webhook_event_routing() {
    let config = WebhookConfig::default();
    let mut subscriber = WebhookSubscriber::new(config)
        .expect("Failed to create subscriber");

    // Endpoint only accepts specific event types
    let endpoint = WebhookEndpoint {
        url: "http://localhost:3000/webhooks/filtered".to_string(),
        secret: Some("filter-secret".to_string()),
        event_types: vec![
            "user.created".to_string(),
            "user.updated".to_string(),
        ],
    };

    subscriber
        .register_endpoint("filtered", endpoint)
        .await
        .expect("Failed to register endpoint");

    // This should be delivered
    subscriber
        .publish(Event::new("user.created", serde_json::json!({"id": 1})))
        .await
        .expect("Should deliver user.created");

    tokio::time::sleep(Duration::from_millis(200)).await;

    // This should also be delivered
    subscriber
        .publish(Event::new("user.updated", serde_json::json!({"id": 1})))
        .await
        .expect("Should deliver user.updated");

    tokio::time::sleep(Duration::from_millis(200)).await;

    // This should NOT be delivered (different event type)
    let result = subscriber
        .publish(Event::new("user.deleted", serde_json::json!({"id": 1})))
        .await;

    // Should succeed (no matching endpoints is not an error)
    assert!(result.is_ok(), "Should not error on unmatched event type");

    tokio::time::sleep(Duration::from_millis(500)).await;
}
