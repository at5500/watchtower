# Watchtower Documentation

Welcome to the Watchtower documentation! This guide will help you understand and use Watchtower effectively.

## Table of Contents

### Core Concepts

- **[Architecture](ARCHITECTURE.md)** - System design, core traits, and component architecture
  - Transport and Subscriber traits
  - Event flow and concurrency model
  - Transport implementations overview
  - Extension points

- **[Fault Tolerance](FAULT_TOLERANCE.md)** - Reliability and error handling
  - Circuit Breaker pattern
  - Dead Letter Queue (DLQ)
  - Backpressure control
  - Retry strategies
  - Health checks

### Getting Started

- **[Quick Start](../README.md#quick-start)** - Get up and running quickly
- **[Development Setup](../README.docker.md)** - Docker, Makefile, and local development
- **[Examples](../examples/)** - Working code examples

### Transport Guides

Each transport has detailed documentation:

- **[NATS](../transport/nats/README.md)** - Lightweight pub/sub messaging
  - Subject-based routing
  - Queue groups
  - Auto-reconnect
  - Configuration and best practices

- **[Redis Streams](../transport/redis/README.md)** - Persistent stream processing
  - Consumer groups with XREADGROUP
  - Stream trimming
  - ACK/NACK acknowledgment
  - Performance tuning

- **[RabbitMQ](../transport/rabbitmq/README.md)** - Advanced message queuing
  - Exchange types (Direct, Topic, Fanout, Headers)
  - Durable queues and persistent messages
  - Dead Letter Exchange (DLX)
  - QoS and prefetch control

- **[WebSocket](../transport/websocket/README.md)** - Real-time bidirectional communication
  - Split read/write streams
  - Auto-reconnect with retry
  - Text and binary messages
  - TLS/SSL support

- **[Webhook](../transport/webhook/README.md)** - HTTP-based notifications
  - HMAC-SHA256 signatures
  - Per-URL circuit breakers
  - Retry with exponential backoff
  - Custom headers

### Advanced Topics

#### Monitoring & Observability

- Statistics API for all components
- Backpressure monitoring
- Circuit breaker state tracking
- Health checks

See: [FAULT_TOLERANCE.md - Monitoring](FAULT_TOLERANCE.md#monitoring)

#### Performance Optimization

- Connection pooling
- Batch processing
- Stream trimming strategies
- Backpressure tuning

See: Individual transport guides

#### Production Deployment

- Service recommendations
- Security best practices
- Scaling strategies
- Monitoring setup

See: [README.docker.md - Production Deployment](../README.docker.md#production-deployment)

## How to Use This Documentation

### I'm new to Watchtower
1. Start with [Quick Start](../README.md#quick-start)
2. Read [Architecture](ARCHITECTURE.md) to understand the design
3. Choose a transport and read its guide
4. Run [examples](../examples/) to see it in action

### I want to add a new transport
1. Read [Architecture - Extension Points](ARCHITECTURE.md#extension-points)
2. Study an existing transport implementation
3. Implement `Transport` and `Subscriber` traits
4. Add tests and documentation

### I'm troubleshooting issues
1. Check [Fault Tolerance](FAULT_TOLERANCE.md) for error handling
2. Review transport-specific troubleshooting sections
3. Enable logging and check statistics
4. See [Development Setup](../README.docker.md) for service health checks

### I need production-ready setup
1. Read [Production Deployment](../README.docker.md#production-deployment)
2. Configure [Fault Tolerance](FAULT_TOLERANCE.md) appropriately
3. Set up monitoring and alerting
4. Review security best practices in transport guides

## Contributing to Documentation

When contributing documentation:

1. **Main README** - Keep concise, overview only
2. **Transport READMEs** - Complete transport-specific guide
3. **docs/** - Detailed explanations of cross-cutting concerns
4. **Examples** - Working code with comments

See [CONTRIBUTING.md](../CONTRIBUTING.md) for details.

## Additional Resources

- [CHANGELOG](../CHANGELOG.md) - Version history
- [LICENSE](../LICENSE) - MIT License
- [Cargo.toml](../Cargo.toml) - Dependencies and workspace

## Need Help?

- Check [examples](../examples/) for working code
- Read transport-specific troubleshooting sections
- Review error messages and statistics
- Open an issue on GitHub (when available)

---

<div align="center">
  <sub>Documentation built with care for the Rust community</sub>
</div>
