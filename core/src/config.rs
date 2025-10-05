//! Backpressure configuration for core module

use serde::{Deserialize, Serialize};

/// Backpressure strategy when the event queue is full
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackpressureStrategy {
    /// Drop the oldest event in the queue
    DropOldest,
    /// Drop the newest event (current one)
    DropNewest,
    /// Block until space is available (async wait)
    Block,
}

impl Default for BackpressureStrategy {
    fn default() -> Self {
        Self::DropOldest
    }
}

/// Backpressure configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureConfig {
    /// Maximum number of events in the queue (0 = unlimited)
    pub max_queue_size: usize,
    /// Strategy when queue is full
    pub strategy: BackpressureStrategy,
    /// Log warning when queue usage exceeds this threshold (0.0-1.0)
    pub warning_threshold: f32,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            strategy: BackpressureStrategy::DropOldest,
            warning_threshold: 0.8,
        }
    }
}
