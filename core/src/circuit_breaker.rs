//! Circuit breaker implementation for fault tolerance

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests pass through normally
    Closed,
    /// Circuit is open, requests are rejected immediately
    Open,
    /// Circuit is half-open, testing if service recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Duration to wait before attempting recovery (half-open state)
    pub timeout: Duration,
    /// Number of successful requests in half-open state before closing
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            timeout: Duration::from_secs(60),
            success_threshold: 2,
        }
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: Option<Instant>,
    pub total_requests: u64,
    pub rejected_requests: u64,
}

struct CircuitBreakerState {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    total_requests: u64,
    rejected_requests: u64,
}

/// Circuit breaker for protecting against cascading failures
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitBreakerState>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(CircuitBreakerState {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                total_requests: 0,
                rejected_requests: 0,
            })),
        }
    }

    /// Check if request should be allowed
    pub async fn should_allow_request(&self) -> bool {
        let mut state = self.state.write().await;
        state.total_requests += 1;

        match state.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                if let Some(last_failure) = state.last_failure_time {
                    if last_failure.elapsed() >= self.config.timeout {
                        // Transition to half-open
                        state.state = CircuitState::HalfOpen;
                        state.success_count = 0;
                        true
                    } else {
                        state.rejected_requests += 1;
                        false
                    }
                } else {
                    state.rejected_requests += 1;
                    false
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited requests to test recovery
                true
            }
        }
    }

    /// Record successful request
    pub async fn record_success(&self) {
        let mut state = self.state.write().await;

        match state.state {
            CircuitState::Closed => {
                // Reset failure count on success
                state.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                state.success_count += 1;
                if state.success_count >= self.config.success_threshold {
                    // Transition back to closed
                    state.state = CircuitState::Closed;
                    state.failure_count = 0;
                    state.success_count = 0;
                }
            }
            CircuitState::Open => {
                // Should not happen, but handle gracefully
            }
        }
    }

    /// Record failed request
    pub async fn record_failure(&self) {
        let mut state = self.state.write().await;
        state.failure_count += 1;
        state.last_failure_time = Some(Instant::now());

        match state.state {
            CircuitState::Closed => {
                if state.failure_count >= self.config.failure_threshold {
                    // Transition to open
                    state.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open state reopens the circuit
                state.state = CircuitState::Open;
                state.success_count = 0;
            }
            CircuitState::Open => {
                // Already open, just update failure time
            }
        }
    }

    /// Get current statistics
    pub async fn stats(&self) -> CircuitBreakerStats {
        let state = self.state.read().await;
        CircuitBreakerStats {
            state: state.state,
            failure_count: state.failure_count,
            success_count: state.success_count,
            last_failure_time: state.last_failure_time,
            total_requests: state.total_requests,
            rejected_requests: state.rejected_requests,
        }
    }

    /// Get current state
    pub async fn get_state(&self) -> CircuitState {
        let state = self.state.read().await;
        state.state
    }

    /// Manually reset circuit breaker to closed state
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        state.state = CircuitState::Closed;
        state.failure_count = 0;
        state.success_count = 0;
        state.last_failure_time = None;
    }

    /// Manually open circuit breaker
    pub async fn trip(&self) {
        let mut state = self.state.write().await;
        state.state = CircuitState::Open;
        state.last_failure_time = Some(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed_to_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            timeout: Duration::from_secs(1),
            success_threshold: 2,
        };
        let breaker = CircuitBreaker::new(config);

        // Should start in closed state
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        assert!(breaker.should_allow_request().await);

        // Record failures
        breaker.record_failure().await;
        breaker.record_failure().await;
        assert_eq!(breaker.get_state().await, CircuitState::Closed);

        breaker.record_failure().await;
        // Should now be open
        assert_eq!(breaker.get_state().await, CircuitState::Open);
        assert!(!breaker.should_allow_request().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_to_closed() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_millis(100),
            success_threshold: 2,
        };
        let breaker = CircuitBreaker::new(config);

        // Trip the breaker
        breaker.record_failure().await;
        breaker.record_failure().await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should transition to half-open
        assert!(breaker.should_allow_request().await);
        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);

        // Record successes
        breaker.record_success().await;
        breaker.record_success().await;

        // Should be closed now
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_to_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_millis(100),
            success_threshold: 2,
        };
        let breaker = CircuitBreaker::new(config);

        // Trip the breaker
        breaker.record_failure().await;
        breaker.record_failure().await;

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should transition to half-open
        assert!(breaker.should_allow_request().await);
        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);

        // Record failure - should reopen
        breaker.record_failure().await;
        assert_eq!(breaker.get_state().await, CircuitState::Open);
    }
}
