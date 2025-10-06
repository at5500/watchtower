# Docker & Development Setup

This guide explains how to run the required services and examples for Watchtower development and testing.

## Quick Start

### Start All Services

```bash
make services-up
```

This starts:
- **NATS** on port 4222 (monitoring on 8222)
- **Redis** on port 6379
- **RabbitMQ** on ports 5672 (AMQP) and 15672 (management UI)
- **WebSocket test server** on port 8080
- **Webhook test receiver** on port 3000

### Run Examples

```bash
# Run NATS example
make example-nats

# Run Redis Streams example
make example-redis

# Run RabbitMQ example
make example-rabbitmq

# Run WebSocket example
make example-websocket

# Run Webhook example
make example-webhook
```

### Stop Services

```bash
make services-down
```

## Services

### NATS

**Port**: 4222 (client), 8222 (monitoring)

**Health Check**:
```bash
curl http://localhost:8222/healthz
```

**CLI Tools**:
```bash
# Install NATS CLI
brew install nats-io/nats-tools/nats

# Subscribe to subjects
nats sub ">"

# Publish message
nats pub "test.subject" "hello world"
```

**Monitoring**:
- Web UI: http://localhost:8222
- Endpoints: `/varz`, `/connz`, `/subsz`, `/routez`

### Redis

**Port**: 6379

**Health Check**:
```bash
redis-cli ping
```

**CLI Tools**:
```bash
# Connect to Redis
redis-cli

# List streams
SCAN 0 MATCH events:*

# View stream info
XINFO STREAM events:order.created

# View consumer groups
XINFO GROUPS events:order.created

# View pending messages
XPENDING events:order.created watchtower-consumers
```

**Configuration**:
- Persistence: AOF enabled
- Max memory: 256MB
- Eviction policy: allkeys-lru

### RabbitMQ

**Ports**: 5672 (AMQP), 15672 (management)

**Credentials**: guest/guest

**Health Check**:
```bash
curl -u guest:guest http://localhost:15672/api/health/checks/alarms
```

**Management UI**:
- URL: http://localhost:15672
- Username: guest
- Password: guest

**CLI Tools**:
```bash
# List exchanges
rabbitmqctl list_exchanges

# List queues
rabbitmqctl list_queues name messages consumers

# List bindings
rabbitmqctl list_bindings

# View connections
rabbitmqctl list_connections
```

### WebSocket Test Server

**Port**: 8080

**Endpoint**: `ws://localhost:8080`

**Features**:
- Echo messages back to sender
- Supports text and binary messages
- Automatic ping/pong handling

**Test**:
```bash
# Using websocat (install: brew install websocat)
echo '{"test":"message"}' | websocat ws://localhost:8080
```

### Webhook Test Receiver

**Port**: 3000

**Endpoints**:
- Health: `GET http://localhost:3000/health`
- Webhook: `POST http://localhost:3000/webhook`
- Webhook (alt): `POST http://localhost:3000/webhooks/events`

**Environment**:
- `WEBHOOK_SECRET`: HMAC verification secret (default: test-secret-key)

**Test**:
```bash
# Without signature
curl -X POST http://localhost:3000/webhook \
  -H "Content-Type: application/json" \
  -d '{"id":"test","event_type":"test.event","payload":{},"metadata":{}}'

# With HMAC signature
SECRET="test-secret-key"
PAYLOAD='{"id":"test","event_type":"test.event","payload":{},"metadata":{}}'
SIGNATURE=$(echo -n "$PAYLOAD" | openssl dgst -sha256 -hmac "$SECRET" | sed 's/^.* //')

curl -X POST http://localhost:3000/webhook \
  -H "Content-Type: application/json" \
  -H "X-Webhook-Signature: sha256=$SIGNATURE" \
  -d "$PAYLOAD"
```

## Development Workflow

### 1. Start Services

```bash
make services-up
```

### 2. Check Service Health

```bash
make check-services
```

### 3. Run Tests

```bash
# Unit tests only
make test

# Integration tests (requires services)
make test-integration

# All tests
make test-all
```

### 4. Run Examples

```bash
make example-nats
```

### 5. View Logs

