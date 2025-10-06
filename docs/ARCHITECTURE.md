# Architecture

Watchtower is built around two core traits that provide a clean separation between transport implementation and application logic.

## Core Traits

### Transport Trait

The `Transport` trait defines the low-level interface for message brokers and communication protocols:

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    fn info(&self) -> TransportInfo;
    async fn publish(&self, event: Event) -> Result<(), WatchtowerError>;
    async fn subscribe(&self, pattern: &str) -> Result<TransportSubscription, WatchtowerError>;
    async fn health_check(&self) -> Result<(), WatchtowerError>;
    async fn shutdown(&self) -> Result<(), WatchtowerError>;

    // Fault tolerance
    async fn publish_to_dlq(&self, event: Event, error: &WatchtowerError) -> Result<(), WatchtowerError>;
    async fn consume_dlq(&self, callback: EventCallback) -> Result<(), WatchtowerError>;
}
```

**Key Responsibilities:**
- Direct interaction with underlying transport (NATS, Redis, RabbitMQ, etc.)
- Connection management and health monitoring
- Error handling and fault tolerance
- Dead Letter Queue operations

### Subscriber Trait

The `Subscriber` trait provides a high-level, callback-based API for applications:

```rust
#[async_trait]
pub trait Subscriber: Send + Sync {
    async fn subscribe(
        &mut self,
        event_types: Vec<String>,
        callback: EventCallback,
    ) -> Result<SubscriptionHandle, WatchtowerError>;

    async fn unsubscribe(&mut self, handle: &SubscriptionHandle) -> Result<(), WatchtowerError>;
    async fn publish(&self, event: Event) -> Result<(), WatchtowerError>;
}
```

**Key Responsibilities:**
- Event filtering by type
- Callback execution with error handling
- Subscription lifecycle management
- Simplified API for application developers

## Component Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Application Layer                      │
│                                                             │
│    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    │
│    │  Publisher  │    │ Subscriber  │    │  Consumer   │    │
│    └──────┬──────┘    └──────┬──────┘    └──────┬──────┘    │
└───────────┼──────────────────┼──────────────────┼───────────┘
            │                  │                  │
            ▼                  ▼                  ▼
┌─────────────────────────────────────────────────────────────┐
│                     Subscriber Trait                        │
│  - Event filtering                                          │
│  - Callback management                                      │
│  - Backpressure control                                     │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                     Transport Trait                         │
│  - Protocol implementation                                  │
│  - Connection management                                    │
│  - Circuit breaker                                          │
│  - Dead Letter Queue                                        │
└──────────────────────────────┬──────────────────────────────┘
                               │
     ┌────────────┬────────────┼─────────────┬────────────┐
     ▼            ▼            ▼             ▼            ▼
┌──────────┐ ┌─────────┐ ┌──────────┐ ┌───────────┐ ┌─────────┐
│   NATS   │ │  Redis  │ │ RabbitMQ │ │ WebSocket │ │ Webhook │
└──────────┘ └─────────┘ └──────────┘ └───────────┘ └─────────┘
```

## Fault Tolerance Components

### Circuit Breaker

Three-state circuit breaker implementation:

- **Closed**: Normal operation, all requests pass through
- **Open**: Fast-fail mode, requests rejected immediately
- **HalfOpen**: Testing recovery, limited requests allowed

Configuration:
```rust
CircuitBreakerConfig {
    failure_threshold: 5,              // Failures before opening
    timeout: Duration::from_secs(60),  // Recovery timeout
    success_threshold: 2,              // Successes to close
}
```

### Backpressure Controller

Protects against overload with configurable strategies:

- **DropOldest**: FIFO eviction when queue is full
- **DropNewest**: Reject new events when full
- **Block**: Async wait for available space

### Dead Letter Queue

Failed events are routed to transport-specific DLQ implementations:

- **RabbitMQ**: Native Dead Letter Exchange (DLX)
- **Redis**: Separate `:dlq` stream with consumer group
- **NATS**: `dlq.*` subject pattern
- **WebSocket/Webhook**: In-memory VecDeque (10K limit)

## Event Flow

### Publishing

```
Application
    │
    ├─> Event Creation
    │
    ├─> Subscriber.publish()
    │      │
    │      ├─> Backpressure Check
    │      │
    │      ├─> Circuit Breaker Check
    │      │
    │      └─> Transport.publish()
    │             │
    │             ├─> Serialization
    │             │
    │             └─> Protocol-specific Send
    │
    └─> Success / Error
```

### Subscribing

```
Application
    │
    ├─> Subscriber.subscribe(event_types, callback)
    │      │
    │      ├─> Create Subscription
    │      │
    │      └─> Spawn Consumer Task
    │             │
    │             └─> Loop:
    │                   ├─> Transport.subscribe()
    │                   │
    │                   ├─> Receive Message
    │                   │
    │                   ├─> Deserialize Event
    │                   │
    │                   ├─> Filter by Type
    │                   │
    │                   ├─> Execute Callback
    │                   │      │
    │                   │      ├─> Success: ACK
    │                   │      │
    │                   │      └─> Error: NACK + DLQ
    │                   │
    │                   └─> Continue / Stop
    │
    └─> Return SubscriptionHandle
```

## Transport Implementations

Each transport implements both `Transport` and `Subscriber` traits with protocol-specific details:

### NATS
- Subject-based routing with wildcards
- Queue groups for load balancing
- Lightweight, no persistence

### Redis Streams
- Consumer groups with XREADGROUP
- Stream trimming for memory management
- ACK/NACK message acknowledgment

### RabbitMQ
- Exchange types: Direct, Topic, Fanout, Headers
- Durable queues and persistent messages
- Publisher confirms and prefetch control

### WebSocket
- Split read/write streams for bidirectional communication
- Auto-reconnect with exponential backoff
- Ping/pong keepalive

### Webhook
- HTTP POST with HMAC-SHA256 signatures
- Per-URL circuit breakers
- Retry with exponential backoff

## Concurrency Model

Watchtower is built on Tokio and uses:

- **Async/await**: All I/O operations are non-blocking
- **Arc/Mutex**: Shared state management
- **Channels**: Inter-task communication (bounded for backpressure)
- **Spawn**: Background tasks for consumers

## Error Handling

Error propagation follows a consistent pattern:

1. **Transport errors**: Connection, serialization, protocol errors
2. **Circuit breaker errors**: When circuit is open
3. **Backpressure errors**: When queue is full (strategy-dependent)
4. **Application errors**: Callback execution failures

All errors are wrapped in `WatchtowerError` enum for unified handling.

## Extension Points

Developers can extend Watchtower by:

1. **Implementing Transport trait**: Add new communication protocols
2. **Custom backpressure strategies**: Implement custom queue management
3. **Event transformation**: Middleware-style event processing
4. **Monitoring hooks**: Integrate with metrics systems

## Design Principles

1. **Modularity**: Each transport is independent, plug-and-play
2. **Fault Tolerance**: Circuit breakers and DLQ by default
3. **Performance**: Async I/O, connection pooling, batch processing
4. **Observability**: Statistics API for all components
5. **Developer Experience**: Prelude modules, builder patterns, clear errors
