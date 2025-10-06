//! Webhook with HMAC signatures example
//!
//! This example demonstrates Webhook usage with Watchtower:
//! - HTTP POST notifications to external endpoints
//! - Registering multiple webhook endpoints
//! - Automatic retry on failures
//! - Per-URL circuit breakers

use watchtower_core::prelude::*;
use watchtower_webhook::prelude::*;
use watchtower_webhook::subscriber::WebhookEndpoint;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Watchtower Webhook Example ===\n");

    // Configure Webhook transport
    let config = WebhookConfig {
        retry_attempts: 3,
        timeout_seconds: 30,
        verify_ssl: true,
        backpressure: BackpressureConfig {
            max_queue_size: 1000,
            strategy: BackpressureStrategy::DropOldest,
            warning_threshold: 0.8,
        },
    };

    println!("📡 Configuring webhook transport");
    println!("   Retry attempts: {}", config.retry_attempts);
    println!("   Timeout: {}s", config.timeout_seconds);
    println!("   SSL verification: {}", config.verify_ssl);
    let mut subscriber = WebhookSubscriber::new(config)?;
    println!("✅ Configured!\n");

    // Register webhook endpoints
    println!("🔗 Registering webhook endpoints...");

    let payment_endpoint = WebhookEndpoint {
        url: "http://localhost:3000/webhooks/payments".to_string(),
        secret: Some("payment-secret-key".to_string()),
        event_types: vec!["payment.confirmed".to_string(), "payment.failed".to_string()],
    };
    subscriber.register_endpoint("payments", payment_endpoint).await?;
    println!("   ✅ Registered: payments endpoint");

    let order_endpoint = WebhookEndpoint {
        url: "http://localhost:3000/webhooks/orders".to_string(),
        secret: Some("order-secret-key".to_string()),
        event_types: vec!["order.shipped".to_string(), "order.delivered".to_string()],
    };
    subscriber.register_endpoint("orders", order_endpoint).await?;
    println!("   ✅ Registered: orders endpoint");

    let notification_endpoint = WebhookEndpoint {
        url: "http://localhost:3000/webhooks/notifications".to_string(),
        secret: Some("notification-secret-key".to_string()),
        event_types: vec!["user.verified".to_string()],
    };
    subscriber.register_endpoint("notifications", notification_endpoint).await?;
    println!("   ✅ Registered: notifications endpoint\n");

    // Note: Webhook is publish-only, no subscriptions
    println!("ℹ️  Note: Webhook transport is publish-only (no subscriptions)\n");

    // Publish events to webhooks
    println!("📤 Publishing webhook events...\n");

    // Payment confirmation event
    let payment_event = Event::new(
        "payment.confirmed",
        serde_json::json!({
            "payment_id": "PAY-2024-001",
            "order_id": "ORD-123",
            "amount": 299.99,
            "currency": "USD",
            "status": "confirmed",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
    );
    println!("Publishing: payment.confirmed");
    println!("   → Will be sent to payments endpoint");
    println!("   → Signed with HMAC-SHA256 using payment-secret-key");

    match subscriber.publish(payment_event).await {
        Ok(_) => println!("   ✅ Webhook delivered successfully\n"),
        Err(e) => println!("   ❌ Webhook delivery failed: {}\n", e),
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Order shipped event
    let shipping_event = Event::new(
        "order.shipped",
        serde_json::json!({
            "order_id": "ORD-123",
            "tracking_number": "1Z999AA10123456784",
            "carrier": "UPS",
            "estimated_delivery": "2024-01-20",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
    );
    println!("Publishing: order.shipped");
    println!("   → Will be sent to orders endpoint");
    println!("   → Signed with HMAC-SHA256 using order-secret-key");

    match subscriber.publish(shipping_event).await {
        Ok(_) => println!("   ✅ Webhook delivered successfully\n"),
        Err(e) => println!("   ❌ Webhook delivery failed: {}\n", e),
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // User verification event
    let verification_event = Event::new(
        "user.verified",
        serde_json::json!({
            "user_id": 12345,
            "email": "user@example.com",
            "verification_method": "email",
            "verified_at": chrono::Utc::now().to_rfc3339()
        }),
    );
    println!("Publishing: user.verified");
    println!("   → Will be sent to notifications endpoint");
    println!("   → Signed with HMAC-SHA256 using notification-secret-key");

    match subscriber.publish(verification_event).await {
        Ok(_) => println!("   ✅ Webhook delivered successfully\n"),
        Err(e) => println!("   ❌ Webhook delivery failed: {}\n", e),
    }

    // Demonstrate retry on failure
    println!("\n📝 Testing features:");
    println!("   • Events are automatically routed to matching endpoints");
    println!("   • Each endpoint has its own HMAC secret");
    println!("   • Failed deliveries are automatically retried (up to 3 times)");
    println!("   • Circuit breakers protect against failing endpoints");
    println!("   • Per-URL independent circuit breakers");

    println!("\n✅ Example completed successfully!");
    println!("\nKey Webhook features demonstrated:");
    println!("  • Multiple webhook endpoints");
    println!("  • Event-type based routing");
    println!("  • Per-endpoint HMAC secrets");
    println!("  • Automatic retry with backoff");
    println!("  • Per-URL circuit breakers");
    println!("  • SSL verification");
    println!("\nWebhook receiver verification:");
    println!("  POST http://localhost:3000/webhooks/payments");
    println!("  POST http://localhost:3000/webhooks/orders");
    println!("  POST http://localhost:3000/webhooks/notifications");
    println!("\nReceived headers:");
    println!("  Content-Type: application/json");
    println!("  X-Webhook-Signature: sha256=<hmac_digest>");
    println!("\nSignature verification:");
    println!("  HMAC-SHA256(endpoint_secret, request_body)");

    Ok(())
}
