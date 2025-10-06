# Integration Tests

This directory contains integration tests for all Watchtower transports. These tests require running instances of the actual services (NATS, Redis, RabbitMQ, WebSocket server, Webhook receiver).

## Quick Start

### 1. Start All Services

```bash
make services-up
```

This will start:
- NATS (port 4222)
- Redis (port 6379)
- RabbitMQ (port 5672, management: 15672)
- WebSocket server (port 8080)
- Webhook receiver (port 3000)

### 2. Run All Integration Tests

```bash
make test-integration
```

### 3. Run Specific Transport Tests

```bash
make test-integration-nats
make test-integration-redis
make test-integration-rabbitmq
make test-integration-websocket
make test-integration-webhook
```

---

## Test Files

### `nats_integration.rs`

Tests NATS functionality:
- ✅ Basic publish/subscribe
- ✅ Wildcard subscriptions (`user.*`)
- ✅ Multiple subscribers (broadcast)

**What it tests:**
- Subject-based routing
- Queue groups
- Message delivery guarantees
- Concurrent subscriptions

**Run:**
```bash
cargo test --test nats_integration -- --ignored
```

---

### `redis_integration.rs`

Tests Redis Streams functionality:
- ✅ Publish/subscribe with streams
- ✅ Consumer groups (load balancing)
- ✅ Stream persistence and recovery
- ✅ ACK/NACK handling

**What it tests:**
- Consumer group coordination
- Message persistence
- Redelivery on consumer restart
- Stream trimming

**Run:**
```bash
cargo test --test redis_integration -- --ignored
```

---

### `rabbitmq_integration.rs`

Tests RabbitMQ functionality:
- ✅ Basic publish/subscribe
- ✅ Topic routing with patterns
- ✅ Fanout exchange (broadcast)
- ✅ Message acknowledgment

**What it tests:**
- Exchange types (Topic, Fanout)
- Routing patterns
- Durable queues
- Publisher confirms
- Multiple subscribers

**Run:**
```bash
cargo test --test rabbitmq_integration -- --ignored
```

---

### `websocket_integration.rs`

Tests WebSocket functionality:
- ✅ Bidirectional communication
- ✅ Multiple subscriptions
- ✅ Connection management
- ✅ Echo server pattern

**What it tests:**
- Real-time messaging
- Split read/write streams
- Event routing
- Connection lifecycle

**Run:**
```bash
cargo test --test websocket_integration -- --ignored
```

---

### `webhook_integration.rs`

Tests Webhook functionality:
- ✅ HTTP POST delivery
- ✅ Multiple endpoints
- ✅ HMAC signature verification
- ✅ Retry on failure
- ✅ Event type filtering

**What it tests:**
- Endpoint registration
- HMAC-SHA256 signatures
- Retry mechanisms
- Circuit breakers
- Event routing

**Run:**
```bash
cargo test --test webhook_integration -- --ignored
```

---

## Test Architecture

### Integration Test Pattern

```rust
#[tokio::test]
#[ignore]  // Only run with --ignored flag
async fn test_name() {
    // 1. Setup transport
    let config = TransportConfig::default();
    let mut subscriber = TransportSubscriber::new(config).await?;

    // 2. Setup assertions
    let counter = Arc::new(AtomicU32::new(0));
    let c = counter.clone();

    // 3. Subscribe
    let handle = subscriber
        .subscribe(
            vec!["event.type".to_string()],
            Arc::new(move |event| {
                let c = c.clone();
                Box::pin(async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
            }),
        )
        .await?;

    // 4. Publish events
    subscriber.publish(Event::new("event.type", payload)).await?;

    // 5. Wait and assert
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(counter.load(Ordering::SeqCst), expected);

    // 6. Cleanup
    subscriber.unsubscribe(&handle).await?;
}
```

### Why `#[ignore]`?

Integration tests are marked with `#[ignore]` because they:
- Require external services to be running
- Take longer to execute
- May fail in CI without proper setup

To run them:
```bash
cargo test -- --ignored           # Run all ignored tests
cargo test --test nats_integration -- --ignored  # Run specific test
```

---

## Test Coverage

### NATS
| Feature | Tested |
|---------|--------|
| Publish/Subscribe | ✅ |
| Wildcards | ✅ |
| Queue Groups | ✅ |
| Reconnection | ⚠️ Manual |

