# Watchtower RabbitMQ Transport

RabbitMQ transport implementation for Watchtower notification system.

## Overview

The RabbitMQ transport provides advanced message queuing using [RabbitMQ](https://www.rabbitmq.com/) with AMQP 0.9.1 protocol. It's ideal for applications requiring durable message queuing, complex routing, and delivery guarantees.

## Features

- **Multiple Exchange Types**: Direct, Topic, Fanout, Headers
- **Persistent Messaging**: Durable queues and persistent messages
- **Dead Letter Exchange**: Native DLX for failed message handling
- **Quality of Service**: Prefetch count and message priority
- **Circuit Breaker**: Fault isolation for publish operations
- **ACK/NACK**: Explicit message acknowledgment with requeue support
- **Backpressure Control**: Queue management strategies
- **Auto-Reconnect**: Automatic reconnection on connection loss

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
watchtower-rabbitmq = "0.1"
```

## Configuration

```rust
use watchtower_rabbitmq::RabbitMqConfig;
use watchtower_core::{BackpressureConfig, BackpressureStrategy};

let config = RabbitMqConfig {
    // Connection URL
    url: "amqp://localhost:5672".to_string(),

    // Exchange configuration
    exchange_name: "events".to_string(),
    exchange_type: "topic".to_string(),
    exchange_durable: true,

    // Queue configuration
    queue_name: "watchtower-events".to_string(),
    queue_durable: true,

    // Routing key pattern
    routing_key: "event.*".to_string(),

    // Quality of Service
    prefetch_count: 10,

    // Dead letter exchange
    dead_letter_exchange: Some("events.dlx".to_string()),

    // Message properties
    message_ttl: None,  // No TTL
    max_priority: Some(10),

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
use watchtower_rabbitmq::{RabbitMqConfig, RabbitMqSubscriber};
use watchtower_core::{Event, EventMetadata, Subscriber};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RabbitMqConfig::default();
    let subscriber = RabbitMqSubscriber::new(config).await?;

    let event = Event::new(
        "order.created",
        serde_json::json!({
            "order_id": "ORD-12345",
            "amount": 299.99,
            "customer_id": 789
        }),
        EventMetadata::default(),
    );

    subscriber.publish(event).await?;
    Ok(())
}
```

Events are published to the configured exchange with routing key derived from event type:
```
Exchange: events
Routing Key: order.created
Properties: persistent, priority=5
```

### Subscribing to Events

```rust
use watchtower_rabbitmq::{RabbitMqConfig, RabbitMqSubscriber};
use watchtower_core::Subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RabbitMqConfig::default();
    let mut subscriber = RabbitMqSubscriber::new(config).await?;

    // Subscribe to event types
    let handle = subscriber.subscribe(
        vec!["order.created".to_string(), "order.updated".to_string()],
        |event| Box::pin(async move {
            println!(\"Processing order event: {:?}\", event);
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

## Exchange Types

RabbitMQ supports multiple exchange types for different routing patterns:

### Direct Exchange

Exact routing key matching:

```rust
let config = RabbitMqConfig {
    exchange_name: "events.direct".to_string(),
    exchange_type: "direct".to_string(),
    routing_key: "user.created".to_string(),  // Exact match
    ..Default::default()
};
```

### Topic Exchange

Pattern-based routing with wildcards:

```rust
let config = RabbitMqConfig {
    exchange_name: "events.topic".to_string(),
    exchange_type: "topic".to_string(),
    routing_key: "order.*".to_string(),  // Matches order.created, order.updated, etc.
    ..Default::default()
};

// Wildcard patterns:
// * - matches exactly one word
// # - matches zero or more words
// Examples:
// "order.*" - matches "order.created", "order.updated"
// "order.#" - matches "order.created", "order.payment.completed"
```

### Fanout Exchange

Broadcast to all bound queues (ignores routing key):

```rust
let config = RabbitMqConfig {
    exchange_name: "events.fanout".to_string(),
    exchange_type: "fanout".to_string(),
    routing_key: "".to_string(),  // Ignored for fanout
    ..Default::default()
};
```

### Headers Exchange

Route based on message headers:

```rust
let config = RabbitMqConfig {
    exchange_name: "events.headers".to_string(),
    exchange_type: "headers".to_string(),
    ..Default::default()
};

// Routing based on headers (requires custom implementation)
```

## Message Properties

### Durability

Persistent messages survive broker restarts:

```rust
let config = RabbitMqConfig {
    exchange_durable: true,  // Exchange survives restarts
    queue_durable: true,     // Queue survives restarts
    // Messages are automatically marked persistent
    ..Default::default()
};
```

### Message TTL

Expire messages after a timeout:

```rust
let config = RabbitMqConfig {
    message_ttl: Some(60000),  // 60 seconds in milliseconds
    ..Default::default()
};
```

### Message Priority

Set priority levels for messages:

```rust
let config = RabbitMqConfig {
    max_priority: Some(10),  // Priorities 0-10
    ..Default::default()
};

// Messages are published with priority 5 by default
```

## Quality of Service

### Prefetch Count

Control how many unacknowledged messages a consumer can have:

```rust
let config = RabbitMqConfig {
    prefetch_count: 10,  // Process max 10 messages concurrently
    ..Default::default()
};
```

**Recommendations:**
- **High throughput**: Higher prefetch (50-100)
- **Resource-intensive**: Lower prefetch (1-10)
- **Fair distribution**: Prefetch = 1

## Circuit Breaker

RabbitMQ transport includes circuit breaker protection:

```rust
use watchtower_rabbitmq::RabbitMqTransport;

let transport = RabbitMqTransport::new(config).await?;

// Circuit breaker tracks publish operation health
match transport.publish(event).await {
    Ok(_) => println!("Published successfully"),
    Err(e) if e.to_string().contains("Circuit breaker is open") => {
        println!("RabbitMQ unavailable, circuit breaker open");
    }
    Err(e) => println!("Error: {}", e),
}

// Monitor circuit breaker
let stats = transport.circuit_breaker_stats().await;
println!("State: {:?}", stats.state);
println!("Failures: {}", stats.failure_count);
```

## Dead Letter Exchange

Native DLX support for failed messages:

```rust
use watchtower_rabbitmq::RabbitMqConfig;

let config = RabbitMqConfig {
    queue_name: "events".to_string(),
    dead_letter_exchange: Some("events.dlx".to_string()),
    ..Default::default()
};

let transport = RabbitMqTransport::new(config).await?;

// Publish to DLQ (uses DLX)
transport.publish_to_dlq(event, &error).await?;

// Consume from DLQ
transport.consume_dlq(|event| Box::pin(async move {
    println!("Retrying failed event: {:?}", event);
    retry_processing(&event).await?;
    Ok(())
})).await?;
```

**DLQ Message Headers:**
```
x-error: {error_message}
x-original-routing-key: {event_type}
```

**DLQ Setup:**
1. Messages NACK'd are routed to DLX
2. DLX routes to DLQ queue with `dlq.{event_type}` routing key
3. Consume from DLQ for retry logic

## Message Acknowledgment

### Auto-ACK on Success

Messages are automatically acknowledged on successful processing:

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

### NACK on Error

Failed messages are NACK'd and can be requeued or sent to DLQ:

```rust
subscriber.subscribe(
    vec!["risky.operation".to_string()],
    |event| Box::pin(async move {
        if let Err(e) = dangerous_operation(&event).await {
            // Message is NACK'd
            // If DLX configured, goes to DLQ
            // Otherwise, requeued or dropped
            return Err(e);
        }
        Ok(())
    })
).await?;
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

### RabbitMQ Management

Use RabbitMQ Management Plugin for monitoring:

```bash
# Enable management plugin
rabbitmq-plugins enable rabbitmq_management

# Access web UI
# http://localhost:15672
# Default credentials: guest/guest

# CLI commands
rabbitmqctl list_exchanges
rabbitmqctl list_queues name messages consumers
rabbitmqctl list_bindings
```

## Best Practices

### Exchange and Queue Naming

Use hierarchical, descriptive names:

```rust
// Good
exchange_name: "events.production"
queue_name: "order-processor.v1"

// Bad
exchange_name: "ex1"
queue_name: "queue"
```

### Durability Strategy

Balance between reliability and performance:

```rust
// High reliability (slower)
let config = RabbitMqConfig {
    exchange_durable: true,
    queue_durable: true,
    // Messages persistent by default
    ..Default::default()
};

// High performance (less reliable)
let config = RabbitMqConfig {
    exchange_durable: false,
    queue_durable: false,
    ..Default::default()
};
```

### Prefetch Tuning

Optimize based on message processing time:

```rust
// Fast processing (< 100ms per message)
prefetch_count: 50

// Medium processing (100ms - 1s per message)
prefetch_count: 10

// Slow processing (> 1s per message)
prefetch_count: 1
```

### Error Handling

Implement retry logic with exponential backoff:

```rust
subscriber.subscribe(
    event_types,
    |event| Box::pin(async move {
        let mut retries = 3;
        let mut delay = Duration::from_secs(1);

        loop {
            match process(&event).await {
                Ok(_) => return Ok(()),
                Err(e) if retries > 0 => {
                    retries -= 1;
                    tokio::time::sleep(delay).await;
                    delay *= 2;  // Exponential backoff
                }
                Err(e) => return Err(e),
            }
        }
    })
).await?;
```

## Performance Tuning

### Connection Pooling

RabbitMQ transport uses a single connection with multiple channels:

```rust
// Single connection is automatically managed
let transport = RabbitMqTransport::new(config).await?;

// Channels are created per operation internally
// No need to manually manage connection pool
```

### Publisher Confirms

Publisher confirms ensure messages are persisted:

```rust
// Automatically enabled in transport
// Publishes wait for broker confirmation
// Circuit breaker tracks confirmation failures
```

### Batch Publishing

For high-throughput scenarios, batch multiple publishes:

```rust
// Publish multiple events concurrently
let futures: Vec<_> = events.iter()
    .map(|event| transport.publish(event.clone()))
    .collect();

futures::future::try_join_all(futures).await?;
```

## Troubleshooting

### Connection Issues

```bash
# Verify RabbitMQ is running
rabbitmqctl status

# Check connection
rabbitmqctl list_connections
```

```rust
// Test connection in code
let config = RabbitMqConfig::default();
match RabbitMqTransport::new(config).await {
    Ok(t) => println!("Connected"),
    Err(e) => eprintln!("Failed: {}", e),
}
```

### Messages Not Consumed

1. **Check queue exists and has messages**:
```bash
rabbitmqctl list_queues name messages consumers
```

2. **Verify binding**:
```bash
rabbitmqctl list_bindings
```

3. **Check consumer count**:
```bash
# Should show > 0 consumers
rabbitmqctl list_queues name consumers
```

### Memory Issues

Monitor queue lengths and set limits:

```bash
# Check queue memory usage
rabbitmqctl list_queues name memory

# Set queue length limit (via policy)
rabbitmqctl set_policy queue-limit "^events" \
  '{"max-length":10000}' --apply-to queues
```

### DLQ Not Working

Verify DLX configuration:

```bash
# Check queue has DLX argument
rabbitmqctl list_queues name arguments

# Should show: x-dead-letter-exchange = events.dlx
```

## Examples

See [examples/rabbitmq_dlx.rs](../../examples/rabbitmq_dlx.rs) for a complete working example.

## Related Documentation

- [RabbitMQ Documentation](https://www.rabbitmq.com/documentation.html)
- [AMQP 0.9.1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html)
- [Watchtower Core](../../core/README.md)
- [Main README](../../README.md)
