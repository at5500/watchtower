# Watchtower NATS Transport

NATS transport implementation for Watchtower notification system.

## Overview

The NATS transport provides lightweight, high-performance publish/subscribe messaging using [NATS](https://nats.io/). It's ideal for real-time event distribution across distributed systems.

## Features

- **Subject-based Routing**: Hierarchical subject naming with wildcard support
- **Queue Groups**: Load balancing across multiple consumers
- **Auto-Reconnect**: Automatic reconnection with configurable delay
- **Circuit Breaker**: Fault isolation for publish operations
- **Dead Letter Queue**: Failed event handling with `dlq.*` subject pattern
- **Backpressure Control**: Configurable queue management strategies

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
watchtower-nats = "0.1"
```

## Configuration

```rust
use watchtower_nats::NatsConfig;
use watchtower_core::{BackpressureConfig, BackpressureStrategy};

let config = NatsConfig {
    // NATS server URL
    url: "nats://localhost:4222".to_string(),

    // Reconnection settings
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
use watchtower_nats::{NatsConfig, NatsSubscriber};
use watchtower_core::{Event, EventMetadata, Subscriber};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = NatsConfig::default();
    let subscriber = NatsSubscriber::new(config).await?;

    let event = Event::new(
        "user.created",
        serde_json::json!({"user_id": 123, "email": "user@example.com"}),
        EventMetadata::default(),
    );

    subscriber.publish(event).await?;
    Ok(())
}
```

### Subscribing to Events

```rust
use watchtower_nats::{NatsConfig, NatsSubscriber};
use watchtower_core::Subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = NatsConfig::default();
    let mut subscriber = NatsSubscriber::new(config).await?;

    // Subscribe to specific event types
    let handle = subscriber.subscribe(
        vec!["user.created".to_string(), "user.updated".to_string()],
        |event| Box::pin(async move {
            println!("Received: {:?}", event);
            Ok(())
        })
    ).await?;

    // Keep running...
    tokio::signal::ctrl_c().await?;

    // Cleanup
    subscriber.unsubscribe(&handle).await?;
    Ok(())
}
```

### Subject Patterns

NATS supports hierarchical subjects with wildcards:

```rust
// Specific subject
subscriber.subscribe(vec!["user.created".to_string()], callback).await?;

// Single-level wildcard (*)
subscriber.subscribe(vec!["user.*".to_string()], callback).await?;

// Multi-level wildcard (>)
subscriber.subscribe(vec!["events.>".to_string()], callback).await?;
```

### Queue Groups

Use queue groups for load balancing across multiple consumers:

```rust
use watchtower_nats::NatsSubscriber;

// Multiple subscribers with the same queue group
// will receive messages in round-robin fashion
let mut sub1 = NatsSubscriber::new(config.clone()).await?;
let mut sub2 = NatsSubscriber::new(config.clone()).await?;

sub1.subscribe(vec!["orders.*".to_string()], callback1).await?;
sub2.subscribe(vec!["orders.*".to_string()], callback2).await?;
```

## Circuit Breaker

The NATS transport includes circuit breaker protection:

```rust
use watchtower_nats::NatsClient;

let client = NatsClient::new(config).await?;

// Circuit breaker automatically opens after failures
match client.publish("subject", &event).await {
    Ok(_) => println!("Published successfully"),
    Err(e) if e.to_string().contains("Circuit breaker is open") => {
        println!("Circuit breaker is open, service unavailable");
    }
    Err(e) => println!("Error: {}", e),
}

// Check circuit breaker status
let stats = client.circuit_breaker_stats().await;
println!("Circuit state: {:?}", stats.state);
println!("Failure count: {}", stats.failure_count);
```

## Dead Letter Queue

Failed events are automatically routed to DLQ:

```rust
use watchtower_nats::NatsClient;

let client = NatsClient::new(config).await?;

// Publish to DLQ
client.publish_to_dlq(&event, &error).await?;

// Consume from DLQ
client.subscribe_dlq(|event| Box::pin(async move {
    println!("Processing failed event: {:?}", event);
    // Retry logic here
    Ok(())
})).await?;
```

DLQ messages are published to subjects matching `dlq.*` pattern with the following structure:

```json
{
  "event": { /* original event */ },
  "error": "error message",
  "original_subject": "user.created"
}
```

## Monitoring

### Backpressure Statistics

```rust
let stats = subscriber.backpressure_stats().await;
println!("Current queue size: {}", stats.current_queue_size);
println!("Max queue size: {}", stats.max_queue_size);
println!("Dropped events: {}", stats.dropped_events);
println!("Strategy: {:?}", stats.strategy);
```

### Circuit Breaker Statistics

```rust
let stats = client.circuit_breaker_stats().await;
println!("State: {:?}", stats.state);
println!("Total requests: {}", stats.total_requests);
println!("Rejected requests: {}", stats.rejected_requests);
println!("Failure count: {}", stats.failure_count);
```

## Best Practices

### Subject Naming

Use hierarchical naming for better organization:

```rust
// Good: Hierarchical, descriptive
"users.created"
"orders.payment.completed"
"notifications.email.sent"

// Bad: Flat, unclear
"user_created"
"order_payment"
```

### Error Handling

Always handle subscription errors:

```rust
let handle = match subscriber.subscribe(event_types, callback).await {
    Ok(h) => h,
    Err(e) => {
        eprintln!("Subscription failed: {}", e);
        return Err(e.into());
    }
};
```

### Resource Cleanup

Ensure proper cleanup on shutdown:

```rust
// Unsubscribe
subscriber.unsubscribe(&handle).await?;

// The transport will automatically clean up on drop
```

## Performance Tuning

### Backpressure Configuration

Adjust based on your workload:

```rust
use watchtower_core::{BackpressureConfig, BackpressureStrategy};

// High-throughput, can tolerate message loss
let config = BackpressureConfig {
    max_queue_size: 10000,
    strategy: BackpressureStrategy::DropOldest,
    warning_threshold: 0.9,
};

// Low-latency, cannot lose messages
let config = BackpressureConfig {
    max_queue_size: 100,
    strategy: BackpressureStrategy::Block,
    warning_threshold: 0.7,
};
```

### Reconnection Settings

Balance between quick recovery and server load:

```rust
let config = NatsConfig {
    url: "nats://localhost:4222".to_string(),
    reconnect_delay_seconds: 5,  // Adjust based on requirements
    ..Default::default()
};
```

## Troubleshooting

### Connection Issues

```rust
// Check NATS server is running
// nats-server -p 4222

// Verify connection
let config = NatsConfig {
    url: "nats://localhost:4222".to_string(),
    ..Default::default()
};

match NatsSubscriber::new(config).await {
    Ok(sub) => println!("Connected successfully"),
    Err(e) => eprintln!("Connection failed: {}", e),
}
```

### Message Not Received

1. Verify subject patterns match
2. Check queue group configuration
3. Ensure subscriber is running before publishing
4. Check backpressure settings

### Circuit Breaker Open

```rust
// Reset circuit breaker manually if needed
// (This is typically not recommended, let it recover naturally)
let stats = client.circuit_breaker_stats().await;
if stats.state == CircuitState::Open {
    println!("Circuit is open, waiting for recovery...");
    // Wait for timeout period to elapse
}
```

## Examples

See [examples/nats_pubsub.rs](../../examples/nats_pubsub.rs) for a complete working example.

## Related Documentation

- [NATS Documentation](https://docs.nats.io/)
- [Watchtower Core](../../core/README.md)
- [Main README](../../README.md)
