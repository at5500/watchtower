# Watchtower Redis Streams Transport

Redis Streams transport implementation for Watchtower notification system.

## Overview

The Redis Streams transport provides persistent, ordered event streaming using [Redis Streams](https://redis.io/topics/streams-intro). It's ideal for applications requiring message persistence, replay capabilities, and consumer group coordination.

## Features

- **Persistent Streams**: Durable message storage with Redis
- **Consumer Groups**: Coordinated message processing with XREADGROUP
- **ACK/NACK**: Explicit message acknowledgment for reliability
- **Stream Trimming**: Automatic length limits with MAXLEN
- **Circuit Breaker**: Fault isolation for stream operations
- **Dead Letter Queue**: Failed event handling with `:dlq` stream suffix
- **Backpressure Control**: Queue management strategies

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
watchtower-redis = "0.1"
```

## Configuration

```rust
use watchtower_redis::RedisConfig;
use watchtower_core::{BackpressureConfig, BackpressureStrategy};

let config = RedisConfig {
    // Redis connection URL
    url: "redis://localhost:6379".to_string(),

    // Stream naming
    stream_prefix: "events".to_string(),

    // Consumer group configuration
    consumer_group: "watchtower-consumers".to_string(),
    consumer_name: "consumer-1".to_string(),

    // Stream length limit (0 = unlimited)
    max_stream_length: 10000,

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
use watchtower_redis::{RedisConfig, RedisSubscriber};
use watchtower_core::{Event, EventMetadata, Subscriber};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RedisConfig::default();
    let subscriber = RedisSubscriber::new(config).await?;

    let event = Event::new(
        "order.created",
        serde_json::json!({
            "order_id": "12345",
            "total": 99.99,
            "items": 3
        }),
        EventMetadata::default(),
    );

    subscriber.publish(event).await?;
    Ok(())
}
```

Events are added to Redis Streams with XADD:
```
XADD events:order.created * event "{json_payload}"
```

### Subscribing to Events

```rust
use watchtower_redis::{RedisConfig, RedisSubscriber};
use watchtower_core::Subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RedisConfig::default();
    let mut subscriber = RedisSubscriber::new(config).await?;

    // Subscribe to event types
    let handle = subscriber.subscribe(
        vec!["order.created".to_string(), "order.updated".to_string()],
        |event| Box::pin(async move {
            println!("Processing order event: {:?}", event);
            // Event is automatically ACK'd on success
            Ok(())
        })
    ).await?;

    // Keep running...
    tokio::signal::ctrl_c().await?;

    subscriber.unsubscribe(&handle).await?;
    Ok(())
}
```

### Consumer Groups

Consumer groups provide coordinated processing:

```rust
use watchtower_redis::RedisConfig;

// Consumer 1
let config1 = RedisConfig {
    url: "redis://localhost:6379".to_string(),
    consumer_group: "processors".to_string(),
    consumer_name: "processor-1".to_string(),
    ..Default::default()
};

// Consumer 2 (same group, different name)
let config2 = RedisConfig {
    url: "redis://localhost:6379".to_string(),
    consumer_group: "processors".to_string(),
    consumer_name: "processor-2".to_string(),
    ..Default::default()
};

// Messages are distributed across consumers in the group
let mut sub1 = RedisSubscriber::new(config1).await?;
let mut sub2 = RedisSubscriber::new(config2).await?;
```

## Stream Operations

### Stream Naming Convention

Streams are named using the pattern: `{stream_prefix}:{event_type}`

```rust
// Configuration
let config = RedisConfig {
    stream_prefix: "events".to_string(),
    ..Default::default()
};

// Publishing to "user.created" creates stream: "events:user.created"
// Publishing to "order.paid" creates stream: "events:order.paid"
```

### Stream Trimming

Automatically trim streams to prevent unbounded growth:

```rust
let config = RedisConfig {
    max_stream_length: 10000,  // Keep last 10k messages
    ..Default::default()
};

// Streams are trimmed with MAXLEN ~ 10000
// Uses approximate trimming (~) for better performance
```

### Message Acknowledgment

Messages are automatically ACK'd on successful processing:

```rust
subscriber.subscribe(
    vec!["payment.processed".to_string()],
    |event| Box::pin(async move {
        process_payment(&event).await?;
        // Automatic ACK here
        Ok(())
    })
).await?;
```

On error, messages are NACK'd (not requeued):

```rust
subscriber.subscribe(
    vec!["risky.operation".to_string()],
    |event| Box::pin(async move {
        if let Err(e) = dangerous_operation(&event).await {
            // Message is NACK'd, won't be retried
            return Err(e);
        }
        Ok(())
    })
).await?;
```

## Circuit Breaker

Redis transport includes circuit breaker protection:

```rust
use watchtower_redis::RedisTransport;

let transport = RedisTransport::new(config).await?;

