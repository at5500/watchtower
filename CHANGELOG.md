# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of watchtower notification library
- Modular transport architecture with pluggable backends
- Core traits: `Transport` and `Subscriber` for unified API
- Backpressure control with configurable strategies (DropOldest, DropNewest, Block)
- Dead Letter Queue (DLQ) support for all transports

#### Transports
- **NATS**: Subject-based messaging with queue groups and consumer support
- **Redis Streams**: XREADGROUP consumer groups with ACK/NACK
- **RabbitMQ**: AMQP with exchanges, queues, DLX, and prefetch control
- **WebSocket**: Bidirectional communication with auto-reconnect and split streams
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

### Transport-Specific Features

#### NATS Transport
- Automatic reconnection with configurable delay
- Subject-based routing with wildcard support
- Queue groups for load balancing
- DLQ with `dlq.*` subject pattern

#### Redis Streams Transport
- Consumer groups with XREADGROUP
- Stream max length configuration
- ACK/NACK message acknowledgment
- DLQ stream with `:dlq` suffix

#### RabbitMQ Transport
- Exchange types: Direct, Topic, Fanout, Headers
- Dead letter exchange configuration
- Message TTL and max priority
- Prefetch count (QoS) configuration
- Persistent/non-persistent messages
- ACK/NACK with requeue support
- DLQ queue bound to dead letter exchange

#### WebSocket Transport
- Auto-reconnect with retry attempts
- Ping/pong keepalive
- Split read/write streams for bidirectional communication
- Text and Binary message support
- In-memory DLQ with size limits

#### Webhook Transport
- HMAC-SHA256 signature verification
- Retry with exponential backoff
- SSL verification toggle
- Custom headers and timeout configuration
- In-memory DLQ with retry logic

### Project Structure
- `core/`: Base traits, errors, events, and shared functionality
- `config/`: Configuration management
- `transport/nats/`: NATS transport implementation
- `transport/redis/`: Redis Streams transport implementation
- `transport/rabbitmq/`: RabbitMQ transport implementation
- `transport/websocket/`: WebSocket transport implementation
- `transport/webhook/`: Webhook transport implementation
- `unified/`: Convenience wrapper for all transports

[Unreleased]: https://github.com/yourusername/watchtower/compare/v0.1.0...HEAD
