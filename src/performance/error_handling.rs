use anyhow::{anyhow, Result};
use futures_util::FutureExt;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;

/// Production-safe error handling system that never panics
#[derive(Debug, Clone)]
pub struct RobustErrorHandler {
    error_stats: Arc<ErrorStats>,
    recovery_strategies: Arc<RwLock<HashMap<ErrorType, RecoveryStrategy>>>,
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    config: ErrorHandlingConfig,
}

#[derive(Debug, Clone)]
pub struct ErrorHandlingConfig {
    pub max_consecutive_errors: u32,
    pub error_rate_threshold: f64, // 0.0 to 1.0
    pub circuit_breaker_timeout: Duration,
    pub enable_graceful_degradation: bool,
    pub log_all_errors: bool,
    pub alert_on_critical_errors: bool,
}

impl Default for ErrorHandlingConfig {
    fn default() -> Self {
        Self {
            max_consecutive_errors: 5,
            error_rate_threshold: 0.1, // 10% error rate
            circuit_breaker_timeout: Duration::from_secs(60),
            enable_graceful_degradation: true,
            log_all_errors: true,
            alert_on_critical_errors: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorType {
    RpcConnection,
    RpcTimeout,
    InsufficientFunds,
    SlippageTooHigh,
    TransactionFailed,
    NetworkError,
    ParseError,
    ConfigurationError,
    SecurityViolation,
    Unknown,
}

#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    Retry { max_attempts: u32, delay: Duration },
    Fallback { alternative_action: String },
    GracefulDegradation { reduced_functionality: String },
    EmergencyStop { reason: String },
    Ignore,
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: CircuitBreakerState,
    failure_count: u32,
    last_failure_time: Option<Instant>,
    success_count: u32,
    timeout: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,   // Normal operation
    Open,     // Failing, blocking requests
    HalfOpen, // Testing if service recovered
}

#[derive(Debug, Default)]
pub struct ErrorStats {
    pub total_errors: AtomicU64,
    pub errors_by_type: Arc<RwLock<HashMap<ErrorType, u64>>>,
    pub consecutive_errors: AtomicU64,
    pub last_error_time: Arc<RwLock<Option<SystemTime>>>,
    pub recovery_attempts: AtomicU64,
    pub successful_recoveries: AtomicU64,
}

impl RobustErrorHandler {
    pub fn new(config: ErrorHandlingConfig) -> Self {
        let mut recovery_strategies = HashMap::new();
        
        // Default recovery strategies
        recovery_strategies.insert(
            ErrorType::RpcConnection,
            RecoveryStrategy::Retry {
                max_attempts: 3,
                delay: Duration::from_millis(1000),
            },
        );
        
        recovery_strategies.insert(
            ErrorType::RpcTimeout,
            RecoveryStrategy::Fallback {
                alternative_action: "Use backup RPC endpoint".to_string(),
            },
        );
        
        recovery_strategies.insert(
            ErrorType::InsufficientFunds,
            RecoveryStrategy::GracefulDegradation {
                reduced_functionality: "Reduce position size".to_string(),
            },
        );
        
        recovery_strategies.insert(
            ErrorType::SlippageTooHigh,
            RecoveryStrategy::GracefulDegradation {
                reduced_functionality: "Skip this opportunity".to_string(),
            },
        );
        
        recovery_strategies.insert(
            ErrorType::SecurityViolation,
            RecoveryStrategy::EmergencyStop {
                reason: "Security violation detected".to_string(),
            },
        );

        Self {
            error_stats: Arc::new(ErrorStats::default()),
            recovery_strategies: Arc::new(RwLock::new(recovery_strategies)),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Handle error with automatic recovery strategy
    pub async fn handle_error<T>(&self, error: &anyhow::Error, context: &str) -> Result<Option<T>> {
        let error_type = self.classify_error(error);
        
        // Record error statistics
        self.record_error(&error_type).await;
        
        // Log error safely (never panic)
        self.safe_log_error(error, context, &error_type);
        
        // Check circuit breaker
        if self.should_circuit_break(context, &error_type).await {
            warn!("🔴 Circuit breaker activated for {}", context);
            return Err(anyhow!("Circuit breaker open for {}", context));
        }
        
        // Apply recovery strategy
        match self.get_recovery_strategy(&error_type).await {
            Some(strategy) => self.apply_recovery_strategy(strategy, error, context).await,
            None => {
                warn!("No recovery strategy for error type: {:?}", error_type);
                Err(anyhow!("No recovery strategy available"))
            }
        }
    }

    /// Safe wrapper that never panics
    pub async fn safe_execute<F, T>(&self, operation: F, context: &str) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        // Wrap operation in panic-safe execution
        let result = std::panic::AssertUnwindSafe(operation).catch_unwind().await;
        
        match result {
            Ok(Ok(value)) => {
                // Success - reset consecutive error count
                self.error_stats.consecutive_errors.store(0, Ordering::Relaxed);
                Ok(value)
            }
            Ok(Err(e)) => {
                // Handled error
                self.handle_error::<T>(&e, context).await?;
                Err(e)
            }
            Err(panic_payload) => {
                // Panic caught - convert to error
                let panic_msg = if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic".to_string()
                };
                
                error!("🚨 PANIC CAUGHT in {}: {}", context, panic_msg);
                
                // Record as critical error
                self.record_error(&ErrorType::Unknown).await;
                
                Err(anyhow!("Panic caught in {}: {}", context, panic_msg))
            }
        }
    }

    fn classify_error(&self, error: &anyhow::Error) -> ErrorType {
        let error_str = error.to_string().to_lowercase();
        
        if error_str.contains("connection") || error_str.contains("network") {
            ErrorType::RpcConnection
        } else if error_str.contains("timeout") {
            ErrorType::RpcTimeout
        } else if error_str.contains("insufficient") || error_str.contains("balance") {
            ErrorType::InsufficientFunds
        } else if error_str.contains("slippage") {
            ErrorType::SlippageTooHigh
        } else if error_str.contains("transaction") && error_str.contains("failed") {
            ErrorType::TransactionFailed
        } else if error_str.contains("parse") || error_str.contains("deserialize") {
            ErrorType::ParseError
        } else if error_str.contains("config") {
            ErrorType::ConfigurationError
        } else if error_str.contains("security") || error_str.contains("unauthorized") {
            ErrorType::SecurityViolation
        } else {
            ErrorType::Unknown
        }
    }

    async fn record_error(&self, error_type: &ErrorType) {
        self.error_stats.total_errors.fetch_add(1, Ordering::Relaxed);
        self.error_stats.consecutive_errors.fetch_add(1, Ordering::Relaxed);
        
        // Update error type statistics
        let mut errors_by_type = self.error_stats.errors_by_type.write().await;
        *errors_by_type.entry(error_type.clone()).or_insert(0) += 1;
        
        // Update last error time
        let mut last_error_time = self.error_stats.last_error_time.write().await;
        *last_error_time = Some(SystemTime::now());
    }

    fn safe_log_error(&self, error: &anyhow::Error, context: &str, error_type: &ErrorType) {
        if self.config.log_all_errors {
            // Use safe logging that won't panic
            let error_msg = format!("Error in {}: {} (Type: {:?})", context, error, error_type);
            
            // Truncate very long error messages to prevent log overflow
            let truncated_msg = if error_msg.len() > 1000 {
                format!("{}... [truncated]", &error_msg[..1000])
            } else {
                error_msg
            };
            
            error!("❌ {}", truncated_msg);
        }
        
        // Send alerts for critical errors
        if self.config.alert_on_critical_errors && self.is_critical_error(error_type) {
            // TODO: Implement alerting system
            error!("🚨 CRITICAL ERROR ALERT: {:?} in {}", error_type, context);
        }
    }

    fn is_critical_error(&self, error_type: &ErrorType) -> bool {
        matches!(
            error_type,
            ErrorType::SecurityViolation | ErrorType::ConfigurationError
        )
    }

    async fn should_circuit_break(&self, context: &str, error_type: &ErrorType) -> bool {
        let mut circuit_breakers = self.circuit_breakers.write().await;
        let circuit_breaker = circuit_breakers
            .entry(context.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.config.circuit_breaker_timeout));

        circuit_breaker.record_failure();
        
        match circuit_breaker.state {
            CircuitBreakerState::Open => true,
            CircuitBreakerState::HalfOpen => {
                // Allow one test request
                circuit_breaker.state = CircuitBreakerState::Closed;
                false
            }
            CircuitBreakerState::Closed => {
                if circuit_breaker.failure_count >= self.config.max_consecutive_errors {
                    circuit_breaker.state = CircuitBreakerState::Open;
                    circuit_breaker.last_failure_time = Some(Instant::now());
                    true
                } else {
                    false
                }
            }
        }
    }

    async fn get_recovery_strategy(&self, error_type: &ErrorType) -> Option<RecoveryStrategy> {
        let strategies = self.recovery_strategies.read().await;
        strategies.get(error_type).cloned()
    }

    async fn apply_recovery_strategy<T>(
        &self,
        strategy: RecoveryStrategy,
        error: &anyhow::Error,
        context: &str,
    ) -> Result<Option<T>> {
        self.error_stats.recovery_attempts.fetch_add(1, Ordering::Relaxed);
        
        match strategy {
            RecoveryStrategy::Retry { max_attempts, delay } => {
                info!("🔄 Applying retry strategy: {} attempts with {:?} delay", max_attempts, delay);
                // Return None to indicate retry should be attempted by caller
                Ok(None)
            }
            RecoveryStrategy::Fallback { alternative_action } => {
                info!("🔀 Applying fallback strategy: {}", alternative_action);
                // Implement fallback logic here
                Ok(None)
            }
            RecoveryStrategy::GracefulDegradation { reduced_functionality } => {
                warn!("⬇️ Graceful degradation: {}", reduced_functionality);
                // Continue with reduced functionality
                Ok(None)
            }
            RecoveryStrategy::EmergencyStop { reason } => {
                error!("🛑 Emergency stop triggered: {}", reason);
                Err(anyhow!("Emergency stop: {}", reason))
            }
            RecoveryStrategy::Ignore => {
                debug!("🤷 Ignoring error as per strategy");
                Ok(None)
            }
        }
    }

    /// Get error statistics
    pub async fn get_error_stats(&self) -> ErrorStatsSnapshot {
        let errors_by_type = self.error_stats.errors_by_type.read().await.clone();
        let last_error_time = *self.error_stats.last_error_time.read().await;
        
        ErrorStatsSnapshot {
            total_errors: self.error_stats.total_errors.load(Ordering::Relaxed),
            consecutive_errors: self.error_stats.consecutive_errors.load(Ordering::Relaxed),
            errors_by_type,
            last_error_time,
            recovery_attempts: self.error_stats.recovery_attempts.load(Ordering::Relaxed),
            successful_recoveries: self.error_stats.successful_recoveries.load(Ordering::Relaxed),
            error_rate: self.calculate_error_rate().await,
        }
    }

    async fn calculate_error_rate(&self) -> f64 {
        let total_errors = self.error_stats.total_errors.load(Ordering::Relaxed);
        let recovery_attempts = self.error_stats.recovery_attempts.load(Ordering::Relaxed);
        
        if recovery_attempts > 0 {
            total_errors as f64 / recovery_attempts as f64
        } else {
            0.0
        }
    }

    /// Health check for error handling system
    pub async fn health_check(&self) -> HealthStatus {
        let stats = self.get_error_stats().await;
        
        if stats.consecutive_errors > self.config.max_consecutive_errors as u64 {
            HealthStatus::Critical
        } else if stats.error_rate > self.config.error_rate_threshold {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        }
    }
}

impl CircuitBreaker {
    fn new(timeout: Duration) -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            last_failure_time: None,
            success_count: 0,
            timeout,
        }
    }

    fn record_failure(&mut self) {
        self.failure_count += 1;
        self.success_count = 0;
        
        // Check if we should transition to half-open
        if self.state == CircuitBreakerState::Open {
            if let Some(last_failure) = self.last_failure_time {
                if last_failure.elapsed() > self.timeout {
                    self.state = CircuitBreakerState::HalfOpen;
                }
            }
        }
    }

    fn record_success(&mut self) {
        self.success_count += 1;
        self.failure_count = 0;
        
        if self.state == CircuitBreakerState::HalfOpen {
            self.state = CircuitBreakerState::Closed;
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErrorStatsSnapshot {
    pub total_errors: u64,
    pub consecutive_errors: u64,
    pub errors_by_type: HashMap<ErrorType, u64>,
    pub last_error_time: Option<SystemTime>,
    pub recovery_attempts: u64,
    pub successful_recoveries: u64,
    pub error_rate: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

/// Safe wrapper for operations that might panic
pub async fn safe_operation<F, T>(operation: F, context: &str) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    match std::panic::AssertUnwindSafe(operation).catch_unwind().await {
        Ok(result) => result,
        Err(panic_payload) => {
            let panic_msg = if let Some(s) = panic_payload.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic_payload.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic".to_string()
            };
            
            error!("🚨 PANIC in {}: {}", context, panic_msg);
            Err(anyhow!("Panic in {}: {}", context, panic_msg))
        }
    }
}

/// Macro for safe unwrapping that logs instead of panicking
#[macro_export]
macro_rules! safe_unwrap {
    ($expr:expr, $context:expr) => {
        match $expr {
            Some(val) => val,
            None => {
                error!("❌ Safe unwrap failed in {}: {:?}", $context, stringify!($expr));
                return Err(anyhow::anyhow!("Safe unwrap failed in {}", $context));
            }
        }
    };
}

/// Macro for safe expect that logs instead of panicking
#[macro_export]
macro_rules! safe_expect {
    ($expr:expr, $msg:expr, $context:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => {
                error!("❌ Safe expect failed in {}: {} - Error: {}", $context, $msg, e);
                return Err(anyhow::anyhow!("Safe expect failed in {}: {}", $context, $msg));
            }
        }
    };
}
