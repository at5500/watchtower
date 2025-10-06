//! Simple webhook receiver for testing Watchtower Webhook transport

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use tracing::{error, info, warn};

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
struct AppState {
    secret: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WebhookEvent {
    id: String,
    event_type: String,
    payload: serde_json::Value,
    metadata: serde_json::Value,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

#[derive(Serialize)]
struct WebhookResponse {
    status: String,
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("webhook_receiver=info")
        .init();

    // Get secret from environment
    let secret = std::env::var("WEBHOOK_SECRET").ok();

    if let Some(ref s) = secret {
        info!("HMAC verification enabled with secret");
    } else {
        warn!("HMAC verification disabled (no WEBHOOK_SECRET)");
    }

    let state = Arc::new(AppState { secret });

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/webhook", post(handle_webhook))
        .route("/webhooks/events", post(handle_webhook))
        .with_state(state);

    // Start server
    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Webhook receiver listening on: {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookResponse>, StatusCode> {
    // Verify HMAC signature if secret is configured
    if let Some(ref secret) = state.secret {
        if let Some(signature) = headers.get("x-webhook-signature") {
            let signature_str = signature.to_str().map_err(|e| {
                error!("Invalid signature header: {}", e);
                StatusCode::BAD_REQUEST
            })?;

            if !verify_signature(&body, signature_str, secret) {
                warn!("HMAC verification failed");
                return Err(StatusCode::UNAUTHORIZED);
            }

            info!("HMAC verification successful");
        } else {
            warn!("Missing X-Webhook-Signature header");
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // Parse webhook event
    let event: WebhookEvent = serde_json::from_slice(&body).map_err(|e| {
        error!("Failed to parse webhook payload: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    info!(
        "Received webhook event: id={}, type={}",
        event.id, event.event_type
    );
    info!("Payload: {}", serde_json::to_string_pretty(&event.payload).unwrap());

    // Process event (echo back)
    Ok(Json(WebhookResponse {
        status: "ok".to_string(),
        message: format!("Event {} received", event.id),
    }))
}

fn verify_signature(payload: &[u8], signature: &str, secret: &str) -> bool {
    // Remove "sha256=" prefix if present
    let signature_hex = signature.strip_prefix("sha256=").unwrap_or(signature);

    // Compute HMAC
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC key error");
    mac.update(payload);

    // Get expected signature
    let expected = hex::encode(mac.finalize().into_bytes());

    // Constant-time comparison
    expected == signature_hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_signature() {
        let payload = b"{\"test\":\"data\"}";
        let secret = "test-secret";

        // Compute signature
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        // Verify
        assert!(verify_signature(payload, &signature, secret));
    }

    #[test]
    fn test_verify_signature_invalid() {
        let payload = b"{\"test\":\"data\"}";
        let secret = "test-secret";
        let wrong_signature = "sha256=invalidhex";

        assert!(!verify_signature(payload, wrong_signature, secret));
    }
}
