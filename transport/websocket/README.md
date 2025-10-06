# Watchtower WebSocket Transport

WebSocket transport implementation for Watchtower notification system.

## Overview

The WebSocket transport provides real-time bidirectional communication using [WebSocket protocol](https://datatracker.ietf.org/doc/html/rfc6455). It's ideal for applications requiring low-latency, persistent connections with duplex messaging.

## Features

- **Bidirectional Messaging**: Full-duplex communication with split read/write streams
- **Auto-Reconnect**: Automatic reconnection with configurable retry attempts
- **Text and Binary**: Support for both text and binary message formats
- **Ping/Pong Keepalive**: Connection health monitoring
- **Circuit Breaker**: Fault isolation for connection attempts
- **Dead Letter Queue**: In-memory failed event handling
- **Backpressure Control**: Queue management strategies
- **TLS Support**: Secure WebSocket (wss://) connections

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
watchtower-websocket = "0.1"
```

## Configuration

```rust
use watchtower_websocket::WebSocketConfig;
use watchtower_core::{BackpressureConfig, BackpressureStrategy};

let config = WebSocketConfig {
    // WebSocket server URL
    url: "ws://localhost:8080/events".to_string(),

    // Auto-reconnect settings
    max_reconnect_attempts: 5,
    reconnect_delay_seconds: 5,

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
use watchtower_websocket::{WebSocketConfig, WebSocketSubscriber};
use watchtower_core::{Event, EventMetadata, Subscriber};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = WebSocketConfig::default();
    let subscriber = WebSocketSubscriber::new(config).await?;

    let event = Event::new(
        "notification.sent",
        serde_json::json!({
            "user_id": 456,
            "message": "Your order has shipped!",
            "channel": "email"
        }),
        EventMetadata::default(),
    );

    subscriber.publish(event).await?;
    Ok(())
}
```

Events are serialized to JSON and sent as text messages:
```json
{
  "id": "evt_123",
  "event_type": "notification.sent",
  "payload": {...},
  "metadata": {...}
}
```

### Subscribing to Events

```rust
use watchtower_websocket::{WebSocketConfig, WebSocketSubscriber};
use watchtower_core::Subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = WebSocketConfig::default();
    let mut subscriber = WebSocketSubscriber::new(config).await?;

    // Subscribe to event types
    let handle = subscriber.subscribe(
        vec!["notification.sent".to_string(), "notification.failed".to_string()],
        |event| Box::pin(async move {
            println!("Received notification: {:?}", event);
            Ok(())
        })
    ).await?;

    // Keep running...
    tokio::signal::ctrl_c().await?;

    subscriber.unsubscribe(&handle).await?;
    Ok(())
}
```

## Bidirectional Communication

WebSocket transport uses split streams for simultaneous read/write:

```rust
use watchtower_websocket::WebSocketTransport;

let transport = WebSocketTransport::new(config).await?;

// Publish and subscribe simultaneously
tokio::join!(
    transport.publish(event1),
    transport.publish(event2),
    // Subscription runs in background
);
```

**Split Stream Architecture:**
- **Writer**: Dedicated for publishing events (locked per operation)
- **Reader**: Dedicated background task for receiving events
- **Isolation**: Read/write operations don't block each other

## Auto-Reconnect

Automatic reconnection on connection loss:

```rust
let config = WebSocketConfig {
    url: "ws://localhost:8080/events".to_string(),
    max_reconnect_attempts: 5,      // Try 5 times
    reconnect_delay_seconds: 5,      // Wait 5s between attempts
    ..Default::default()
};

let transport = WebSocketTransport::new(config).await?;

// If connection drops, transport automatically:
// 1. Detects closure via ping/pong or message errors
// 2. Waits 5 seconds
// 3. Attempts reconnection (up to 5 times)
// 4. Circuit breaker tracks reconnection attempts
```

**Reconnection Behavior:**
- Exponential backoff: `delay * attempt_number`
- Circuit breaker opens after max attempts
- Subscriptions resume automatically after reconnect

## Message Formats

### Text Messages

JSON-encoded events (default):

```rust
// Automatically serialized to JSON text
let event = Event::new(
    "user.login",
    serde_json::json!({"user_id": 123}),
    EventMetadata::default(),
);

subscriber.publish(event).await?;

// Sent as WebSocket text message:
// {"id":"...","event_type":"user.login","payload":{...}}
```

### Binary Messages

Also supported for deserialization:

```rust
// Transport can receive binary WebSocket messages
// Automatically deserializes JSON from binary data
// Useful when server sends binary-encoded JSON
```

## Circuit Breaker

WebSocket transport includes circuit breaker protection:

```rust
use watchtower_websocket::WebSocketTransport;

let transport = WebSocketTransport::new(config).await?;

// Circuit breaker tracks connection attempts
match transport.publish(event).await {
    Ok(_) => println!("Published successfully"),
    Err(e) if e.to_string().contains("Circuit breaker is open") => {
        println!("WebSocket unavailable, circuit breaker open");
    }
    Err(e) => println!("Error: {}", e),
}

// Monitor circuit breaker
let stats = transport.circuit_breaker_stats().await;
println!("State: {:?}", stats.state);
println!("Failures: {}", stats.failure_count);
```

**Circuit Breaker Triggers:**
- Connection failures during initial connect
- Reconnection failures after max attempts
- Message send failures

## Dead Letter Queue

In-memory DLQ for failed events:

```rust
use watchtower_websocket::WebSocketTransport;

let transport = WebSocketTransport::new(config).await?;

// Publish to DLQ (stored in-memory)
transport.publish_to_dlq(event, &error).await?;

// Consume from DLQ
transport.consume_dlq(|event| Box::pin(async move {
    println!("Retrying failed event: {:?}", event);
    retry_processing(&event).await?;
    Ok(())
})).await?;
```

**DLQ Characteristics:**
- **Storage**: In-memory VecDeque
- **Capacity**: 10,000 events (oldest dropped when full)
- **Persistence**: Lost on restart (not durable)
- **Use Case**: Temporary buffering for retry logic

## Monitoring

### Backpressure Statistics

```rust
let stats = subscriber.backpressure_stats().await;
println!("Queue size: {}/{}", stats.current_queue_size, stats.max_queue_size);
println!("Dropped: {}", stats.dropped_events);
println!("Fill ratio: {:.2}%", stats.fill_ratio() * 100.0);
```

### Circuit Breaker Statistics

```rust
let stats = transport.circuit_breaker_stats().await;
println!("Circuit state: {:?}", stats.state);
println!("Total requests: {}", stats.total_requests);
println!("Rejected: {}", stats.rejected_requests);
println!("Success rate: {:.2}%",
    (stats.total_requests - stats.rejected_requests) as f64 / stats.total_requests as f64 * 100.0
);
```

### Connection Health

```rust
// Health check verifies connection
match transport.health_check().await {
    Ok(_) => println!("WebSocket connected"),
    Err(e) => eprintln!("WebSocket disconnected: {}", e),
}
```

## Best Practices

### URL Formatting

Use proper WebSocket URL schemes:

```rust
// Good
"ws://localhost:8080/events"     // Plain WebSocket
"wss://api.example.com/events"   // Secure WebSocket (TLS)

// Bad
"http://localhost:8080/events"   // Wrong scheme
"localhost:8080"                 // Missing scheme
```

### Reconnection Strategy

Balance between recovery and resource usage:

```rust
// Quick recovery (low latency required)
let config = WebSocketConfig {
    max_reconnect_attempts: 10,
    reconnect_delay_seconds: 2,
    ..Default::default()
};

// Conservative (avoid overwhelming server)
let config = WebSocketConfig {
    max_reconnect_attempts: 3,
    reconnect_delay_seconds: 10,
    ..Default::default()
};
```

### Backpressure Configuration

Optimize for your message volume:

```rust
// High-volume, can tolerate loss
let config = WebSocketConfig {
    backpressure: BackpressureConfig {
        max_queue_size: 10000,
        strategy: BackpressureStrategy::DropOldest,
        warning_threshold: 0.9,
    },
    ..Default::default()
};

// Low-volume, cannot lose messages
let config = WebSocketConfig {
    backpressure: BackpressureConfig {
        max_queue_size: 100,
        strategy: BackpressureStrategy::Block,
        warning_threshold: 0.7,
    },
    ..Default::default()
};
```

### Error Handling

Implement graceful degradation:

```rust
match subscriber.publish(event.clone()).await {
    Ok(_) => println!("Event sent"),
    Err(e) => {
        // Log error
        eprintln!("WebSocket publish failed: {}", e);

        // Fallback: Store in DLQ for retry
        transport.publish_to_dlq(event, &e).await?;

        // Or: Switch to alternative transport
        fallback_transport.publish(event).await?;
    }
}
```

## Performance Tuning

### Connection Management

```rust
// Reuse single transport instance
let transport = Arc::new(WebSocketTransport::new(config).await?);

// Share across tasks
let t1 = transport.clone();
let t2 = transport.clone();

tokio::spawn(async move {
    t1.publish(event1).await
});

tokio::spawn(async move {
    t2.publish(event2).await
});
```

### Message Batching

For high throughput, batch sends:

```rust
// Publish multiple events concurrently
let futures: Vec<_> = events.iter()
    .map(|event| subscriber.publish(event.clone()))
    .collect();

futures::future::try_join_all(futures).await?;
```

### Ping/Pong Optimization

WebSocket ping/pong is handled automatically by the underlying library for connection keepalive.

## Troubleshooting

### Connection Issues

```rust
// Test basic connectivity
use tokio_tungstenite::connect_async;

match connect_async("ws://localhost:8080").await {
    Ok(_) => println!("WebSocket server reachable"),
    Err(e) => eprintln!("Connection failed: {}", e),
}
```

**Common Issues:**
- **Connection refused**: Server not running
- **TLS errors**: Certificate issues with wss://
- **Timeout**: Network issues or firewall

### Messages Not Received

1. **Verify subscription is active**:
```rust
// Check that subscribe() was called before publish
let handle = subscriber.subscribe(event_types, callback).await?;
// Now publish events
```

2. **Check event type matching**:
```rust
// Subscription filters by event type
// Ensure published event_type matches subscription
subscriber.subscribe(vec!["user.created".to_string()], callback).await?;
// This won't match:
Event::new("order.created", ...) // Different event type
```

3. **Verify connection is open**:
```rust
transport.health_check().await?;
```

### Circuit Breaker Open

```rust
// Check circuit breaker state
let stats = transport.circuit_breaker_stats().await;

if stats.state == CircuitState::Open {
    println!("Circuit is open, waiting for recovery...");
    println!("Failure count: {}", stats.failure_count);

    // Wait for timeout period to elapse
    // Circuit will transition to HalfOpen and allow test requests
    tokio::time::sleep(Duration::from_secs(60)).await;

    // Retry
    transport.publish(event).await?;
}
```

### Memory Issues (DLQ)

Monitor DLQ size:

```rust
// DLQ has 10K event limit
// Check if DLQ is full (indicates persistent issues)

// Custom monitoring (requires implementation)
// If DLQ consistently full, increase capacity or fix root cause
```

## Security

### TLS/SSL

Use secure WebSocket for production:

```rust
let config = WebSocketConfig {
    url: "wss://secure.example.com/events".to_string(),  // Note: wss://
    ..Default::default()
};

// TLS certificate validation is automatic
// For self-signed certificates, use custom connector (advanced)
```

### Authentication

WebSocket transport doesn't include built-in auth. Implement at application level:

```rust
// Option 1: Token in URL query parameter
let config = WebSocketConfig {
    url: "wss://api.example.com/events?token=abc123".to_string(),
    ..Default::default()
};

// Option 2: Custom headers (requires custom connector)
// Advanced use case - modify transport implementation
```

## Examples

See [examples/websocket_bidirectional.rs](../../examples/websocket_bidirectional.rs) for a complete working example.

## Related Documentation

- [WebSocket Protocol (RFC 6455)](https://datatracker.ietf.org/doc/html/rfc6455)
- [tokio-tungstenite Documentation](https://docs.rs/tokio-tungstenite/)
- [Watchtower Core](../../core/README.md)
- [Main README](../../README.md)
