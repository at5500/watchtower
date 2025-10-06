.PHONY: help build test clean services services-up services-down services-logs examples fmt clippy docs

# Default target
help:
	@echo "Watchtower - Development Commands"
	@echo ""
	@echo "Services:"
	@echo "  make services-up      - Start all required services (NATS, Redis, RabbitMQ, etc.)"
	@echo "  make services-down    - Stop all services"
	@echo "  make services-logs    - View service logs"
	@echo "  make services-clean   - Stop services and remove volumes"
	@echo ""
	@echo "Build & Test:"
	@echo "  make build                  - Build all crates"
	@echo "  make test                   - Run unit tests"
	@echo "  make test-integration       - Run all integration tests (requires services)"
	@echo "  make test-integration-nats  - Run NATS integration tests"
	@echo "  make test-integration-redis - Run Redis integration tests"
	@echo "  make test-all               - Run all tests"
	@echo ""
	@echo "Examples (Transport-specific):"
	@echo "  make example-nats           - Run NATS example"
	@echo "  make example-redis          - Run Redis Streams example"
	@echo "  make example-rabbitmq       - Run RabbitMQ example"
	@echo "  make example-websocket      - Run WebSocket example"
	@echo "  make example-webhook        - Run Webhook example"
	@echo ""
	@echo "Examples (Advanced patterns):"
	@echo "  make example-multi-transport- Run multi-transport integration"
	@echo "  make example-error-handling - Run error handling & DLQ example"
	@echo "  make example-circuit-breaker- Run circuit breaker example"
	@echo "  make example-backpressure   - Run backpressure example"
	@echo "  make examples-all           - Run all examples"
	@echo ""
	@echo "Quality:"
	@echo "  make fmt              - Format code"
	@echo "  make clippy           - Run clippy lints"
	@echo "  make check            - Run fmt + clippy"
	@echo "  make docs             - Generate documentation"
	@echo ""
	@echo "Cleanup:"
	@echo "  make clean            - Clean build artifacts"
	@echo "  make clean-all        - Clean everything (build + services)"

# Build targets
build:
	@echo "Building all crates..."
	cargo build

build-release:
	@echo "Building release..."
	cargo build --release

# Test targets
test:
	@echo "Running unit tests..."
	cargo test --lib

test-integration:
	@echo "Running integration tests..."
	@echo "Ensuring services are running..."
	@make services-up
	@sleep 5
	cargo test --test '*' -- --ignored --test-threads=1

test-integration-nats:
	@echo "Running NATS integration tests..."
	@make services-up
	cargo test --test nats_integration -- --ignored

test-integration-redis:
	@echo "Running Redis integration tests..."
	@make services-up
	cargo test --test redis_integration -- --ignored

test-integration-rabbitmq:
	@echo "Running RabbitMQ integration tests..."
	@make services-up
	cargo test --test rabbitmq_integration -- --ignored

test-integration-websocket:
	@echo "Running WebSocket integration tests..."
	@make services-up
	cargo test --test websocket_integration -- --ignored

test-integration-webhook:
	@echo "Running Webhook integration tests..."
	@make services-up
	cargo test --test webhook_integration -- --ignored

test-all: test test-integration

# Service management
services-up:
	@echo "Starting services..."
	docker-compose up -d
	@echo "Waiting for services to be healthy..."
	@sleep 10
	@docker-compose ps

services-down:
	@echo "Stopping services..."
	docker-compose down

services-logs:
	docker-compose logs -f

services-clean:
	@echo "Stopping services and removing volumes..."
	docker-compose down -v

services-restart:
	@make services-down
	@make services-up

# Service health checks
check-nats:
	@echo "Checking NATS..."
	@curl -s http://localhost:8222/healthz || echo "NATS not ready"

check-redis:
	@echo "Checking Redis..."
	@redis-cli ping || echo "Redis not ready"

check-rabbitmq:
	@echo "Checking RabbitMQ..."
	@curl -s -u guest:guest http://localhost:15672/api/health/checks/alarms || echo "RabbitMQ not ready"

check-services: check-nats check-redis check-rabbitmq
	@echo "All services checked"

# Example targets
example-nats:
	@echo "Running NATS example..."
	@make services-up
	cargo run --example nats_pubsub

example-redis:
	@echo "Running Redis Streams example..."
	@make services-up
	cargo run --example redis_streams

example-rabbitmq:
	@echo "Running RabbitMQ example..."
	@make services-up
	cargo run --example rabbitmq_dlx

example-websocket:
	@echo "Running WebSocket example..."
	@make services-up
	cargo run --example websocket_bidirectional

example-webhook:
	@echo "Running Webhook example..."
	@make services-up
	cargo run --example webhook_signatures

example-prelude:
	@echo "Running Prelude usage example..."
	cargo run --example prelude_usage

example-derive:
	@echo "Running Derive macro example..."
	cargo run --example derive_macro

example-multi-transport:
	@echo "Running Multi-transport integration example..."
	@make services-up
	cargo run --example multi_transport

example-error-handling:
	@echo "Running Error Handling & DLQ example..."
	@make services-up
	cargo run --example error_handling_dlq

example-circuit-breaker:
	@echo "Running Circuit Breaker example..."
	@make services-up
	cargo run --example circuit_breaker

example-backpressure:
	@echo "Running Backpressure example..."
	@make services-up
	cargo run --example backpressure

examples-all:
	@echo "Running all examples..."
	@make example-nats
	@make example-redis
	@make example-rabbitmq
	@make example-websocket
	@make example-webhook
	@make example-multi-transport
	@make example-error-handling
	@make example-circuit-breaker
	@make example-backpressure

# Code quality
fmt:
	@echo "Formatting code..."
	cargo fmt --all

clippy:
	@echo "Running clippy..."
	cargo clippy --all-targets --all-features -- -D warnings

check: fmt clippy
	@echo "Code quality check complete"

# Documentation
docs:
	@echo "Generating documentation..."
	cargo doc --no-deps --all-features --open

docs-private:
	@echo "Generating documentation (including private items)..."
	cargo doc --no-deps --all-features --document-private-items --open

# Benchmarks
bench:
	@echo "Running benchmarks..."
	cargo bench

# Cleanup
clean:
	@echo "Cleaning build artifacts..."
	cargo clean

clean-all: clean services-clean
	@echo "Cleaned everything"

# Watch mode (requires cargo-watch)
watch:
	@echo "Running tests in watch mode..."
	cargo watch -x test

watch-example:
	@echo "Running example in watch mode..."
	cargo watch -x 'run --example $(EXAMPLE)'

# Install dev tools
install-tools:
	@echo "Installing development tools..."
	cargo install cargo-watch
	cargo install cargo-edit
	cargo install cargo-outdated

# Dependency management
deps-update:
	@echo "Updating dependencies..."
	cargo update

deps-outdated:
	@echo "Checking for outdated dependencies..."
	cargo outdated

# Release preparation
release-check: clean build-release test-all clippy
	@echo "Release checks complete"

# Docker image build (for examples)
docker-build-websocket:
	@echo "Building WebSocket server image..."
	docker build -f docker/Dockerfile.websocket -t watchtower-websocket .

docker-build-webhook:
	@echo "Building Webhook receiver image..."
	docker build -f docker/Dockerfile.webhook -t watchtower-webhook .

docker-build-all: docker-build-websocket docker-build-webhook
	@echo "All Docker images built"
