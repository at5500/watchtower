# Watchtower Derive Macros

Procedural macros for the Watchtower event system that reduce boilerplate and provide compile-time guarantees.

## Overview

The `watchtower-derive` crate provides derive macros to automatically generate event conversions from your domain structs.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
watchtower-core = "0.1"
watchtower-derive = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

## Usage

### Basic Event Derive

```rust
use watchtower_derive::Event;
use serde::{Serialize, Deserialize};

#[derive(Event, Serialize, Deserialize)]
#[event(type = "user.created")]
struct UserCreated {
    user_id: u64,
    email: String,
    name: String,
}

// Automatic conversion to Event
let user_event = UserCreated {
    user_id: 123,
    email: "user@example.com".to_string(),
    name: "John Doe".to_string(),
};

// Convert to Event using .into()
let event: watchtower_core::Event = user_event.into();

// Or use .to_event()
let event = user_event.to_event();
```

### With Source Attribution

Track which service generated the event:

```rust
#[derive(Event, Serialize, Deserialize)]
#[event(type = "order.placed", source = "order-service")]
struct OrderPlaced {
    order_id: String,
    amount: f64,
}
```

The source will be automatically set in the event metadata.

### Event Type Method

Get the event type as a static string:

```rust
#[derive(Event, Serialize, Deserialize)]
#[event(type = "payment.processed")]
struct PaymentProcessed {
    payment_id: String,
    amount: f64,
}

// Get event type without creating an instance
let event_type = PaymentProcessed::event_type(); // "payment.processed"
```

## Attributes

### `#[event(...)]`

Required container attribute with the following parameters:

- `type` (required): Event type string
  ```rust
  #[event(type = "user.created")]
  ```

- `source` (optional): Source identifier (e.g., service name)
  ```rust
  #[event(type = "user.created", source = "user-service")]
  ```

## Generated Code

The `#[derive(Event)]` macro generates:

### 1. Into<Event> Implementation

Automatic conversion from your struct to `watchtower_core::Event`:

```rust
impl Into<watchtower_core::Event> for YourStruct {
    fn into(self) -> watchtower_core::Event {
        // Serialize struct to JSON
        // Create EventMetadata
        // Build Event
    }
}
```

### 2. Helper Methods

```rust
impl YourStruct {
    /// Convert this struct into an Event
    pub fn to_event(self) -> watchtower_core::Event { ... }

    /// Get the event type for this struct
    pub fn event_type() -> &'static str { ... }
}
```

## Benefits

### Type Safety

Event types are validated at compile time:

```rust
#[derive(Event, Serialize, Deserialize)]
#[event(type = "user.created")]
struct UserCreated { ... }

// Compiler knows event type is "user.created"
```

### Less Boilerplate

Before:
```rust
let event = Event::new(
    "user.created",
    serde_json::to_value(&user_data)?,
);
```

After:
```rust
let event = user_data.to_event();
```

### Compile-Time Guarantees

Missing event type:
```rust
#[derive(Event)]  // ❌ Compile error: missing #[event(type = "...")]
struct MyEvent { ... }
```

Invalid source:
```rust
#[derive(Event)]
#[event(type = "test", source = 123)]  // ❌ Compile error: expected string
struct MyEvent { ... }
```

## Complete Example

```rust
use watchtower_derive::Event;
use watchtower_core::prelude::*;
use watchtower_nats::prelude::*;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

#[derive(Event, Serialize, Deserialize)]
#[event(type = "user.registered", source = "auth-service")]
struct UserRegistered {
    user_id: u64,
    email: String,
    username: String,
    registered_at: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = NatsConfig::default();
    let mut subscriber = NatsSubscriber::new(config).await?;

    // Subscribe
    let handle = subscriber.subscribe(
        vec!["user.registered".to_string()],
        Arc::new(|event| Box::pin(async move {
            println!("Received: {:?}", event);
            Ok(())
        }))
    ).await?;

    // Create and publish
    let registration = UserRegistered {
        user_id: 12345,
        email: "user@example.com".to_string(),
        username: "johndoe".to_string(),
        registered_at: chrono::Utc::now().to_rfc3339(),
    };

    // Publish using derive macro
    subscriber.publish(registration.to_event()).await?;

    Ok(())
}
```

## Best Practices

### 1. Use Descriptive Event Types

```rust
// Good: Hierarchical, descriptive
#[event(type = "users.registered")]
#[event(type = "orders.payment.completed")]
#[event(type = "inventory.stock.updated")]

// Bad: Flat, unclear
#[event(type = "user_event")]
#[event(type = "order123")]
```

### 2. Always Derive Serialize/Deserialize

The `Event` derive requires your struct to be serializable:

```rust
#[derive(Event, Serialize, Deserialize)]  // ✅ Correct
#[event(type = "my.event")]
struct MyEvent { ... }

#[derive(Event)]  // ❌ Will fail: serde traits required
#[event(type = "my.event")]
struct MyEvent { ... }
```

### 3. Set Source for Multi-Service Systems

In microservices, track event origin:

```rust
#[derive(Event, Serialize, Deserialize)]
#[event(type = "order.created", source = "order-service")]
struct OrderCreated { ... }

#[derive(Event, Serialize, Deserialize)]
#[event(type = "payment.processed", source = "payment-service")]
struct PaymentProcessed { ... }
```

### 4. Use Static Event Type Method

```rust
// Get event type for subscription
let subscription_types = vec![
    UserCreated::event_type().to_string(),
    UserUpdated::event_type().to_string(),
];

subscriber.subscribe(subscription_types, callback).await?;
```

## Limitations

1. **Requires Serde**: Your struct must implement `Serialize` and `Deserialize`
2. **Static Event Types**: Event type must be a string literal, not computed
3. **Struct Only**: Currently only supports structs (not enums or unions)

## Related Documentation

- [Event System](../core/README.md) - Core event types
- [Examples](../examples/derive_macro.rs) - Complete working example
- [Main Documentation](../README.md) - Watchtower overview

## License

MIT - See [LICENSE](../LICENSE) for details
