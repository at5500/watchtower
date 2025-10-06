# Watchtower Examples

This directory contains comprehensive examples demonstrating Watchtower's features and patterns.

## Prerequisites

Start all required services before running examples:

```bash
make services-up
```

## Transport-Specific Examples

### NATS Publish/Subscribe
```bash
make example-nats
# or: cargo run --example nats_pubsub
```

**Demonstrates:**
- Subject-based pub/sub
- Wildcard subscriptions (e.g., `user.*`)
- Multiple event types per subscription
- Async message handling

**Key Concepts:**
- Fast, in-memory messaging
- Subject-based routing
- Queue groups for load balancing

---

### Redis Streams
```bash
make example-redis
# or: cargo run --example redis_streams
```

**Demonstrates:**
- Consumer groups for load balancing
- Stream persistence
- ACK/NACK message acknowledgment
- Stream trimming

**Key Concepts:**
- Event sourcing
- Persistent message storage
- Consumer group coordination

---

### RabbitMQ with Dead Letter Exchange
```bash
make example-rabbitmq
# or: cargo run --example rabbitmq_dlx
```

**Demonstrates:**
- Topic exchange with routing patterns
- Durable queues and persistent messages
- Message acknowledgment (ACK/NACK)
- Dead Letter Exchange for failed messages
- Publisher confirms

**Key Concepts:**
- Guaranteed message delivery
- DLX for error handling
- Message priorities

---

### WebSocket Bidirectional Communication
```bash
make example-websocket
# or: cargo run --example websocket_bidirectional
```

**Demonstrates:**
- Bidirectional real-time communication
- Split read/write streams
- Text message format (JSON)
- Auto-reconnect on disconnect

**Key Concepts:**
- Real-time updates
- Full-duplex communication
- Connection management

---

### Webhook with HMAC Signatures
```bash
make example-webhook
# or: cargo run --example webhook_signatures
```

**Demonstrates:**
- HTTP POST notifications to external endpoints
- Registering multiple webhook endpoints
- Automatic retry on failures
- Per-URL circuit breakers
- HMAC-SHA256 signature verification

**Key Concepts:**
- External API integration
- Security with HMAC
- Fault tolerance

---

## Advanced Pattern Examples

### Multi-Transport Integration
```bash
make example-multi-transport
# or: cargo run --example multi_transport
```

**Demonstrates:**
- Using multiple transports together
- NATS for real-time messaging
- Redis Streams for event sourcing
- RabbitMQ for task queues
- Webhook for external notifications

**Key Concepts:**
- Choose the right transport for each use case
- Orchestrate multiple messaging systems
- Complete order workflow example

**Use Case:** E-commerce order processing system

---

### Error Handling & Dead Letter Queue
```bash
make example-error-handling
# or: cargo run --example error_handling_dlq
```

**Demonstrates:**
- Handling errors in event processing
- Dead Letter Queue (DLQ) for failed messages
- Retry strategies with exponential backoff
- Distinguishing transient vs permanent errors
- Error monitoring and statistics

**Key Concepts:**
- DLX configuration
- Automatic retry mechanisms
- Error tracking and recovery

**Use Case:** Payment processing with error handling

---

### Circuit Breaker Pattern
```bash
make example-circuit-breaker
# or: cargo run --example circuit_breaker
```

**Demonstrates:**
- Circuit breaker states (Closed, Open, HalfOpen)
- Automatic failure detection
- Service protection from cascading failures
- Health monitoring and recovery
- Fast failure when service is down

**Key Concepts:**
- Prevent cascading failures
- Automatic recovery detection
- Protect both client and server

**Use Case:** External API calls with reliability

---

### Backpressure Handling
```bash
make example-backpressure
# or: cargo run --example backpressure
```

**Demonstrates:**
- Three backpressure strategies:
  - **DropOldest**: Drop oldest messages (real-time data)
  - **DropNewest**: Drop newest messages (historical data)
  - **Block**: Block until space available (critical operations)
- Queue monitoring and warnings
- Handling overload scenarios

**Key Concepts:**
- Flow control mechanisms
- Preventing system overload
- Choosing the right strategy

**Use Case:** High-throughput event processing

---

## Other Examples

### Prelude Usage
```bash
make example-prelude
# or: cargo run --example prelude_usage
```

Shows how to use prelude modules for convenient imports.

### Derive Macro
```bash
make example-derive
# or: cargo run --example derive_macro
```

Demonstrates automatic event generation from structs using `#[derive(Event)]`.

---

## Running All Examples

```bash
make examples-all
```

This will run all examples sequentially.

---

## Common Patterns

### Basic Event Publishing

```rust
use watchtower_core::prelude::*;
use watchtower_nats::prelude::*;

let config = NatsConfig::default();
let mut subscriber = NatsSubscriber::new(config).await?;

let event = Event::new(
    "user.created",
    serde_json::json!({"user_id": 123})
);

subscriber.publish(event).await?;
```

### Event Subscription

```rust
let handle = subscriber
    .subscribe(
        vec!["user.created".to_string()],
        Arc::new(|event| {
            Box::pin(async move {
                println!("Received: {}", event.event_type());
                Ok(())
            })
        }),
    )
    .await?;
```

### Error Handling

```rust
Arc::new(|event| {
    Box::pin(async move {
        match process_event(&event).await {
            Ok(_) => Ok(()),
            Err(e) if is_transient(&e) => {
                // Will retry automatically
                Err(WatchtowerError::PublicationError(e.to_string()))
            }
            Err(e) => {
                // Permanent error - goes to DLQ
                Err(WatchtowerError::PublicationError(e.to_string()))
            }
        }
    })
})
```

---

## Troubleshooting

### Services Not Running

If examples fail to connect, ensure services are running:

```bash
make services-up
docker-compose ps
```

### Check Service Health

```bash
make check-services
```

### View Service Logs

```bash
make services-logs
```

### Restart Services

```bash
make services-restart
```

---

## Next Steps

1. **Read the documentation**: `make docs`
2. **Run integration tests**: `make test-integration`
3. **Explore the source code**: Check `transport/*/src/`
4. **Customize examples**: Modify and experiment!

---

## Architecture Guides

For deeper understanding, see:
- [Architecture](../docs/ARCHITECTURE.md) - System design and components
- [Fault Tolerance](../docs/FAULT_TOLERANCE.md) - Reliability patterns
- [Main README](../README.md) - Quick start and overview
