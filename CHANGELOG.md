# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **WebSocket Server Transport**: New server-side WebSocket transport implementation
  - `transport/websocket-server/`: Broadcast events to multiple connected WebSocket clients
  - Axum-based WebSocket server with connection management
  - Support for up to 1000+ concurrent client connections
  - Broadcast events to all connected clients simultaneously
  - Client metadata tracking and connection statistics
  - Configurable ping/pong keepalive mechanism
  - Built-in health check endpoints
  - Ready-to-use Axum handlers for easy integration
  - `examples/websocket-server/`: Complete working example with periodic event broadcasting
  - Added to workspace members and dependency tree
- **Architecture Documentation**: Added visual diagrams to README.md
  - ASCII diagram showing layered architecture (Application, Core, Transport, External Services)
  - Mermaid flowchart diagram showing component dependencies
  - Crate organization table with features for each transport
  - Architecture layers description
- **Integration Tests**: Comprehensive test suite for all transports
  - `tests/nats_integration.rs`: Pub/sub, wildcard subscriptions, multiple subscribers
  - `tests/redis_integration.rs`: Streams, consumer groups, persistence validation
  - `tests/rabbitmq_integration.rs`: Pub/sub, topic routing, fanout exchanges
  - `tests/websocket_integration.rs`: Bidirectional messaging, subscriptions, reconnection
  - `tests/webhook_integration.rs`: Delivery, multiple endpoints, HMAC signatures, retry logic, event routing
- **Advanced Examples**:
  - `examples/multi_transport.rs`: Orchestrate 4 transports in a complete order workflow
  - `examples/error_handling_dlq.rs`: DLQ usage, retry strategies, permanent vs transient errors
  - `examples/circuit_breaker.rs`: State transitions, failure detection, recovery patterns
  - `examples/backpressure.rs`: Demonstrate all 3 backpressure strategies with live metrics
- **Documentation**:
  - `tests/README.md`: Integration testing guide with setup instructions
  - `examples/README.md`: Comprehensive examples guide with troubleshooting
  - `ARCHITECTURE_ISSUES.md`: Detailed architecture analysis and improvement roadmap
  - Added "Planned Features" section to main README.md

### Fixed
- **Critical**: Implemented DropOldest backpressure strategy (was falling back to Block)
  - Added VecDeque-based manual queue management in `BackpressureController`
  - Proper FIFO semantics: oldest events dropped when queue reaches capacity
  - Statistics tracking for dropped events
- Fixed unused `event_type` parameter in `Event::with_metadata()` - now properly overrides metadata
- Fixed unused `error` field in DLQ entries - now logged for better debugging
- Removed all unused imports flagged by clippy
- Added documentation for intentionally stored fields (event_types in subscription metadata)

### Changed
- Updated Makefile with integration test commands for individual transports
- Updated Makefile with commands for running new advanced examples
- Enhanced DLQ logging to include original error information

### Added
- Initial release of watchtower notification library
- Modular transport architecture with pluggable backends
- Core traits: `Transport` and `Subscriber` for unified API
- Backpressure control with configurable strategies (DropOldest, DropNewest, Block)
- Dead Letter Queue (DLQ) support for all transports
- Circuit Breaker pattern for fault tolerance across all transports

#### Transports
- **NATS**: Subject-based messaging with queue groups and consumer support
- **Redis Streams**: XREADGROUP consumer groups with ACK/NACK
- **RabbitMQ**: AMQP with exchanges, queues, DLX, and prefetch control
- **WebSocket**: Bidirectional communication with auto-reconnect and split streams
- **WebSocket Server**: Server-side WebSocket broadcasting to multiple connected clients
- **Webhook**: HTTP-based notifications with HMAC signatures and retry logic

#### Features
- Bidirectional messaging (publish/subscribe) for all applicable transports
- Graceful shutdown with task abortion
- Health checks for connection monitoring
- Backpressure statistics and monitoring
- Event serialization/deserialization with serde_json
- Async message processing with tokio

#### Dead Letter Queue (DLQ)
- **RabbitMQ**: Native dead letter exchange with error metadata in headers
- **Redis Streams**: Separate `:dlq` stream with dedicated consumer group
- **NATS**: Subject pattern `dlq.*` with JSON payload containing event and error
- **WebSocket**: In-memory VecDeque with 10K event limit
- **Webhook**: In-memory VecDeque with exponential backoff retry (5s delay)

#### Circuit Breaker
- **Core Implementation**: Three-state pattern (Closed, Open, HalfOpen) with configurable thresholds
- **Configuration**:
  - `failure_threshold`: Number of failures before opening circuit (default: 5)
  - `timeout`: Recovery attempt delay (default: 60s)
  - `success_threshold`: Successes needed to close from half-open (default: 2)
- **Webhook**: Per-URL circuit breakers with isolated failure tracking
- **WebSocket**: Connection-level circuit breaker with retry protection
- **RabbitMQ**: Publish operation protection with confirmation tracking
- **Redis Streams**: XADD operation protection with error isolation
- **NATS**: Publish operation protection with subject-level monitoring
- **Features**:
  - Automatic state transitions based on success/failure patterns
  - Fast-fail behavior when circuit is open
  - Statistics API for monitoring (`circuit_breaker_stats()`)
  - Prevents cascading failures and resource exhaustion
  - Per-transport isolation (failures don't affect other transports)

### Transport-Specific Features

#### NATS Transport
- Automatic reconnection with configurable delay
- Subject-based routing with wildcard support
- Queue groups for load balancing
- DLQ with `dlq.*` subject pattern
- Circuit breaker on publish operations with success/failure tracking

#### Redis Streams Transport
- Consumer groups with XREADGROUP
- Stream max length configuration
- ACK/NACK message acknowledgment
- DLQ stream with `:dlq` suffix
- Circuit breaker on XADD operations with error isolation

#### RabbitMQ Transport
- Exchange types: Direct, Topic, Fanout, Headers
- Dead letter exchange configuration
- Message TTL and max priority
- Prefetch count (QoS) configuration
- Persistent/non-persistent messages
- ACK/NACK with requeue support
- DLQ queue bound to dead letter exchange
- Circuit breaker on publish operations with confirmation tracking

#### WebSocket Transport
- Auto-reconnect with retry attempts
- Ping/pong keepalive
- Split read/write streams for bidirectional communication
- Text and Binary message support
- In-memory DLQ with size limits
- Circuit breaker on connection attempts with retry protection

#### WebSocket Server Transport
- Axum-based WebSocket server implementation
- Broadcast events to multiple connected clients
- Connection manager with client tracking
- Configurable max connections and broadcast buffer
- Ping interval configuration for keepalive
- Client metadata support
- Active connections monitoring
- Health check API

#### Webhook Transport
- HMAC-SHA256 signature verification
- Retry with exponential backoff
- SSL verification toggle
- Custom headers and timeout configuration
- In-memory DLQ with retry logic
- Per-URL circuit breakers with isolated failure tracking

### Project Structure
- `core/`: Base traits, errors, events, and shared functionality
- `config/`: Configuration management
- `transport/nats/`: NATS transport implementation
- `transport/redis/`: Redis Streams transport implementation
- `transport/rabbitmq/`: RabbitMQ transport implementation
- `transport/websocket/`: WebSocket client transport implementation
- `transport/websocket-server/`: WebSocket server transport implementation
- `transport/webhook/`: Webhook transport implementation
- `unified/`: Convenience wrapper for all transports

[Unreleased]: https://github.com/yourusername/watchtower/compare/v0.1.0...HEAD
