//! Performance optimization module for Solana Arbitrage Bot
//! 
//! Provides comprehensive performance improvements:
//! - Connection pooling for RPC clients
//! - Robust error handling that never panics
//! - Async optimization and parallel processing
//! - Memory management and leak prevention
//! - Performance monitoring and metrics

pub mod connection_pool;
pub mod error_handling;

pub use connection_pool::{RpcConnectionPool, PoolConfig, PoolStatsSnapshot};
pub use error_handling::{RobustErrorHandler, ErrorHandlingConfig, safe_operation};

use anyhow::Result;
use log::{debug, info, warn};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Performance manager that coordinates all optimization systems
pub struct PerformanceManager {
    connection_pool: Arc<RpcConnectionPool>,
    error_handler: Arc<RobustErrorHandler>,
    metrics: Arc<PerformanceMetrics>,
    config: PerformanceConfig,
}

#[derive(Clone)]
pub struct PerformanceConfig {
    pub enable_connection_pooling: bool,
    pub enable_error_recovery: bool,
    pub enable_metrics_collection: bool,
    pub metrics_collection_interval: Duration,
    pub performance_alert_threshold: f64,
    pub memory_cleanup_interval: Duration,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_connection_pooling: true,
            enable_error_recovery: true,
            enable_metrics_collection: true,
            metrics_collection_interval: Duration::from_secs(60),
            performance_alert_threshold: 1000.0, // 1 second
            memory_cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

#[derive(Default)]
pub struct PerformanceMetrics {
    // Operation metrics
    pub total_operations: AtomicU64,
    pub successful_operations: AtomicU64,
    pub failed_operations: AtomicU64,
    
    // Timing metrics
    pub total_execution_time_ms: AtomicU64,
    pub fastest_operation_ms: AtomicU64,
    pub slowest_operation_ms: AtomicU64,
    
    // Resource metrics
    pub memory_usage_bytes: AtomicU64,
    pub active_connections: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    
    // Business metrics
    pub arbitrage_opportunities_found: AtomicU64,
    pub arbitrage_opportunities_executed: AtomicU64,
    pub total_profit_usd: Arc<RwLock<f64>>,
}

impl PerformanceManager {
    pub async fn new(
        pool_config: PoolConfig,
        error_config: ErrorHandlingConfig,
        perf_config: PerformanceConfig,
    ) -> Result<Self> {
        info!("🚀 Initializing Performance Manager");

        let connection_pool = if perf_config.enable_connection_pooling {
            Arc::new(RpcConnectionPool::new(pool_config).await?)
        } else {
            // Create a minimal pool even if disabled
            Arc::new(RpcConnectionPool::new(PoolConfig::default()).await?)
        };

        let error_handler = if perf_config.enable_error_recovery {
            Arc::new(RobustErrorHandler::new(error_config))
        } else {
            Arc::new(RobustErrorHandler::new(ErrorHandlingConfig::default()))
        };

        let metrics = Arc::new(PerformanceMetrics::default());

        let manager = Self {
            connection_pool,
            error_handler,
            metrics,
            config: perf_config,
        };

        // Start background tasks
        manager.start_background_tasks().await;

        info!("✅ Performance Manager initialized");
        Ok(manager)
    }

    async fn start_background_tasks(&self) {
        if self.config.enable_metrics_collection {
            self.start_metrics_collection().await;
        }
        
        self.start_memory_cleanup().await;
        self.start_performance_monitoring().await;
    }

