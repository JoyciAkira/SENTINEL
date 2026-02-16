//! Circuit Breaker Pattern for LLM Providers
//!
//! Prevents cascade failures by stopping requests to failing providers.
//! Inspired by Netflix Hystrix pattern.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed,
    /// Failure threshold reached - requests fail fast
    Open,
    /// Testing if service recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Duration to wait before attempting recovery (half-open)
    pub recovery_timeout: Duration,
    /// Success threshold to close circuit from half-open
    pub success_threshold: u32,
    /// Percentage of requests to sample in half-open state
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            success_threshold: 3,
            half_open_max_requests: 1,
        }
    }
}

/// Circuit breaker for a single provider
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    half_open_requests: Arc<RwLock<u32>>,
    provider_name: String,
}

impl CircuitBreaker {
    /// Create new circuit breaker
    pub fn new(provider_name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            half_open_requests: Arc::new(RwLock::new(0)),
            provider_name: provider_name.into(),
        }
    }

    /// Check if request should be allowed
    pub async fn can_execute(&self) -> Result<()> {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => {
                // Normal operation - allow request
                Ok(())
            }
            CircuitState::Open => {
                // Check if recovery timeout has passed
                let last_failure = *self.last_failure_time.read().await;

                if let Some(last) = last_failure {
                    if last.elapsed() >= self.config.recovery_timeout {
                        // Transition to half-open
                        tracing::info!(
                            "Circuit breaker for {} transitioning to half-open",
                            self.provider_name
                        );
                        *self.state.write().await = CircuitState::HalfOpen;
                        *self.success_count.write().await = 0;
                        *self.half_open_requests.write().await = 0;
                        Ok(())
                    } else {
                        // Still in recovery period
                        let remaining = self.config.recovery_timeout - last.elapsed();
                        Err(anyhow!(
                            "Circuit breaker OPEN for {} - retry after {:?}",
                            self.provider_name,
                            remaining
                        ))
                    }
                } else {
                    Err(anyhow!("Circuit breaker OPEN for {}", self.provider_name))
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited requests to test recovery
                let mut half_open_reqs = self.half_open_requests.write().await;

                if *half_open_reqs < self.config.half_open_max_requests {
                    *half_open_reqs += 1;
                    Ok(())
                } else {
                    Err(anyhow!(
                        "Circuit breaker HALF-OPEN for {} - max test requests reached",
                        self.provider_name
                    ))
                }
            }
        }
    }

    /// Record successful execution
    pub async fn record_success(&self) {
        let state = *self.state.read().await;

        match state {
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;

                if *success_count >= self.config.success_threshold {
                    // Transition to closed
                    tracing::info!(
                        "Circuit breaker for {} closed - service recovered",
                        self.provider_name
                    );
                    *self.state.write().await = CircuitState::Closed;
                    *self.failure_count.write().await = 0;
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                let mut failure_count = self.failure_count.write().await;
                if *failure_count > 0 {
                    *failure_count = 0;
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but reset
                *self.failure_count.write().await = 0;
            }
        }
    }

    /// Record failed execution
    pub async fn record_failure(&self) {
        let mut failure_count = self.failure_count.write().await;
        *failure_count += 1;

        *self.last_failure_time.write().await = Some(Instant::now());

        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => {
                if *failure_count >= self.config.failure_threshold {
                    // Transition to open
                    tracing::warn!(
                        "Circuit breaker for {} opened after {} failures",
                        self.provider_name,
                        *failure_count
                    );
                    *self.state.write().await = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                // Failure in half-open -> back to open
                tracing::warn!(
                    "Circuit breaker for {} back to OPEN - recovery failed",
                    self.provider_name
                );
                *self.state.write().await = CircuitState::Open;
                *self.success_count.write().await = 0;
            }
            CircuitState::Open => {
                // Already open, just update last failure time
            }
        }
    }

    /// Get current state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// Get metrics
    pub async fn metrics(&self) -> CircuitBreakerMetrics {
        CircuitBreakerMetrics {
            state: *self.state.read().await,
            failure_count: *self.failure_count.read().await,
            success_count: *self.success_count.read().await,
            last_failure_time: *self.last_failure_time.read().await,
        }
    }

    /// Force open (for testing or manual intervention)
    pub async fn force_open(&self) {
        tracing::warn!("Circuit breaker for {} manually opened", self.provider_name);
        *self.state.write().await = CircuitState::Open;
        *self.last_failure_time.write().await = Some(Instant::now());
    }

    /// Force close (for testing or manual intervention)
    pub async fn force_close(&self) {
        tracing::info!("Circuit breaker for {} manually closed", self.provider_name);
        *self.state.write().await = CircuitState::Closed;
        *self.failure_count.write().await = 0;
        *self.success_count.write().await = 0;
    }
}

