# Watchtower Webhook Transport

Webhook transport implementation for Watchtower notification system.

## Overview

The Webhook transport provides HTTP-based event notifications to external endpoints. It's ideal for integrating with third-party services, triggering external workflows, and one-way event delivery.

## Features

- **HTTP/HTTPS**: POST requests to configured endpoints
- **HMAC Signatures**: HMAC-SHA256 payload verification
- **Retry Logic**: Exponential backoff on failures
- **Circuit Breaker**: Per-URL fault isolation
- **Custom Headers**: Flexible header configuration
- **SSL Verification**: Optional certificate validation
- **Dead Letter Queue**: In-memory failed event buffering
- **Backpressure Control**: Queue management strategies
- **Timeout Configuration**: Request timeout limits

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
watchtower-webhook = "0.1"
```

## Configuration

```rust
use watchtower_webhook::WebhookConfig;
use watchtower_core::{BackpressureConfig, BackpressureStrategy};
use std::time::Duration;

let config = WebhookConfig {
    // Target URL
    url: "https://api.example.com/webhooks/events".to_string(),

    // Security
    secret: Some("your-secret-key".to_string()),  // For HMAC signatures
    verify_ssl: true,

    // HTTP configuration
    timeout: Duration::from_secs(30),
    headers: vec![
        ("X-API-Version".to_string(), "v1".to_string()),
        ("User-Agent".to_string(), "Watchtower/0.1".to_string()),
    ],

    // Retry settings
    max_retries: 3,
    retry_delay: Duration::from_secs(5),

    // Backpressure configuration
    backpressure: BackpressureConfig {
        max_queue_size: 1000,
        strategy: BackpressureStrategy::DropOldest,
        warning_threshold: 0.8,
    },
};
```

## Usage

### Publishing Events

```rust
use watchtower_webhook::{WebhookConfig, WebhookSubscriber};
use watchtower_core::{Event, EventMetadata, Subscriber};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = WebhookConfig {
        url: "https://api.example.com/webhooks/events".to_string(),
        secret: Some("shared-secret".to_string()),
        ..Default::default()
    };

    let subscriber = WebhookSubscriber::new(config).await?;

    let event = Event::new(
        "invoice.paid",
        serde_json::json!({
            "invoice_id": "INV-2024-001",
            "amount": 149.99,
            "paid_at": "2024-01-15T10:30:00Z"
        }),
        EventMetadata::default(),
    );

    subscriber.publish(event).await?;
    Ok(())
}
```

**HTTP Request:**
```http
POST /webhooks/events HTTP/1.1
Host: api.example.com
Content-Type: application/json
X-Webhook-Signature: sha256=<hmac-signature>
X-API-Version: v1
User-Agent: Watchtower/0.1

{
  "id": "evt_123",
  "event_type": "invoice.paid",
  "payload": {
    "invoice_id": "INV-2024-001",
    "amount": 149.99,
    "paid_at": "2024-01-15T10:30:00Z"
  },
  "metadata": {...}
}
```

### Subscribe Not Supported

Webhook transport is **publish-only** (one-way):

```rust
// This will return an error
subscriber.subscribe(event_types, callback).await?;
// Error: Webhook transport does not support subscriptions
```

**Rationale**: Webhooks are for outbound notifications. For bidirectional communication, use WebSocket or message queue transports.

## HMAC Signatures

Verify webhook authenticity with HMAC-SHA256:

### Sender (Watchtower)

```rust
let config = WebhookConfig {
    url: "https://receiver.example.com/webhook".to_string(),
    secret: Some("shared-secret-key".to_string()),
    ..Default::default()
};

// Signature automatically added to X-Webhook-Signature header
subscriber.publish(event).await?;
```

### Receiver (Your Server)

```python
import hmac
import hashlib

def verify_signature(payload: bytes, signature: str, secret: str) -> bool:
    expected = hmac.new(
        secret.encode(),
        payload,
        hashlib.sha256
    ).hexdigest()

    received = signature.replace('sha256=', '')
    return hmac.compare_digest(expected, received)

