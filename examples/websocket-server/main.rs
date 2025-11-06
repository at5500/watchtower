//! WebSocket Server Transport Example
//!
//! This example demonstrates how to use the WebSocket Server Transport
//! to broadcast events to connected WebSocket clients.
//!
//! Run this example:
//! ```bash
//! cargo run --example websocket-server
//! ```
//!
//! Then connect a WebSocket client:
//! ```bash
//! wscat -c ws://localhost:3030/ws
//! ```

use axum::{routing::get, Router};
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, Level};
use tracing_subscriber;

use watchtower_core::{Event, EventMetadata, Transport};
use watchtower_websocket_server::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting WebSocket Server Transport example");

    // Create WebSocket server transport
    let config = WebSocketServerConfig::default()
        .with_max_connections(100)
        .with_broadcast_buffer(500)
        .with_ping_interval(30);

    let transport = Arc::new(WebSocketServerTransport::new(config));

    info!("WebSocket server transport created");

    // Clone transport for event publishing task
    let transport_clone = transport.clone();

    // Spawn task to publish events periodically
    tokio::spawn(async move {
        let mut counter = 0;
        loop {
            sleep(Duration::from_secs(5)).await;

            counter += 1;

            // Create test event
            let event = Event::new(
                "test.event",
                json!({
                    "counter": counter,
                    "message": format!("Test event #{}", counter),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }),
                EventMetadata::default(),
            );

            info!(
                event_type = %event.event_type(),
                event_id = %event.id(),
                counter = counter,
                "Publishing test event"
            );

            // Broadcast event to all connected clients
            if let Err(e) = transport_clone.publish(event).await {
                tracing::error!(error = %e, "Failed to publish event");
            }
        }
    });

    // Create Axum router
    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .route("/health", get(health_handler))
        .with_state(transport.clone());

    info!("Router created with /ws and /health endpoints");

    // Start HTTP server
    let addr = "0.0.0.0:3030";
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("WebSocket server listening on: {}", addr);
    info!("Connect with: wscat -c ws://localhost:3030/ws");
    info!("Health check: curl http://localhost:3030/health");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
async fn health_handler(
    axum::extract::State(transport): axum::extract::State<Arc<WebSocketServerTransport>>,
) -> String {
    let active = transport.active_connections().await;
    let max = transport.config().max_connections;

    format!(
        "WebSocket Server OK\nActive connections: {}/{}\n",
        active, max
    )
}