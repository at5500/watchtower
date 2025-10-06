# Fault Tolerance

Watchtower provides built-in fault tolerance mechanisms to ensure reliable message delivery even in the face of failures.

## Circuit Breaker

The circuit breaker pattern prevents cascading failures by automatically isolating problematic endpoints.

### Configuration

```rust
use watchtower_core::{CircuitBreakerConfig, CircuitState};
use std::time::Duration;

let config = CircuitBreakerConfig {
    failure_threshold: 5,                  // Open after 5 failures
    timeout: Duration::from_secs(60),      // Retry after 60s
    success_threshold: 2,                  // Close after 2 successes
};
```

### States

#### Closed (Normal Operation)
- All requests pass through
- Failures are counted
- Transitions to **Open** when `failure_threshold` is reached

#### Open (Fast-Fail)
- All requests are rejected immediately
- No load on failing service
- After `timeout`, transitions to **HalfOpen**

#### HalfOpen (Recovery Testing)
- Limited requests are allowed through
- Success increments success counter
- Failure immediately returns to **Open**
- When `success_threshold` is reached, transitions to **Closed**

### State Transitions

```
        failure_threshold
Closed ──────────────────────> Open
  ▲                              │
  │                              │ timeout
  │ success_threshold            │
  │                              ▼
  └──────────────────────── HalfOpen
        failure
```

### Usage

```rust
use watchtower_webhook::WebhookClient;

let client = WebhookClient::new(config).await?;

match client.send(&url, &event).await {
    Ok(_) => println!("Sent successfully"),
    Err(e) if e.to_string().contains("Circuit breaker is open") => {
        eprintln!("Service unavailable, circuit breaker open");
        // Use fallback or queue for later
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

### Monitoring

```rust
let stats = client.circuit_breaker_stats().await;
println!("State: {:?}", stats.state);
println!("Failures: {}", stats.failure_count);
println!("Total requests: {}", stats.total_requests);
println!("Rejected: {}", stats.rejected_requests);
```

## Dead Letter Queue

Failed events are automatically routed to a Dead Letter Queue for later processing or analysis.

### Transport-Specific Implementations

#### RabbitMQ (Native DLX)

```rust
use watchtower_rabbitmq::RabbitMQConfig;

let config = RabbitMQConfig {
    queue_name: "events".to_string(),
    dead_letter_exchange: Some("events.dlx".to_string()),
    ..Default::default()
};
```

**DLQ Message Structure:**
- Headers: `x-error` (error message), `x-original-routing-key`
- Routing: `dlq.{event_type}`
- Durable storage

#### Redis Streams

```rust
use watchtower_redis::RedisTransport;

let transport = RedisTransport::new(config).await?;

// DLQ stream: {stream_prefix}:dlq
transport.publish_to_dlq(event, &error).await?;
```

**DLQ Stream Fields:**
- `event`: Serialized event
- `error`: Error message
- `original_type`: Original event type

#### NATS

```rust
use watchtower_nats::NatsClient;

let client = NatsClient::new(config).await?;

// DLQ subject: dlq.{event_type}
client.publish_to_dlq(&event, &error).await?;
```

**DLQ Message:**
```json
{
  "event": { /* original event */ },
  "error": "error message",
  "original_subject": "event.type"
}
```

#### WebSocket / Webhook (In-Memory)

```rust
// In-memory VecDeque with 10K capacity
// Oldest events dropped when full
// Not persistent across restarts
```

### Consuming from DLQ

```rust
transport.consume_dlq(|event| Box::pin(async move {
    println!("Processing failed event: {:?}", event);

    // Option 1: Retry processing
    match retry_logic(&event).await {
        Ok(_) => Ok(()),
        Err(e) => {
            // Log permanently failed events
            log_to_monitoring(&event, &e).await;
            Ok(()) // Don't re-queue
        }
    }
})).await?;
```

## Backpressure Control

Prevents system overload by managing event queue size.

### Strategies

#### DropOldest (FIFO Eviction)

Best for: Real-time systems where recent data is more valuable

```rust
use watchtower_core::{BackpressureConfig, BackpressureStrategy};