# In your webhook handler
@app.post("/webhook")
async def handle_webhook(request: Request):
    payload = await request.body()
    signature = request.headers.get('X-Webhook-Signature')

    if not verify_signature(payload, signature, 'shared-secret-key'):
        return {"error": "Invalid signature"}, 401

    # Process event
    event = json.loads(payload)
    process_event(event)
    return {"status": "ok"}
```

### Signature Format

```
X-Webhook-Signature: sha256=<hex-encoded-hmac>
```

**Algorithm**: HMAC-SHA256
**Input**: Raw JSON request body
**Key**: Shared secret from configuration

## Retry Logic

Automatic retry with exponential backoff:

```rust
let config = WebhookConfig {
    max_retries: 3,                         // Retry up to 3 times
    retry_delay: Duration::from_secs(5),    // Base delay: 5 seconds
    ..Default::default()
};

// Retry schedule:
// Attempt 1: Immediate
// Attempt 2: After 5 seconds
// Attempt 3: After 10 seconds (5 * 2)
// Attempt 4: After 15 seconds (5 * 3)
// Then: Give up, send to DLQ
```

**Retry Triggers:**
- HTTP 5xx errors (server errors)
- Network timeouts
- Connection failures

**No Retry:**
- HTTP 4xx errors (client errors - bad request)
- HTTP 2xx success

## Circuit Breaker

Per-URL circuit breakers prevent cascading failures:

```rust
use watchtower_webhook::WebhookClient;

let client = WebhookClient::new(config).await?;

// Each URL has independent circuit breaker
match client.send("https://service1.com/webhook", &event).await {
    Ok(_) => println!("Sent to service1"),
    Err(e) if e.to_string().contains("Circuit breaker is open") => {
        println!("Service1 unavailable, circuit breaker open");
    }
    Err(e) => println!("Error: {}", e),
}

// Different URL, different circuit breaker
client.send("https://service2.com/webhook", &event).await?;

// Monitor circuit breaker for specific URL
let stats = client.circuit_breaker_stats("https://service1.com/webhook").await;
println!("State: {:?}", stats.state);
println!("Failures: {}", stats.failure_count);
```

**Isolation Benefits:**
- Failures to one endpoint don't affect others
- Each URL tracked independently
- Prevents retry storms to failing endpoints

## Dead Letter Queue

In-memory DLQ for failed webhook deliveries:

```rust
use watchtower_webhook::WebhookTransport;

let transport = WebhookTransport::new(config).await?;

// Failed events automatically go to DLQ after max retries
transport.publish(event).await?;  // Fails after retries

// Manually publish to DLQ
transport.publish_to_dlq(event, &error).await?;

// Consume from DLQ for retry or logging
transport.consume_dlq(|event| Box::pin(async move {
    println!("Failed webhook delivery: {:?}", event);

    // Option 1: Retry with different endpoint
    fallback_webhook.publish(event.clone()).await?;

    // Option 2: Log to monitoring system
    log_failed_webhook(event).await?;

    Ok(())
})).await?;
```

**DLQ Characteristics:**
- **Storage**: In-memory VecDeque
- **Capacity**: 10,000 events (oldest dropped when full)
- **Persistence**: Lost on restart (not durable)
- **Retry**: Exponential backoff (5s base delay)

## Custom Headers

Add custom headers to webhook requests:

```rust
let config = WebhookConfig {
    url: "https://api.example.com/webhook".to_string(),
    headers: vec![
        ("X-API-Key".to_string(), "api-key-123".to_string()),
        ("X-Tenant-ID".to_string(), "tenant-456".to_string()),
        ("X-Request-ID".to_string(), "req-789".to_string()),
    ],
    ..Default::default()
};
```

**Resulting HTTP Headers:**
```http
POST /webhook HTTP/1.1
Content-Type: application/json
X-Webhook-Signature: sha256=...
X-API-Key: api-key-123
X-Tenant-ID: tenant-456
X-Request-ID: req-789
```

## SSL Verification

Control SSL certificate validation:

```rust
// Production: Verify SSL certificates (default)
let config = WebhookConfig {
    url: "https://api.example.com/webhook".to_string(),
    verify_ssl: true,
    ..Default::default()
};

