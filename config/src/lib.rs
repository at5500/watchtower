//! Configuration module for Watchtower notification system
//!
//! This module only contains the unified NotificationConfig.
//! Individual transport configurations are in their respective modules.

use serde::{Deserialize, Serialize};

/// Unified notification system configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationConfig {
    /// Webhook configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook: Option<serde_json::Value>,

    /// NATS configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nats: Option<serde_json::Value>,

    /// Redis configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redis: Option<serde_json::Value>,
}