### Redis
| Feature | Tested |
|---------|--------|
| Streams | ✅ |
| Consumer Groups | ✅ |
| Persistence | ✅ |
| ACK/NACK | ✅ |

### RabbitMQ
| Feature | Tested |
|---------|--------|
| Topic Exchange | ✅ |
| Fanout Exchange | ✅ |
| DLX | ⚠️ See examples |
| Priorities | ⚠️ See examples |

### WebSocket
| Feature | Tested |
|---------|--------|
| Bidirectional | ✅ |
| Multiple Subs | ✅ |
| Reconnect | ⚠️ Manual |

### Webhook
| Feature | Tested |
|---------|--------|
| HTTP POST | ✅ |
| HMAC Signatures | ✅ |
| Retry | ✅ |
| Event Routing | ✅ |

---

## Debugging Tests

### Enable Logging

```bash
RUST_LOG=debug cargo test --test nats_integration -- --ignored --nocapture
```

### Check Service Health

```bash
# NATS
curl http://localhost:8222/healthz

# Redis
redis-cli ping

# RabbitMQ
curl -u guest:guest http://localhost:15672/api/health/checks/alarms

# WebSocket
websocat ws://localhost:8080

# Webhook
curl http://localhost:3000/health
```

### View Service Logs

```bash
docker-compose logs -f nats
docker-compose logs -f redis
docker-compose logs -f rabbitmq
docker-compose logs -f websocket-server
docker-compose logs -f webhook-receiver
```

---

## Adding New Tests

1. Create test file: `tests/my_feature_integration.rs`

2. Add the test structure:

```rust
//! My feature integration tests
//!
//! These tests require...
//! Run with: cargo test --test my_feature_integration -- --ignored

use std::sync::Arc;
use tokio::time::Duration;
use watchtower_core::prelude::*;

#[tokio::test]
#[ignore]
async fn test_my_feature() {
    // Test implementation
}
```

3. Add to Makefile:

```makefile
test-integration-myfeature:
	@echo "Running My Feature integration tests..."
	@make services-up
	cargo test --test my_feature_integration -- --ignored
```

4. Update this README

---

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      nats:
        image: nats:latest
        ports:
          - 4222:4222

      redis:
        image: redis:latest
        ports:
          - 6379:6379

      rabbitmq:
        image: rabbitmq:3-management
        ports:
          - 5672:5672
          - 15672:15672

    steps:
      - uses: actions/checkout@v2

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run Integration Tests
        run: cargo test -- --ignored --test-threads=1
```

---

## Best Practices

### ✅ DO

- Always start services before running tests
- Use `Arc<AtomicU32>` for counters in async callbacks
- Add appropriate timeouts for assertions
- Clean up subscriptions after tests
- Use `--test-threads=1` to avoid race conditions
- Check service health before running tests

### ❌ DON'T

- Run tests without services
- Use unwrap() - use `?` instead
- Forget to unsubscribe
- Use fixed sleep times without timeouts
- Share state between tests
- Run integration tests in parallel (without isolation)

---

## Troubleshooting

### Tests Timeout

**Cause:** Services not running or unhealthy

**Solution:**
```bash
make services-restart
make check-services
```

### Connection Refused

**Cause:** Service ports not exposed or container not running

**Solution:**
```bash
docker-compose ps  # Check status
docker-compose logs [service]  # Check logs
```

### Random Failures

**Cause:** Race conditions or insufficient wait times

**Solution:**
- Use `timeout()` instead of fixed `sleep()`
- Run with `--test-threads=1`
- Increase wait times if needed

### Messages Not Received

**Cause:** Subscription not active before publish

**Solution:**
```rust
// Wait for subscription to be active
tokio::time::sleep(Duration::from_millis(500)).await;

// Then publish
subscriber.publish(event).await?;
```

---

## Performance Testing

For performance/load testing, see benchmarks:

```bash
cargo bench
```

---

## Related Documentation

- [Examples](../examples/README.md) - Practical usage examples
- [Architecture](../docs/ARCHITECTURE.md) - System design
- [Fault Tolerance](../docs/FAULT_TOLERANCE.md) - Reliability patterns