// Development: Disable SSL verification (self-signed certs)
let config = WebhookConfig {
    url: "https://localhost:8443/webhook".to_string(),
    verify_ssl: false,  // WARNING: Only for development!
    ..Default::default()
};
```

**Security Warning**: Never disable SSL verification in production!

## Monitoring

### Circuit Breaker Statistics

```rust
// Per-URL statistics
let stats = client.circuit_breaker_stats("https://api.example.com/webhook").await;
println!("URL: https://api.example.com/webhook");
println!("Circuit state: {:?}", stats.state);
println!("Total requests: {}", stats.total_requests);
println!("Rejected: {}", stats.rejected_requests);
println!("Failure count: {}", stats.failure_count);

// Success rate
let success_rate = (stats.total_requests - stats.rejected_requests) as f64
    / stats.total_requests as f64 * 100.0;
println!("Success rate: {:.2}%", success_rate);
```

### Backpressure Statistics

```rust
let stats = subscriber.backpressure_stats().await;
println!("Queue size: {}/{}", stats.current_queue_size, stats.max_queue_size);
println!("Dropped events: {}", stats.dropped_events);
println!("Fill ratio: {:.2}%", stats.fill_ratio() * 100.0);

// Alert if queue is filling up
if stats.fill_ratio() > 0.8 {
    eprintln!("WARNING: Webhook queue at {:.0}% capacity", stats.fill_ratio() * 100.0);
}
```

### HTTP Response Tracking

Monitor webhook delivery success/failure:

```rust
match subscriber.publish(event).await {
    Ok(_) => {
        // Log success
        metrics.increment("webhook.success");
    }
    Err(e) => {
        // Log failure
        metrics.increment("webhook.failure");
        eprintln!("Webhook delivery failed: {}", e);
    }
}
```

## Best Practices

### URL Configuration

Use HTTPS in production:

```rust
// Good
"https://api.example.com/webhooks/events"

// Bad (unencrypted)
"http://api.example.com/webhooks/events"
```

### Secret Management

Never hardcode secrets:

```rust
// Bad
let config = WebhookConfig {
    secret: Some("hardcoded-secret".to_string()),
    ..Default::default()
};

// Good
use std::env;

let config = WebhookConfig {
    secret: env::var("WEBHOOK_SECRET").ok(),
    ..Default::default()
};
```

### Timeout Configuration

Set appropriate timeouts based on endpoint characteristics:

```rust
// Fast endpoint (API gateway)
let config = WebhookConfig {
    timeout: Duration::from_secs(10),
    ..Default::default()
};

// Slow endpoint (processes synchronously)
let config = WebhookConfig {
    timeout: Duration::from_secs(60),
    ..Default::default()
};
```

### Retry Strategy

Balance between delivery guarantees and resource usage:

```rust
// Critical events: More retries
let config = WebhookConfig {
    max_retries: 5,
    retry_delay: Duration::from_secs(10),
    ..Default::default()
};

// Non-critical events: Fewer retries
let config = WebhookConfig {
    max_retries: 1,
    retry_delay: Duration::from_secs(5),
    ..Default::default()
};
```

### Error Handling

Implement fallback mechanisms:

```rust
// Try primary webhook
match primary_webhook.publish(event.clone()).await {
    Ok(_) => println!("Primary delivery successful"),
    Err(e) => {
        eprintln!("Primary failed: {}", e);

        // Fallback to secondary webhook
        if let Err(e2) = secondary_webhook.publish(event.clone()).await {
            eprintln!("Secondary also failed: {}", e2);

            // Final fallback: log to persistent storage
            event_store.save(event).await?;
        }
    }
}
```

## Performance Tuning

### Connection Pooling

HTTP client automatically pools connections:

```rust
// Single client reused across requests
let client = WebhookClient::new(config).await?;

