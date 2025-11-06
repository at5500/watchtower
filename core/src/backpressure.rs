//! Backpressure mechanism for event flow control

use crate::config::BackpressureStrategy;
use crate::event::Event;
use crate::errors::WatchtowerError;
#[cfg(feature = "debug-logging")]
use crate::debug_log;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::warn;

/// Backpressure statistics
#[derive(Debug, Clone, Default)]
pub struct BackpressureStats {
    /// Total events received
    pub total_received: u64,
    /// Events dropped due to backpressure
    pub events_dropped: u64,
    /// Current queue size
    pub current_queue_size: usize,
    /// Maximum queue size configured
    pub max_queue_size: usize,
}

impl BackpressureStats {
    /// Get queue usage percentage (0.0-1.0)
    pub fn queue_usage(&self) -> f32 {
        if self.max_queue_size == 0 {
            0.0
        } else {
            self.current_queue_size as f32 / self.max_queue_size as f32
        }
    }

    /// Check if queue usage is above threshold
    pub fn is_above_threshold(&self, threshold: f32) -> bool {
        self.queue_usage() > threshold
    }
}

/// Backpressure controller for managing event flow
pub struct BackpressureController {
    sender: mpsc::Sender<Event>,
    receiver: Arc<RwLock<mpsc::Receiver<Event>>>,
    stats: Arc<RwLock<BackpressureStats>>,
    strategy: BackpressureStrategy,
    warning_threshold: f32,
    // For DropOldest strategy: manual queue management
    manual_queue: Option<Arc<RwLock<VecDeque<Event>>>>,
    max_queue_size: usize,
}

impl BackpressureController {
    /// Create a new backpressure controller
    pub fn new(
        max_queue_size: usize,
        strategy: BackpressureStrategy,
        warning_threshold: f32,
    ) -> Self {
        let actual_max = if max_queue_size > 0 {
            max_queue_size
        } else {
            1000 // Default size for unlimited
        };

        let (sender, receiver) = mpsc::channel(actual_max);

        let stats = Arc::new(RwLock::new(BackpressureStats {
            total_received: 0,
            events_dropped: 0,
            current_queue_size: 0,
            max_queue_size: actual_max,
        }));

        // For DropOldest, use manual queue management
        let manual_queue = if matches!(strategy, BackpressureStrategy::DropOldest) {
            Some(Arc::new(RwLock::new(VecDeque::with_capacity(actual_max))))
        } else {
            None
        };

        Self {
            sender,
            receiver: Arc::new(RwLock::new(receiver)),
            stats,
            strategy,
            warning_threshold,
            manual_queue,
            max_queue_size: actual_max,
        }
    }