let config = BackpressureConfig {
    max_queue_size: 1000,
    strategy: BackpressureStrategy::DropOldest,
    warning_threshold: 0.8,
};
```

**Behavior:**
- Queue fills to 1000 events
- New event arrives → oldest event dropped
- Warning logged at 80% capacity (800 events)

#### DropNewest (Reject New)

Best for: Systems where data consistency is critical

```rust
let config = BackpressureConfig {
    max_queue_size: 1000,
    strategy: BackpressureStrategy::DropNewest,
    warning_threshold: 0.8,
};
```

**Behavior:**
- Queue fills to 1000 events
- New event arrives → new event rejected
- Publish returns error

#### Block (Async Wait)

Best for: Systems that can tolerate latency

```rust
let config = BackpressureConfig {
    max_queue_size: 1000,
    strategy: BackpressureStrategy::Block,
    warning_threshold: 0.8,
};
```

**Behavior:**
- Queue fills to 1000 events
- New event arrives → async wait until space available
- May cause timeout if queue never drains

### Monitoring

```rust
let stats = subscriber.backpressure_stats().await;
println!("Queue: {}/{}", stats.current_queue_size, stats.max_queue_size);
println!("Dropped: {}", stats.dropped_events);
println!("Fill ratio: {:.1}%", stats.fill_ratio() * 100.0);

if stats.fill_ratio() > 0.8 {
    eprintln!("WARNING: Queue at high capacity!");
}
```

### Tuning Guidelines

**High-throughput, low-latency:**
```rust
BackpressureConfig {
    max_queue_size: 10000,
    strategy: BackpressureStrategy::DropOldest,
    warning_threshold: 0.9,
}
```

**Reliable, cannot lose data:**
```rust
BackpressureConfig {
    max_queue_size: 1000,
    strategy: BackpressureStrategy::Block,
    warning_threshold: 0.7,
}
```

**Resource-constrained:**
```rust
BackpressureConfig {
    max_queue_size: 100,
    strategy: BackpressureStrategy::DropNewest,
    warning_threshold: 0.8,
}
```

## Retry Strategies

### Exponential Backoff

```rust
async fn publish_with_retry(
    subscriber: &impl Subscriber,
    event: Event,
    max_retries: u32,
) -> Result<(), WatchtowerError> {
    let mut delay = Duration::from_secs(1);

    for attempt in 0..=max_retries {
        match subscriber.publish(event.clone()).await {
            Ok(_) => return Ok(()),
            Err(e) if attempt == max_retries => return Err(e),
            Err(_) => {
                tokio::time::sleep(delay).await;
                delay *= 2; // Exponential backoff
            }
        }
    }
    unreachable!()
}
```

### Circuit Breaker Aware Retry

```rust
async fn smart_retry(
    client: &WebhookClient,
    event: Event,
) -> Result<(), WatchtowerError> {
    loop {
        match client.send(&url, &event).await {
            Ok(_) => return Ok(()),
            Err(e) if e.to_string().contains("Circuit breaker is open") => {
                // Wait for circuit to close
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## Health Checks

Monitor transport health:

```rust
async fn monitor_health(transport: &impl Transport) {
    loop {
        match transport.health_check().await {
            Ok(_) => println!("✓ Transport healthy"),
            Err(e) => eprintln!("✗ Transport unhealthy: {}", e),
        }

        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}
```

## Graceful Shutdown

```rust
async fn shutdown_gracefully(
    mut subscriber: impl Subscriber,
    handle: SubscriptionHandle,
) -> Result<(), WatchtowerError> {
    // Stop accepting new events
    subscriber.unsubscribe(&handle).await?;

    // Wait for in-flight events to complete
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Shutdown transport
    // (automatically called on drop)

    Ok(())
}
```

## Best Practices

1. **Always configure circuit breakers** for external services
2. **Monitor backpressure statistics** in production
3. **Set up DLQ consumers** to handle failed events
4. **Use appropriate backpressure strategy** for your use case
5. **Implement retry logic** with exponential backoff
6. **Health check regularly** and alert on failures
7. **Test failure scenarios** in development
8. **Document recovery procedures** for operations team

## Related Documentation

- [Architecture](ARCHITECTURE.md)
- [Monitoring & Observability](../README.docker.md#monitoring)
- [Transport Documentation](../README.md#transport-documentation)