// Circuit breaker tracks XADD operation health
match transport.publish(event).await {
    Ok(_) => println!("Published successfully"),
    Err(e) if e.to_string().contains("Circuit breaker is open") => {
        println!("Redis unavailable, circuit breaker open");
    }
    Err(e) => println!("Error: {}", e),
}

// Monitor circuit breaker
let stats = transport.circuit_breaker_stats().await;
println!("State: {:?}", stats.state);
println!("Failures: {}", stats.failure_count);
```

## Dead Letter Queue

Failed events are routed to a DLQ stream:

```rust
use watchtower_redis::RedisTransport;

let transport = RedisTransport::new(config).await?;

// Publish to DLQ (creates stream: events:dlq)
transport.publish_to_dlq(event, &error).await?;

// Consume from DLQ
transport.consume_dlq(|event| Box::pin(async move {
    println!("Retrying failed event: {:?}", event);
    retry_processing(&event).await?;
    Ok(())
})).await?;
```

DLQ stream structure:
```
Stream: {stream_prefix}:dlq
Fields:
  - event: {serialized_event}
  - error: {error_message}
  - original_type: {event_type}
```

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

### Redis Stream Info

Check stream status directly in Redis:

```bash
# Stream length
redis-cli XLEN events:order.created

# Consumer group info
redis-cli XINFO GROUPS events:order.created

# Pending messages
redis-cli XPENDING events:order.created watchtower-consumers
```

## Best Practices

### Stream Naming

Use hierarchical, descriptive names:

```rust
// Good
"orders.created"
"payments.completed"
"inventory.updated"

// Bad
"order1"
"payment"
```

### Consumer Group Strategy

- **Multiple consumers, same group**: Load balancing, each message processed once
- **Multiple consumers, different groups**: Broadcast, each consumer gets all messages

```rust
// Load balancing
let config = RedisConfig {
    consumer_group: "workers".to_string(),
    consumer_name: format!("worker-{}", worker_id),
    ..Default::default()
};

// Broadcasting
let config = RedisConfig {
    consumer_group: format!("service-{}", service_name),
    consumer_name: "consumer-1".to_string(),
    ..Default::default()
};
```

### Stream Trimming

Balance between history and storage:

```rust
// Keep 1 hour of messages (assuming 1000 msg/hour)
max_stream_length: 1000

// Keep 1 day of messages (assuming 100 msg/hour)
max_stream_length: 2400

// Unlimited (be careful with storage)
max_stream_length: 0
```

### Error Handling

Implement retry logic for transient errors:

```rust
subscriber.subscribe(
    event_types,
    |event| Box::pin(async move {
        let mut retries = 3;
        loop {
            match process(&event).await {
                Ok(_) => return Ok(()),
                Err(e) if retries > 0 => {
                    retries -= 1;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Err(e) => return Err(e),
            }
        }
    })
).await?;
```

## Performance Tuning

### Connection Pooling

Redis transport uses multiplexed connections:

```rust
// Single connection handles multiple concurrent operations
let transport = RedisTransport::new(config).await?;

// All operations share the connection
tokio::join!(
    transport.publish(event1),
    transport.publish(event2),
    transport.publish(event3),
);
```

### Batch Reading

Consumer reads multiple messages per poll:

```rust
// Configured in XREADGROUP call
// Reads up to 10 messages per iteration
StreamReadOptions::default()
    .group(&consumer_group, &consumer_name)
    .count(10)  // Batch size
    .block(1000); // Block for 1s if no messages
```

### Stream Trimming Performance

Use approximate trimming for better performance:

```rust
// Exact trimming (slower, exact count)
XADD stream MAXLEN = 1000 * field value

// Approximate trimming (faster, ~1000 messages)
XADD stream MAXLEN ~ 1000 * field value  // Used by Watchtower
```

## Troubleshooting

### Connection Issues

```bash
# Verify Redis is running
redis-cli ping

# Check connection
redis-cli -u redis://localhost:6379 ping
```

```rust
// Test connection in code
let config = RedisConfig::default();
match RedisTransport::new(config).await {
    Ok(t) => println!("Connected"),
    Err(e) => eprintln!("Failed: {}", e),
}
```

### Messages Not Consumed

1. **Check consumer group exists**:
```bash
redis-cli XINFO GROUPS events:your.stream
```

2. **Verify stream has messages**:
```bash
redis-cli XLEN events:your.stream
```

3. **Check for pending messages**:
```bash
redis-cli XPENDING events:your.stream your-group
```

### Memory Issues

Monitor stream lengths:

```bash
# Check all streams
redis-cli --scan --pattern "events:*" | xargs -L1 redis-cli XLEN

# Set up trimming
# Adjust max_stream_length in config
```

## Examples

See [examples/redis_streams.rs](../../examples/redis_streams.rs) for a complete working example.

## Related Documentation

- [Redis Streams Documentation](https://redis.io/topics/streams-intro)
- [Watchtower Core](../../core/README.md)
- [Main README](../../README.md)
