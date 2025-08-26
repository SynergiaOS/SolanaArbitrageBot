//! Architecture module for Solana Arbitrage Bot
//! 
//! Provides solutions for common architectural problems:
//! - Race condition prevention with atomic operations
//! - Memory leak prevention with bounded collections
//! - Deadlock prevention with ordered locking
//! - Performance monitoring and optimization

pub mod atomic_state;
pub mod deadlock_free;

pub use atomic_state::{AtomicPriceState, PriceSnapshot, BoundedTradeHistory, TradeRecord};
pub use deadlock_free::{DeadlockFreeManager, LockFreeSharedState, DeadlockDetector};

use anyhow::Result;
use log::{error, info};
use std::sync::Arc;
use std::time::Duration;

/// Enhanced shared state that prevents race conditions and deadlocks
pub struct EnhancedSharedState {
    // Atomic price state prevents race conditions
    price_state: Arc<AtomicPriceState>,
    
    // Lock-free state for counters and simple data
    lock_free_state: Arc<LockFreeSharedState>,
    
    // Bounded trade history prevents memory leaks
    trade_history: Arc<BoundedTradeHistory>,
    
    // Deadlock prevention for complex operations
    deadlock_manager: Arc<DeadlockFreeManager>,
    
    // Deadlock detection and recovery
    deadlock_detector: Arc<DeadlockDetector>,
}

impl EnhancedSharedState {
    pub fn new() -> Self {
        Self {
            price_state: Arc::new(AtomicPriceState::new()),
            lock_free_state: Arc::new(LockFreeSharedState::new()),
            trade_history: Arc::new(BoundedTradeHistory::new(1000)), // Max 1000 trades
            deadlock_manager: Arc::new(DeadlockFreeManager::new(5)), // 5 second timeout
            deadlock_detector: Arc::new(DeadlockDetector::new(1000, 5000)), // Check every 1s, max 5s operations
        }
    }

    /// Initialize the enhanced state system
    pub async fn initialize(&self) -> Result<()> {
        info!("🚀 Initializing Enhanced Shared State...");
        
        // Start deadlock detection
        self.deadlock_detector.start_detection().await;
        
        info!("✅ Enhanced Shared State initialized");
        Ok(())
    }

    /// Update Raydium price atomically
    pub fn update_raydium_price(&self, price: rust_decimal::Decimal) {
        self.price_state.update_raydium_price(price);
    }

    /// Update Orca price atomically
    pub fn update_orca_price(&self, price: rust_decimal::Decimal) {
        self.price_state.update_orca_price(price);
    }

    /// Get consistent price snapshot for arbitrage
    pub fn get_price_snapshot(&self) -> atomic_state::PriceSnapshot {
        self.price_state.get_price_snapshot()
    }

    /// Check if prices are fresh enough for trading
    pub fn are_prices_fresh(&self, max_age_seconds: u64) -> bool {
        self.price_state.are_prices_fresh(max_age_seconds)
    }

    /// Increment trades counter atomically
    pub fn increment_trades(&self) -> u64 {
        self.lock_free_state.increment_trades()
    }

    /// Get current trades count
    pub fn get_trades_today(&self) -> u64 {
        self.lock_free_state.get_trades_today()
    }

    /// Add profit with minimal locking
    pub async fn add_profit(&self, profit: f64) -> Result<()> {
        self.lock_free_state.add_profit(profit).await
    }

    /// Get current profit
    pub async fn get_profit_today(&self) -> Result<f64> {
        self.lock_free_state.get_profit_today().await
    }

    /// Record trade in bounded history
    pub async fn record_trade(&self, trade: TradeRecord) {
        self.trade_history.add_trade(trade).await;
    }

    /// Get recent trades
    pub async fn get_recent_trades(&self, count: usize) -> Vec<TradeRecord> {
        self.trade_history.get_recent_trades(count).await
    }

    /// Reset daily counters
    pub async fn reset_daily(&self) -> Result<()> {
        self.lock_free_state.reset_daily().await
    }

    /// Perform complex operation with deadlock protection
    pub async fn safe_complex_operation<F, R>(&self, operation_id: String, operation: F) -> Result<R>
    where
        F: std::future::Future<Output = Result<R>>,
    {
        // Register operation for deadlock detection
        self.deadlock_detector.register_operation(
            operation_id.clone(),
            "Complex arbitrage operation".to_string()
        ).await;

        // Execute operation with timeout
        let result = tokio::time::timeout(Duration::from_secs(10), operation).await;

        // Unregister operation
        self.deadlock_detector.unregister_operation(&operation_id).await;

        match result {
            Ok(op_result) => op_result,
            Err(_) => {
                error!("⏰ Operation timeout: {}", operation_id);
                Err(anyhow::anyhow!("Operation timeout: {}", operation_id))
            }
        }
    }