    /// Send an event through backpressure controller
    pub async fn send(&self, event: Event) -> Result<(), WatchtowerError> {
        let mut stats = self.stats.write().await;
        stats.total_received += 1;

        // Check threshold and warn if needed
        if stats.is_above_threshold(self.warning_threshold) {
            warn!(
                current_size = stats.current_queue_size,
                max_size = stats.max_queue_size,
                usage_pct = stats.queue_usage() * 100.0,
                "Event queue usage above threshold"
            );
        }

        drop(stats); // Release lock before sending

        match self.strategy {
            BackpressureStrategy::Block => {
                // Block until space is available
                self.sender
                    .send(event)
                    .await
                    .map_err(|_| WatchtowerError::InternalError("Channel closed".to_string()))?;

                let mut stats = self.stats.write().await;
                stats.current_queue_size = stats.current_queue_size.saturating_add(1);

                Ok(())
            }
            BackpressureStrategy::DropNewest => {
                // Try to send, drop if full
                match self.sender.try_send(event) {
                    Ok(_) => {
                        let mut stats = self.stats.write().await;
                        stats.current_queue_size = stats.current_queue_size.saturating_add(1);
                        Ok(())
                    }
                    Err(mpsc::error::TrySendError::Full(_dropped_event)) => {
                        let mut stats = self.stats.write().await;
                        stats.events_dropped += 1;

                        #[cfg(feature = "debug-logging")]
                        debug_log!(
                            event_id = %_dropped_event.id(),
                            event_type = %_dropped_event.event_type(),
                            "Event dropped due to backpressure (DropNewest strategy)"
                        );

                        Err(WatchtowerError::PublicationError(
                            "Event dropped due to backpressure".to_string(),
                        ))
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        Err(WatchtowerError::InternalError("Channel closed".to_string()))
                    }
                }
            }
            BackpressureStrategy::DropOldest => {
                // Use manual queue management to drop oldest when full
                if let Some(manual_queue) = &self.manual_queue {
                    let mut queue = manual_queue.write().await;

                    // If queue is full, drop the oldest event
                    if queue.len() >= self.max_queue_size {
                        if let Some(_dropped_event) = queue.pop_front() {
                            let mut stats = self.stats.write().await;
                            stats.events_dropped += 1;

                            #[cfg(feature = "debug-logging")]
                            debug_log!(
                                event_id = %_dropped_event.id(),
                                event_type = %_dropped_event.event_type(),
                                "Dropped oldest event due to backpressure (DropOldest strategy)"
                            );
                        }
                    }

                    // Add new event to the back
                    queue.push_back(event);

                    let mut stats = self.stats.write().await;
                    stats.current_queue_size = queue.len();

                    Ok(())
                } else {
                    // Fallback if manual_queue not initialized (shouldn't happen)
                    warn!("DropOldest manual queue not initialized, using Block fallback");
                    self.sender
                        .send(event)
                        .await
                        .map_err(|_| WatchtowerError::InternalError("Channel closed".to_string()))?;

                    let mut stats = self.stats.write().await;
                    stats.current_queue_size = stats.current_queue_size.saturating_add(1);

                    Ok(())
                }
            }
        }
    }

    /// Receive an event from the queue
    pub async fn receive(&self) -> Option<Event> {
        // For DropOldest strategy, use manual queue
        if let Some(manual_queue) = &self.manual_queue {
            let mut queue = manual_queue.write().await;
            let event = queue.pop_front();

            if event.is_some() {
                let mut stats = self.stats.write().await;
                stats.current_queue_size = queue.len();
            }

            event
        } else {
            // For other strategies, use mpsc channel
            let mut receiver = self.receiver.write().await;
            let event = receiver.recv().await;

            if event.is_some() {
                let mut stats = self.stats.write().await;
                stats.current_queue_size = stats.current_queue_size.saturating_sub(1);
            }

            event
        }
    }

    /// Get current statistics
    pub async fn stats(&self) -> BackpressureStats {
        self.stats.read().await.clone()
    }

    /// Get a clone of the sender for direct use
    pub fn sender(&self) -> mpsc::Sender<Event> {
        self.sender.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_backpressure_block_strategy() {
        let controller = BackpressureController::new(2, BackpressureStrategy::Block, 0.8);

        let event1 = Event::new("test.event", json!({"data": 1}));
        let event2 = Event::new("test.event", json!({"data": 2}));

        controller.send(event1).await.unwrap();
        controller.send(event2).await.unwrap();

        let stats = controller.stats().await;
        assert_eq!(stats.current_queue_size, 2);
        assert_eq!(stats.total_received, 2);
    }

    #[tokio::test]
    async fn test_backpressure_drop_newest() {
        let controller = BackpressureController::new(1, BackpressureStrategy::DropNewest, 0.8);

        let event1 = Event::new("test.event", json!({"data": 1}));
        let event2 = Event::new("test.event", json!({"data": 2}));

        controller.send(event1).await.unwrap();
        let result = controller.send(event2).await;

        assert!(result.is_err());

        let stats = controller.stats().await;
        assert_eq!(stats.events_dropped, 1);
        assert_eq!(stats.total_received, 2);
    }

    #[tokio::test]
    async fn test_queue_usage() {
        let stats = BackpressureStats {
            total_received: 100,
            events_dropped: 10,
            current_queue_size: 80,
            max_queue_size: 100,
        };

        assert_eq!(stats.queue_usage(), 0.8);
        assert!(stats.is_above_threshold(0.75));
        assert!(!stats.is_above_threshold(0.85));
    }
}