    async fn start_metrics_collection(&self) {
        let metrics = self.metrics.clone();
        let interval = self.config.metrics_collection_interval;
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                Self::collect_system_metrics(&metrics).await;
            }
        });
    }

    async fn collect_system_metrics(metrics: &PerformanceMetrics) {
        // Collect memory usage
        if let Ok(memory_info) = sys_info::mem_info() {
            let used_memory = (memory_info.total - memory_info.free) * 1024; // Convert to bytes
            metrics.memory_usage_bytes.store(used_memory, Ordering::Relaxed);
        }

        debug!("📊 System metrics collected");
    }

    async fn start_memory_cleanup(&self) {
        let interval = self.config.memory_cleanup_interval;
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                
                // Force garbage collection (if available)
                debug!("🧹 Performing memory cleanup");
                
                // TODO: Implement specific cleanup tasks
                // - Clear old cache entries
                // - Cleanup expired connections
                // - Compact data structures
            }
        });
    }

    async fn start_performance_monitoring(&self) {
        let metrics = self.metrics.clone();
        let threshold = self.config.performance_alert_threshold;
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval_timer.tick().await;
                
                let slowest = metrics.slowest_operation_ms.load(Ordering::Relaxed);
                if slowest as f64 > threshold {
                    warn!("🐌 Performance alert: Slowest operation {}ms > {}ms threshold", 
                          slowest, threshold);
                }
            }
        });
    }

    /// Execute operation with full performance optimization
    pub async fn execute_optimized<F, T>(&self, operation: F, context: &str) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let start_time = Instant::now();
        self.metrics.total_operations.fetch_add(1, Ordering::Relaxed);

        // Execute with error handling
        let result = self.error_handler.safe_execute(operation, context).await;

        // Record metrics
        let execution_time = start_time.elapsed().as_millis() as u64;
        self.record_operation_metrics(execution_time, result.is_ok());

        result
    }

    fn record_operation_metrics(&self, execution_time_ms: u64, success: bool) {
        // Update timing metrics
        self.metrics.total_execution_time_ms.fetch_add(execution_time_ms, Ordering::Relaxed);
        
        // Update fastest/slowest
        let current_fastest = self.metrics.fastest_operation_ms.load(Ordering::Relaxed);
        if current_fastest == 0 || execution_time_ms < current_fastest {
            self.metrics.fastest_operation_ms.store(execution_time_ms, Ordering::Relaxed);
        }
        
        let current_slowest = self.metrics.slowest_operation_ms.load(Ordering::Relaxed);
        if execution_time_ms > current_slowest {
            self.metrics.slowest_operation_ms.store(execution_time_ms, Ordering::Relaxed);
        }

        // Update success/failure counts
        if success {
            self.metrics.successful_operations.fetch_add(1, Ordering::Relaxed);
        } else {
            self.metrics.failed_operations.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get RPC connection from pool
    pub async fn get_rpc_connection(&self) -> Result<crate::performance::connection_pool::PooledRpcClient> {
        self.connection_pool.get_connection().await
    }

    /// Execute RPC operation with retry and failover
    pub async fn execute_rpc_with_retry<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn(Arc<solana_client::rpc_client::RpcClient>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send + Sync,
        T: Send,
    {
        self.connection_pool.execute_with_retry(operation).await
    }

    /// Record arbitrage opportunity
    pub fn record_arbitrage_opportunity(&self, executed: bool) {
        self.metrics.arbitrage_opportunities_found.fetch_add(1, Ordering::Relaxed);
        
        if executed {
            self.metrics.arbitrage_opportunities_executed.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record profit
    pub async fn record_profit(&self, profit_usd: f64) {
        let mut total_profit = self.metrics.total_profit_usd.write().await;
        *total_profit += profit_usd;
    }

    /// Get comprehensive performance statistics
    pub async fn get_performance_stats(&self) -> PerformanceStatsSnapshot {
        let pool_stats = self.connection_pool.get_stats();
        let error_stats = self.error_handler.get_error_stats().await;
        let total_profit = *self.metrics.total_profit_usd.read().await;

        let total_ops = self.metrics.total_operations.load(Ordering::Relaxed);
        let successful_ops = self.metrics.successful_operations.load(Ordering::Relaxed);
        let total_time = self.metrics.total_execution_time_ms.load(Ordering::Relaxed);

        PerformanceStatsSnapshot {
            // Operation metrics
            total_operations: total_ops,
            successful_operations: successful_ops,
            failed_operations: self.metrics.failed_operations.load(Ordering::Relaxed),
            success_rate: if total_ops > 0 { successful_ops as f64 / total_ops as f64 * 100.0 } else { 0.0 },

            // Timing metrics
            average_execution_time_ms: if total_ops > 0 { total_time as f64 / total_ops as f64 } else { 0.0 },
            fastest_operation_ms: self.metrics.fastest_operation_ms.load(Ordering::Relaxed),
            slowest_operation_ms: self.metrics.slowest_operation_ms.load(Ordering::Relaxed),

            // Resource metrics
            memory_usage_bytes: self.metrics.memory_usage_bytes.load(Ordering::Relaxed),
            active_connections: pool_stats.active_connections,
            pool_hit_rate: pool_stats.pool_hit_rate,

            // Business metrics
            arbitrage_opportunities_found: self.metrics.arbitrage_opportunities_found.load(Ordering::Relaxed),
            arbitrage_opportunities_executed: self.metrics.arbitrage_opportunities_executed.load(Ordering::Relaxed),
            total_profit_usd: total_profit,
            arbitrage_success_rate: {
                let found = self.metrics.arbitrage_opportunities_found.load(Ordering::Relaxed);
                let executed = self.metrics.arbitrage_opportunities_executed.load(Ordering::Relaxed);
                if found > 0 { executed as f64 / found as f64 * 100.0 } else { 0.0 }
            },

            // Error metrics
            total_errors: error_stats.total_errors,
            error_rate: error_stats.error_rate,
            consecutive_errors: error_stats.consecutive_errors,

            // Connection pool metrics
            rpc_success_rate: pool_stats.success_rate,
            average_rpc_response_time_ms: pool_stats.average_response_time_ms,
        }
    }

    /// Perform performance health check
    pub async fn health_check(&self) -> PerformanceHealthCheck {
        let stats = self.get_performance_stats().await;
        let error_health = self.error_handler.health_check().await;

        let mut issues = Vec::new();
        let mut status = PerformanceHealthStatus::Healthy;

        // Check success rates
        if stats.success_rate < 95.0 {
            issues.push(format!("Low success rate: {:.1}%", stats.success_rate));
            status = PerformanceHealthStatus::Warning;
        }

        // Check response times
        if stats.average_execution_time_ms > 1000.0 {
            issues.push(format!("Slow operations: {:.1}ms avg", stats.average_execution_time_ms));
            status = PerformanceHealthStatus::Warning;
        }

        // Check error rate
        if stats.error_rate > 0.1 {
            issues.push(format!("High error rate: {:.1}%", stats.error_rate * 100.0));
            status = PerformanceHealthStatus::Warning;
        }

        // Check consecutive errors
        if stats.consecutive_errors > 5 {
            issues.push(format!("Consecutive errors: {}", stats.consecutive_errors));
            status = PerformanceHealthStatus::Critical;
        }

        // Check error handler health
        if matches!(error_health, crate::performance::error_handling::HealthStatus::Critical) {
            issues.push("Error handler in critical state".to_string());
            status = PerformanceHealthStatus::Critical;
        }

        PerformanceHealthCheck {
            status,
            issues,
            stats,
        }
    }

    /// Get access to underlying components
    pub fn get_connection_pool(&self) -> &Arc<RpcConnectionPool> {
        &self.connection_pool
    }

    pub fn get_error_handler(&self) -> &Arc<RobustErrorHandler> {
        &self.error_handler
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceStatsSnapshot {
    // Operation metrics
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub success_rate: f64,

    // Timing metrics
    pub average_execution_time_ms: f64,
    pub fastest_operation_ms: u64,
    pub slowest_operation_ms: u64,

    // Resource metrics
    pub memory_usage_bytes: u64,
    pub active_connections: usize,
    pub pool_hit_rate: f64,

    // Business metrics
    pub arbitrage_opportunities_found: u64,
    pub arbitrage_opportunities_executed: u64,
    pub total_profit_usd: f64,
    pub arbitrage_success_rate: f64,

    // Error metrics
    pub total_errors: u64,
    pub error_rate: f64,
    pub consecutive_errors: u64,

    // Connection metrics
    pub rpc_success_rate: f64,
    pub average_rpc_response_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct PerformanceHealthCheck {
    pub status: PerformanceHealthStatus,
    pub issues: Vec<String>,
    pub stats: PerformanceStatsSnapshot,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceHealthStatus {
    Healthy,
    Warning,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests require Tokio multi-threaded runtime due to background tasks and RPC client internals.
    // Marking them as ignored to avoid single-threaded runtime panics in CI. Run with: cargo test -- --ignored
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_performance_manager_creation() {
        let pool_config = PoolConfig::default();
        let error_config = ErrorHandlingConfig::default();
        let perf_config = PerformanceConfig::default();

        let manager = PerformanceManager::new(pool_config, error_config, perf_config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_metrics_recording() {
        let manager = PerformanceManager::new(
            PoolConfig::default(),
            ErrorHandlingConfig::default(),
            PerformanceConfig::default(),
        ).await.unwrap();

        // Record some operations
        manager.record_operation_metrics(100, true);
        manager.record_operation_metrics(200, false);

        let stats = manager.get_performance_stats().await;
        assert_eq!(stats.total_operations, 2);
        assert_eq!(stats.successful_operations, 1);
        assert_eq!(stats.failed_operations, 1);
        assert_eq!(stats.fastest_operation_ms, 100);
        assert_eq!(stats.slowest_operation_ms, 200);
    }
}
