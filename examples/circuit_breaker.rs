//! Circuit Breaker pattern example
//!
//! This example demonstrates:
//! - Circuit breaker states (Closed, Open, HalfOpen)
//! - Automatic failure detection
//! - Service protection from cascading failures
//! - Health monitoring and recovery

use std::time::Duration;
use watchtower_core::prelude::*;
use watchtower_webhook::prelude::*;
use watchtower_webhook::subscriber::WebhookEndpoint;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Watchtower Circuit Breaker Example ===\n");

    // Configure Webhook with circuit breaker
    let config = WebhookConfig {
        retry_attempts: 2,
        timeout_seconds: 2,
        verify_ssl: true,
        backpressure: BackpressureConfig::default(),
    };

    println!("📡 Setting up Webhook with circuit breaker...");
    let mut subscriber = WebhookSubscriber::new(config)?;

    // Register endpoints - one healthy, one failing
    let healthy_endpoint = WebhookEndpoint {
        url: "http://localhost:3000/webhooks/healthy".to_string(),
        secret: Some("healthy-secret".to_string()),
        event_types: vec!["test.event".to_string()],
    };

    let failing_endpoint = WebhookEndpoint {
        url: "http://localhost:9999/webhooks/failing".to_string(), // Non-existent
        secret: Some("failing-secret".to_string()),
        event_types: vec!["test.event".to_string()],
    };

    subscriber.register_endpoint("healthy", healthy_endpoint).await?;
    subscriber.register_endpoint("failing", failing_endpoint).await?;
    println!("✅ Endpoints registered\n");

    println!("🔄 Circuit Breaker States:");
    println!("   Closed:    Normal operation, requests pass through");
    println!("   Open:      Too many failures, requests blocked");
    println!("   HalfOpen:  Testing if service recovered\n");

    // Simulate circuit breaker behavior
    println!("📊 Demonstrating Circuit Breaker Pattern:\n");

    // Phase 1: Normal operation (Circuit CLOSED)
    println!("Phase 1: CLOSED - Normal operation");
    println!("─────────────────────────────────────");
    for i in 1..=3 {
        println!("Request {}: Sending to healthy endpoint...", i);
        let event = Event::new(
            "test.event",
            serde_json::json!({
                "request_id": i,
                "phase": "closed"
            }),
        );

        match subscriber.publish(event).await {
            Ok(_) => println!("   ✅ Success\n"),
            Err(e) => println!("   ❌ Failed: {}\n", e),
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    // Phase 2: Triggering circuit breaker (CLOSED → OPEN)
    println!("\nPhase 2: Triggering failures (CLOSED → OPEN)");
    println!("──────────────────────────────────────────────");
    println!("Sending requests to failing endpoint...\n");

    let mut failure_count = 0;
    for i in 1..=5 {
        println!("Request {}: Attempting to send...", i);
        let event = Event::new(
            "test.event",
            serde_json::json!({
                "request_id": i,
                "phase": "triggering_open"
            }),
        );

        let start = std::time::Instant::now();
        match subscriber.publish(event).await {
            Ok(_) => println!("   ✅ Success\n"),
            Err(e) => {
                failure_count += 1;
                let elapsed = start.elapsed();
                println!("   ❌ Failed: {} (after {:?})", e, elapsed);
                println!("   Failure count: {}\n", failure_count);

                if failure_count >= 3 {
                    println!("   ⚠️  CIRCUIT OPENED - Too many failures!\n");
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Phase 3: Circuit OPEN - Fast failures
    println!("\nPhase 3: OPEN - Failing fast");
    println!("─────────────────────────────");
    println!("Circuit is OPEN, requests fail immediately without trying...\n");

    for i in 1..=3 {
        println!("Request {}: Blocked by circuit breaker", i);
        let event = Event::new(
            "test.event",
            serde_json::json!({
                "request_id": i,
                "phase": "open"
            }),
        );

        let start = std::time::Instant::now();
        match subscriber.publish(event).await {
            Ok(_) => println!("   ✅ Success (unexpected)\n"),
            Err(e) => {
                let elapsed = start.elapsed();
                println!("   ⚡ Fast fail: {} ({:?})", e, elapsed);
                println!("   (No network call made - circuit is OPEN)\n");
            }
        }

        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    // Phase 4: Wait for half-open state
    println!("\nPhase 4: HALF_OPEN - Testing recovery");
    println!("──────────────────────────────────────");
    println!("Waiting for circuit breaker timeout...\n");
    println!("After timeout, circuit enters HALF_OPEN state");
    println!("Next request will test if service recovered\n");

    tokio::time::sleep(Duration::from_secs(5)).await;

    // Phase 5: Recovery attempt
    println!("Phase 5: Testing recovery with healthy endpoint");
    println!("─────────────────────────────────────────────");

    for i in 1..=3 {
        println!("Request {}: Testing recovery...", i);
        let event = Event::new(
            "test.event",
            serde_json::json!({
                "request_id": i,
                "phase": "recovery"
            }),
        );

        match subscriber.publish(event).await {
            Ok(_) => {
                println!("   ✅ Success!");
                println!("   Circuit is now CLOSED - service recovered\n");
            }
            Err(e) => {
                println!("   ❌ Still failing: {}", e);
                println!("   Circuit remains OPEN\n");
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\n✅ Circuit Breaker demonstration complete!\n");
    println!("Key Benefits:");
    println!("  • Prevents cascading failures");
    println!("  • Fast failure when service is down");
    println!("  • Automatic recovery detection");
    println!("  • Protects both client and server");
    println!("\nCircuit Breaker Configuration:");
    println!("  • Failure threshold: Number of failures before opening");
    println!("  • Timeout: How long to wait before testing recovery");
    println!("  • Success threshold: Successes needed to close circuit");
    println!("\nUse Cases:");
    println!("  ✓ External API calls");
    println!("  ✓ Database connections");
    println!("  ✓ Microservice communication");
    println!("  ✓ Any unreliable network operation");
    println!("\nMonitoring:");
    println!("  • Track circuit state changes");
    println!("  • Alert on OPEN state");
    println!("  • Monitor failure rates");
    println!("  • Dashboard for all circuits");

    Ok(())
}