/// Circuit breaker metrics
#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: Option<Instant>,
}

/// Circuit breaker registry for multiple providers
pub struct CircuitBreakerRegistry {
    breakers: Arc<RwLock<std::collections::HashMap<String, Arc<CircuitBreaker>>>>,
    default_config: CircuitBreakerConfig,
}

impl CircuitBreakerRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            breakers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            default_config: CircuitBreakerConfig::default(),
        }
    }

    /// Get or create circuit breaker for provider
    pub async fn get_or_create(&self, provider_name: &str) -> Arc<CircuitBreaker> {
        let mut breakers = self.breakers.write().await;

        if let Some(cb) = breakers.get(provider_name) {
            cb.clone()
        } else {
            let cb = Arc::new(CircuitBreaker::new(
                provider_name,
                self.default_config.clone(),
            ));
            breakers.insert(provider_name.to_string(), cb.clone());
            cb
        }
    }

    /// Get circuit breaker for provider (if exists)
    pub async fn get(&self, provider_name: &str) -> Option<Arc<CircuitBreaker>> {
        self.breakers.read().await.get(provider_name).cloned()
    }

    /// Get all metrics
    pub async fn all_metrics(&self) -> Vec<(String, CircuitBreakerMetrics)> {
        let breakers = self.breakers.read().await;
        let mut metrics = Vec::new();

        for (name, cb) in breakers.iter() {
            metrics.push((name.clone(), cb.metrics().await));
        }

        metrics
    }
}

impl Default for CircuitBreakerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_lifecycle() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_millis(100),
            success_threshold: 2,
            half_open_max_requests: 1,
        };

        let cb = CircuitBreaker::new("test", config);

        // Initial state: Closed
        assert_eq!(cb.state().await, CircuitState::Closed);
        assert!(cb.can_execute().await.is_ok());

        // Record failures to open circuit
        for _ in 0..3 {
            cb.record_failure().await;
        }

        // Circuit should be open now
        assert_eq!(cb.state().await, CircuitState::Open);
        assert!(cb.can_execute().await.is_err());

        // Wait for recovery timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should transition to half-open
        assert!(cb.can_execute().await.is_ok());
        assert_eq!(cb.state().await, CircuitState::HalfOpen);

        // Record successes to close circuit
        for _ in 0..2 {
            cb.record_success().await;
        }

        // Circuit should be closed
        assert_eq!(cb.state().await, CircuitState::Closed);
        assert!(cb.can_execute().await.is_ok());
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_failure() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout: Duration::from_millis(50),
            success_threshold: 2,
            half_open_max_requests: 1,
        };

        let cb = CircuitBreaker::new("test", config);

        // Open circuit
        for _ in 0..2 {
            cb.record_failure().await;
        }
        assert_eq!(cb.state().await, CircuitState::Open);

        // Wait for recovery
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Transition to half-open
        assert!(cb.can_execute().await.is_ok());
        assert_eq!(cb.state().await, CircuitState::HalfOpen);

        // Failure in half-open -> back to open
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_registry() {
        let registry = CircuitBreakerRegistry::new();

        // Get breaker for provider
        let cb1 = registry.get_or_create("openai").await;
        let cb2 = registry.get_or_create("openai").await;

        // Should be same instance
        assert!(Arc::ptr_eq(&cb1, &cb2));

        // Different provider should be different instance
        let cb3 = registry.get_or_create("anthropic").await;
        assert!(!Arc::ptr_eq(&cb1, &cb3));
    }
}