    /// Get comprehensive system statistics
    pub async fn get_system_stats(&self) -> SystemStats {
        let price_stats = self.price_state.get_stats();
        let performance_stats = self.lock_free_state.get_performance_stats();
        let lock_stats = self.deadlock_manager.get_stats();
        let memory_stats = self.trade_history.get_memory_stats().await;

        SystemStats {
            price_stats,
            performance_stats,
            lock_stats,
            memory_stats,
        }
    }

    /// Perform system health check
    pub async fn health_check(&self) -> HealthCheckResult {
        let mut issues = Vec::new();
        let mut status = HealthStatus::Healthy;

        // Check price freshness
        if !self.are_prices_fresh(60) {
            issues.push("Stale price data".to_string());
            status = HealthStatus::Warning;
        }

        // Check lock performance
        let lock_stats = self.deadlock_manager.get_stats();
        if lock_stats.success_rate < 95.0 {
            issues.push(format!("Low lock success rate: {:.1}%", lock_stats.success_rate));
            status = HealthStatus::Warning;
        }

        // Check memory usage
        let memory_stats = self.trade_history.get_memory_stats().await;
        if memory_stats.current_count >= memory_stats.cleanup_threshold {
            issues.push("High memory usage in trade history".to_string());
            if status == HealthStatus::Healthy {
                status = HealthStatus::Warning;
            }
        }

        // Check for deadlocks
        if lock_stats.deadlock_detections > 0 {
            issues.push("Deadlocks detected".to_string());
            status = HealthStatus::Critical;
        }

        HealthCheckResult {
            status,
            issues,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Comprehensive system statistics
#[derive(Debug)]
pub struct SystemStats {
    pub price_stats: atomic_state::PriceStats,
    pub performance_stats: deadlock_free::PerformanceStats,
    pub lock_stats: deadlock_free::LockStats,
    pub memory_stats: atomic_state::MemoryStats,
}

/// Health check result
#[derive(Debug)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub issues: Vec<String>,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

/// Migration utilities for existing code
pub mod migration {
    use super::*;
    use rust_decimal::Decimal;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// Convert legacy SharedState to EnhancedSharedState
    pub async fn migrate_from_legacy(
        legacy_raydium: Arc<Mutex<Option<Decimal>>>,
        legacy_orca: Arc<Mutex<Option<Decimal>>>,
        _legacy_trades: Arc<Mutex<u32>>,
        _legacy_profit: Arc<Mutex<Decimal>>,
    ) -> Result<EnhancedSharedState> {
        let enhanced = EnhancedSharedState::new();
        
        // Migrate current values
        if let Some(raydium_price) = *legacy_raydium.lock().await {
            enhanced.update_raydium_price(raydium_price);
        }
        
        if let Some(orca_price) = *legacy_orca.lock().await {
            enhanced.update_orca_price(orca_price);
        }
        
        // Note: trades and profit will start fresh in the new system
        // This is intentional to avoid race conditions during migration
        
        enhanced.initialize().await?;
        
        info!("✅ Successfully migrated to Enhanced Shared State");
        Ok(enhanced)
    }

    /// Helper to gradually replace legacy mutex usage
    pub struct LegacyCompatibilityLayer {
        enhanced_state: Arc<EnhancedSharedState>,
    }

    impl LegacyCompatibilityLayer {
        pub fn new(enhanced_state: Arc<EnhancedSharedState>) -> Self {
            Self { enhanced_state }
        }

        /// Get prices in legacy format for backward compatibility
        pub async fn get_legacy_prices(&self) -> (Option<Decimal>, Option<Decimal>) {
            let snapshot = self.enhanced_state.get_price_snapshot();
            (snapshot.raydium_price, snapshot.orca_price)
        }

        /// Get trades/profit in legacy format
        pub async fn get_legacy_stats(&self) -> Result<(u32, Decimal)> {
            let trades = self.enhanced_state.get_trades_today() as u32;
            let profit = self.enhanced_state.get_profit_today().await?;
            let profit_decimal = Decimal::from_f64_retain(profit).unwrap_or(Decimal::ZERO);
            
            Ok((trades, profit_decimal))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enhanced_shared_state() {
        let state = EnhancedSharedState::new();
        state.initialize().await.unwrap();

        // Test price updates
        let price = rust_decimal::Decimal::from_f64_retain(150.0).unwrap();
        state.update_raydium_price(price);
        state.update_orca_price(price);

        // Test price snapshot
        let snapshot = state.get_price_snapshot();
        assert!(snapshot.is_valid);
        assert_eq!(snapshot.raydium_price, Some(price));
        assert_eq!(snapshot.orca_price, Some(price));

        // Test counters
        assert_eq!(state.get_trades_today(), 0);
        state.increment_trades();
        assert_eq!(state.get_trades_today(), 1);

        // Test profit
        state.add_profit(100.0).await.unwrap();
        assert_eq!(state.get_profit_today().await.unwrap(), 100.0);
    }

    #[tokio::test]
    async fn test_health_check() {
        let state = EnhancedSharedState::new();
        state.initialize().await.unwrap();

        let health = state.health_check().await;
        // Should be warning due to stale prices (no updates yet)
        assert_eq!(health.status, HealthStatus::Warning);
    }
}