```bash
# All services
make services-logs

# Specific service
docker-compose logs -f nats
docker-compose logs -f redis
docker-compose logs -f rabbitmq
```

### 6. Restart Services

```bash
make services-restart
```

### 7. Clean Up

```bash
# Stop services
make services-down

# Stop and remove volumes
make services-clean
```

## Docker Compose Services

### Service Configuration

**docker-compose.yml** defines:
- Health checks for all services
- Persistent volumes for Redis and RabbitMQ
- Isolated network (`watchtower`)
- Port mappings for development

### Volume Management

**Named Volumes**:
- `redis-data`: Redis AOF persistence
- `rabbitmq-data`: RabbitMQ message storage

**Clean Volumes**:
```bash
make services-clean
# or
docker-compose down -v
```

## Building Docker Images

### WebSocket Server

```bash
make docker-build-websocket
# or
docker build -f docker/Dockerfile.websocket -t watchtower-websocket .
```

### Webhook Receiver

```bash
make docker-build-webhook
# or
docker build -f docker/Dockerfile.webhook -t watchtower-webhook .
```

### All Images

```bash
make docker-build-all
```

## Troubleshooting

### Port Conflicts

If ports are already in use:

```bash
# Check what's using ports
lsof -i :4222  # NATS
lsof -i :6379  # Redis
lsof -i :5672  # RabbitMQ
lsof -i :8080  # WebSocket
lsof -i :3000  # Webhook

# Stop conflicting services or change ports in docker-compose.yml
```

### Service Not Starting

```bash
# View service logs
docker-compose logs [service-name]

# Check service status
docker-compose ps

# Restart specific service
docker-compose restart [service-name]
```

### Connection Refused

```bash
# Ensure services are healthy
make check-services

# Wait for services to be ready
docker-compose ps

# Check health status
docker inspect watchtower-nats | grep Health
docker inspect watchtower-redis | grep Health
docker inspect watchtower-rabbitmq | grep Health
```

### Clean State Reset

```bash
# Nuclear option: remove everything
make clean-all

# Start fresh
make services-up
```

## Makefile Reference

### Service Commands

- `make services-up` - Start all services
- `make services-down` - Stop all services
- `make services-logs` - View service logs
- `make services-clean` - Stop and remove volumes
- `make services-restart` - Restart all services
- `make check-services` - Health check all services

### Build Commands

- `make build` - Build all crates
- `make build-release` - Build release mode
- `make clean` - Clean build artifacts

### Test Commands

- `make test` - Run unit tests
- `make test-integration` - Run integration tests
- `make test-all` - Run all tests

### Example Commands

- `make example-nats` - Run NATS example
- `make example-redis` - Run Redis example
- `make example-rabbitmq` - Run RabbitMQ example
- `make example-websocket` - Run WebSocket example
- `make example-webhook` - Run Webhook example
- `make example-circuit-breaker` - Run circuit breaker example
- `make example-backpressure` - Run backpressure example

### Quality Commands

- `make fmt` - Format code
- `make clippy` - Run clippy lints
- `make check` - Run fmt + clippy
- `make docs` - Generate documentation

### Docker Commands

- `make docker-build-websocket` - Build WebSocket server image
- `make docker-build-webhook` - Build webhook receiver image
- `make docker-build-all` - Build all Docker images

## CI/CD Integration

### GitHub Actions Example

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Start services
        run: make services-up

      - name: Wait for services
        run: sleep 10

      - name: Run tests
        run: make test-all

      - name: Stop services
        run: make services-down
```

## Production Deployment

For production use:

1. **Use managed services** instead of Docker Compose:
   - NATS: NATS Cloud or self-hosted cluster
   - Redis: Redis Enterprise, AWS ElastiCache, etc.
   - RabbitMQ: CloudAMQP, AWS MQ, etc.

2. **Remove test servers**:
   - WebSocket and Webhook servers are for testing only
   - Use your own production endpoints

3. **Configure TLS/SSL**:
   - Enable encryption for all services
   - Use proper certificates

4. **Set authentication**:
   - Change default credentials
   - Use strong passwords/tokens
   - Configure ACLs and permissions

5. **Monitor and scale**:
   - Set up monitoring (Prometheus, Grafana)
   - Configure auto-scaling
   - Set up backups for persistent data