// Connections are pooled internally
for event in events {
    client.send(&config.url, &event).await?;
}
// Connection pool automatically managed
```

### Concurrent Delivery

Send to multiple webhooks concurrently:

```rust
let futures: Vec<_> = webhook_urls.iter()
    .map(|url| {
        let event = event.clone();
        async move {
            let config = WebhookConfig {
                url: url.clone(),
                ..Default::default()
            };
            let subscriber = WebhookSubscriber::new(config).await?;
            subscriber.publish(event).await
        }
    })
    .collect();

futures::future::try_join_all(futures).await?;
```

### Batch Processing

Group events for efficiency:

```rust
// Process events in batches
for chunk in events.chunks(100) {
    let futures: Vec<_> = chunk.iter()
        .map(|event| subscriber.publish(event.clone()))
        .collect();

    futures::future::try_join_all(futures).await?;

    // Optional: Rate limiting delay between batches
    tokio::time::sleep(Duration::from_millis(100)).await;
}
```

## Troubleshooting

### Connection Failures

```rust
// Test endpoint availability
match reqwest::get("https://api.example.com/health").await {
    Ok(response) if response.status().is_success() => {
        println!("Endpoint reachable");
    }
    Ok(response) => {
        eprintln!("Endpoint returned: {}", response.status());
    }
    Err(e) => {
        eprintln!("Connection failed: {}", e);
    }
}
```

**Common Issues:**
- **DNS resolution failure**: Check domain name
- **Connection timeout**: Network issues or firewall
- **SSL errors**: Certificate validation issues

### 4xx Client Errors

Webhook receiver rejecting requests:

```rust
// Check request format
// Ensure JSON payload is valid
// Verify required headers are present
// Check authentication (API keys, signatures)

// Example: Add required header
let config = WebhookConfig {
    headers: vec![
        ("Authorization".to_string(), "Bearer token".to_string()),
    ],
    ..Default::default()
};
```

### 5xx Server Errors

Endpoint experiencing issues:

```rust
// Circuit breaker will open automatically
// Check endpoint status
// Verify with endpoint provider
// Consider fallback endpoint

let stats = client.circuit_breaker_stats(&url).await;
if stats.state == CircuitState::Open {
    println!("Circuit breaker open due to server errors");
    println!("Failures: {}", stats.failure_count);
    // Use fallback or wait for recovery
}
```

### Signature Verification Failures

HMAC signature mismatch:

```rust
// Ensure secret matches between sender and receiver
// Verify signature is calculated on raw body (not parsed JSON)
// Check for whitespace or encoding issues

// Debug: Log signature
println!("Signature sent: {}", signature);

// Receiver should compare using timing-safe comparison
```

## Security

### HTTPS Only

Always use HTTPS in production:

```rust
let config = WebhookConfig {
    url: "https://api.example.com/webhook".to_string(),  // HTTPS
    verify_ssl: true,  // Verify certificates
    ..Default::default()
};
```

### Signature Verification

Enable HMAC signatures for authentication:

```rust
let config = WebhookConfig {
    secret: Some(env::var("WEBHOOK_SECRET")?),
    ..Default::default()
};

// Receiver MUST verify signature before processing
```

### Rate Limiting

Implement rate limiting to prevent abuse:

```rust
use std::time::Instant;
use tokio::time::Duration;

let mut last_send = Instant::now();
let rate_limit = Duration::from_millis(100);  // Max 10/second

for event in events {
    let elapsed = last_send.elapsed();
    if elapsed < rate_limit {
        tokio::time::sleep(rate_limit - elapsed).await;
    }

    subscriber.publish(event).await?;
    last_send = Instant::now();
}
```

## Examples

See [examples/webhook_signatures.rs](../../examples/webhook_signatures.rs) for a complete working example.

## Related Documentation

- [HTTP/1.1 Specification (RFC 7231)](https://datatracker.ietf.org/doc/html/rfc7231)
- [HMAC-SHA256 (RFC 2104)](https://datatracker.ietf.org/doc/html/rfc2104)
- [reqwest Documentation](https://docs.rs/reqwest/)
- [Watchtower Core](../../core/README.md)
- [Main README](../../README.md)
