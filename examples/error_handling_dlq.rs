//! Error handling and Dead Letter Queue example
//!
//! This example demonstrates:
//! - Handling errors in event processing
//! - Dead Letter Queue (DLQ) for failed messages
//! - Retry strategies
//! - Error monitoring and recovery

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use watchtower_core::prelude::*;
use watchtower_rabbitmq::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Watchtower Error Handling & DLQ Example ===\n");

    // Configure RabbitMQ with Dead Letter Exchange
    let config = RabbitMQConfig {
        url: "amqp://guest:guest@localhost:5672/%2f".to_string(),
        exchange: "app.events".to_string(),
        exchange_type: ExchangeType::Topic,
        queue_prefix: "app".to_string(),
        durable: true,
        persistent: true,
        auto_delete: false,
        dead_letter_exchange: Some("app.dlx".to_string()), // DLQ configuration
        message_ttl: 60000, // 60 seconds TTL
        max_priority: 10,
        prefetch_count: 10,
        retry_attempts: 3,
        retry_delay_seconds: 2,
        backpressure: BackpressureConfig::default(),
    };

    println!("📡 Connecting to RabbitMQ with DLX...");
    println!("   Main Exchange: {}", config.exchange);
    println!("   DLX: {:?}", config.dead_letter_exchange);
    println!("   Retry attempts: {}", config.retry_attempts);
    let mut subscriber = RabbitMQSubscriber::new(config).await?;
    println!("✅ Connected!\n");

    // Track error statistics
    let success_count = Arc::new(AtomicU32::new(0));
    let error_count = Arc::new(AtomicU32::new(0));
    let retry_count = Arc::new(AtomicU32::new(0));

    let sc = success_count.clone();
    let ec = error_count.clone();
    let rc = retry_count.clone();

    // Subscribe to payment events with error handling
    println!("🔔 Setting up payment processor with error handling...\n");
    let handle = subscriber
        .subscribe(
            vec!["payment.process".to_string()],
            Arc::new(move |event| {
                let sc = sc.clone();
                let ec = ec.clone();
                let rc = rc.clone();

                Box::pin(async move {
                    println!("💳 Processing payment event:");
                    println!("   Event ID: {}", event.id());

                    // Simulate different error scenarios
                    let payload = &event.payload;
                    let should_fail = payload
                        .get("simulate_failure")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let transient_error = payload
                        .get("transient_error")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if should_fail {
                        ec.fetch_add(1, Ordering::SeqCst);
                        println!("   ❌ Permanent failure - will go to DLQ");
                        println!("   Reason: Invalid payment data\n");
                        return Err(WatchtowerError::PublicationError(
                            "Invalid payment data".to_string(),
                        ));
                    }

                    if transient_error {
                        let retry_num = rc.fetch_add(1, Ordering::SeqCst);
                        if retry_num < 2 {
                            // Fail first 2 attempts
                            println!("   ⚠️  Transient error (attempt {})", retry_num + 1);
                            println!("   Reason: Network timeout - will retry\n");
                            return Err(WatchtowerError::PublicationError(
                                "Network timeout".to_string(),
                            ));
                        } else {
                            println!("   ✅ Succeeded after {} retries", retry_num);
                        }
                    }

                    // Successful processing
                    sc.fetch_add(1, Ordering::SeqCst);
                    println!("   Payload: {}", serde_json::to_string_pretty(&event.payload)?);
                    println!("   ✅ Payment processed successfully\n");

                    Ok(())
                })
            }),
        )
        .await?;

    println!("✅ Payment processor ready!\n");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    println!("📤 Publishing test events...\n");

    // 1. Successful payment
    println!("Test 1: Normal successful payment");
    subscriber
        .publish(Event::new(
            "payment.process",
            serde_json::json!({
                "payment_id": "PAY-001",
                "amount": 100.0,
                "currency": "USD"
            }),
        ))
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 2. Payment with transient error (will retry and succeed)
    println!("Test 2: Payment with transient error (will retry)");
    subscriber
        .publish(Event::new(
            "payment.process",
            serde_json::json!({
                "payment_id": "PAY-002",
                "amount": 200.0,
                "currency": "USD",
                "transient_error": true
            }),
        ))
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 3. Payment that will fail permanently (goes to DLQ)
    println!("Test 3: Payment with permanent failure (goes to DLQ)");
    subscriber
        .publish(Event::new(
            "payment.process",
            serde_json::json!({
                "payment_id": "PAY-003",
                "amount": -50.0,  // Invalid amount
                "simulate_failure": true
            }),
        ))
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 4. Another successful payment
    println!("Test 4: Another successful payment");
    subscriber
        .publish(Event::new(
            "payment.process",
            serde_json::json!({
                "payment_id": "PAY-004",
                "amount": 150.0,
                "currency": "EUR"
            }),
        ))
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Print statistics
    println!("\n📊 Processing Statistics:");
    println!("   ✅ Successful: {}", success_count.load(Ordering::SeqCst));
    println!("   ❌ Failed (to DLQ): {}", error_count.load(Ordering::SeqCst));
    println!("   🔄 Retry attempts: {}", retry_count.load(Ordering::SeqCst));

    // Cleanup
    println!("\n🧹 Cleaning up...");
    subscriber.unsubscribe(&handle).await?;

    println!("\n✅ Example completed!\n");
    println!("Key concepts demonstrated:");
    println!("  • Dead Letter Exchange (DLX) for failed messages");
    println!("  • Automatic retry with exponential backoff");
    println!("  • Distinguishing transient vs permanent errors");
    println!("  • Error tracking and monitoring");
    println!("\nDLQ Management:");
    println!("  View DLQ messages:");
    println!("    rabbitmqctl list_queues name messages");
    println!("  Process DLQ manually:");
    println!("    1. Inspect failed messages");
    println!("    2. Fix the issue");
    println!("    3. Re-queue or discard");
    println!("\nBest Practices:");
    println!("  ✓ Use DLX for all critical queues");
    println!("  ✓ Set appropriate TTL for messages");
    println!("  ✓ Monitor DLQ depth regularly");
    println!("  ✓ Implement DLQ processing workflow");
    println!("  ✓ Log all errors for debugging");

    Ok(())
}
