//! Enhanced Safety Module - Complete Refactoring
//! Production-ready safety checks and protections

use crate::config_manager::{BotConfig, SafetyConfig, CircuitBreakerConfig};
use crate::utils::conversions::*;
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use log::{error, info, warn};
use rust_decimal::Decimal;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Circuit breaker states for fault tolerance
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Circuit tripped, operations blocked
    HalfOpen, // Testing recovery
}

/// Enhanced circuit breaker with metrics
pub struct CircuitBreaker {
    state: CircuitState,
    config: CircuitBreakerConfig,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<std::time::Instant>,
    total_failures: u64,
    total_successes: u64,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: CircuitState::Closed,
            config,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            total_failures: 0,
            total_successes: 0,
        }
    }

    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed().as_secs() >= self.config.timeout_seconds {
                        info!("Circuit breaker entering half-open state");
                        self.state = CircuitState::HalfOpen;
                        self.success_count = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn record_success(&mut self) {
        self.total_successes += 1;
        
        match self.state {
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.config.success_threshold {
                    info!("Circuit breaker closed - system recovered");
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            _ => {}
        }
    }

    pub fn record_failure(&mut self) {
        self.total_failures += 1;
        self.failure_count += 1;
        self.last_failure_time = Some(std::time::Instant::now());

        if self.failure_count >= self.config.failure_threshold {
            error!("Circuit breaker opened - too many failures");
            self.state = CircuitState::Open;
            self.success_count = 0;
        }
    }

    pub fn get_metrics(&self) -> CircuitBreakerMetrics {
        CircuitBreakerMetrics {
            state: self.state.clone(),
            total_failures: self.total_failures,
            total_successes: self.total_successes,
            current_failure_count: self.failure_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    pub state: CircuitState,
    pub total_failures: u64,
    pub total_successes: u64,
    pub current_failure_count: u32,
}

/// Trade record for history tracking
#[derive(Clone, Debug)]
pub struct TradeRecord {
    pub timestamp: DateTime<Utc>,
    pub profit_usd: Decimal,
    pub amount_sol: Decimal,
    pub success: bool,
    pub dex: String,
    pub signature: Option<String>,
}

/// Main safety guard with configuration-driven checks
pub struct SafetyGuard {
    config: Arc<RwLock<BotConfig>>,
    trade_history: Arc<Mutex<VecDeque<TradeRecord>>>,
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
    daily_stats: Arc<Mutex<DailyStats>>,
}

#[derive(Debug, Clone)]
struct DailyStats {
    trades_today: u32,
    profit_today: Decimal,
    loss_streak: u32,
    last_reset: DateTime<Utc>,
}

impl SafetyGuard {
    /// Create new safety guard with configuration
    pub fn new(config: Arc<RwLock<BotConfig>>) -> Self {
        let circuit_config = {
            let cfg = config.blocking_read();
            cfg.safety.circuit_breaker.clone()
        };

        Self {
            config,
            trade_history: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),
            circuit_breaker: Arc::new(Mutex::new(CircuitBreaker::new(circuit_config))),
            daily_stats: Arc::new(Mutex::new(DailyStats {
                trades_today: 0,
                profit_today: Decimal::ZERO,
                loss_streak: 0,
                last_reset: Utc::now(),
            })),
        }
    }

    /// Check if trading should continue
    pub async fn should_continue_trading(&self) -> Result<bool> {
        // Check circuit breaker first
        {
            let mut breaker = self.circuit_breaker.lock().await;
            if !breaker.can_execute() {
                warn!("Circuit breaker is open - trading paused");
                return Ok(false);
            }
        }

        // Check daily stats
        let config = self.config.read().await;
        let mut stats = self.daily_stats.lock().await;

        // Reset daily stats if new day
        if stats.last_reset.date() < Utc::now().date() {
            info!("New trading day - resetting daily stats");
            stats.trades_today = 0;
            stats.profit_today = Decimal::ZERO;
            stats.last_reset = Utc::now();
        }

        // Check trade limit
        if stats.trades_today >= config.trading.max_daily_trades {
            warn!("Daily trade limit reached: {}/{}", 
                stats.trades_today, config.trading.max_daily_trades);
            return Ok(false);
        }

        // Check loss limit
        if stats.profit_today < -config.trading.max_daily_loss_usd {
            error!("Daily loss limit exceeded: ${}", stats.profit_today);
            return Ok(false);
        }

        // Check loss streak
        if stats.loss_streak >= 5 {
            error!("Too many consecutive losses: {}", stats.loss_streak);
            return Ok(false);
        }

        Ok(true)
    }

    /// Pre-trade safety check
    pub async fn pre_trade_check(
        &self,
        opportunity: &crate::calculator::ArbitrageOpportunity,
    ) -> Result<bool> {
        let config = self.config.read().await;
        
        // Check if safety checks are enabled
        if !config.safety.enable_safety_checks {
            warn!("⚠️ Safety checks disabled - proceeding without validation");
            return Ok(true);
        }

        // Validate opportunity parameters
        if opportunity.profit_after_fees_usd < decimal_to_f64(config.trading.min_profit_usd) {
            warn!("Profit below minimum: ${:.2} < ${}", 
                opportunity.profit_after_fees_usd,
                config.trading.min_profit_usd);
            return Ok(false);
        }

        if opportunity.amount_sol > decimal_to_f64(config.trading.max_position_sol) {
            warn!("Position size too large: {} > {} SOL",
                opportunity.amount_sol,
                config.trading.max_position_sol);
            return Ok(false);
        }

        // Check confidence score
        if opportunity.confidence_score < 0.5 {
            warn!("Confidence score too low: {:.1}%", 
                opportunity.confidence_score * 100.0);
            return Ok(false);
        }

        // Check slippage
        if opportunity.price_impact > decimal_to_f64(config.trading.max_slippage_percent) {
            warn!("Price impact too high: {:.2}% > {}%",
                opportunity.price_impact,
                config.trading.max_slippage_percent);
            return Ok(false);
        }

        // Ledger-specific checks if using hardware wallet
        if config.wallet.use_ledger {
            let value_usd = opportunity.amount_sol * opportunity.buy_price;
            if value_usd > decimal_to_f64(config.wallet.max_auto_approve_usd) {
                info!("Transaction requires manual Ledger approval: ${:.2}", value_usd);
                // In production, this would trigger a manual approval flow
            }
        }

        info!("✅ Pre-trade safety checks passed");
        Ok(true)
    }

    /// Record trade result
    pub async fn record_trade(
        &self,
        profit_usd: Decimal,
        amount_sol: Decimal,
        success: bool,
        dex: String,
        signature: Option<String>,
    ) -> Result<()> {
        // Update history
        let record = TradeRecord {
            timestamp: Utc::now(),
            profit_usd,
            amount_sol,
            success,
            dex,
            signature,
        };

        let mut history = self.trade_history.lock().await;
        history.push_back(record.clone());
        
        // Keep only last 1000 trades
        while history.len() > 1000 {
            history.pop_front();
        }

        // Update daily stats
        let mut stats = self.daily_stats.lock().await;
        stats.trades_today += 1;
        stats.profit_today += profit_usd;

        if success {
            stats.loss_streak = 0;
            self.circuit_breaker.lock().await.record_success();
        } else {
            stats.loss_streak += 1;
            self.circuit_breaker.lock().await.record_failure();
        }

        info!("📊 Trade recorded - Daily: {} trades, ${:.2} profit, {} loss streak",
            stats.trades_today, stats.profit_today, stats.loss_streak);

        Ok(())
    }

    /// Get current safety status
    pub async fn get_status(&self) -> SafetyStatus {
        let config = self.config.read().await;
        let stats = self.daily_stats.lock().await;
        let breaker_metrics = self.circuit_breaker.lock().await.get_metrics();
        let history = self.trade_history.lock().await;

        let recent_trades: Vec<TradeRecord> = history.iter()
            .rev()
            .take(10)
            .cloned()
            .collect();

        SafetyStatus {
            circuit_breaker_state: breaker_metrics.state,
            trades_today: stats.trades_today,
            profit_today: stats.profit_today,
            loss_streak: stats.loss_streak,
            max_daily_trades: config.trading.max_daily_trades,
            max_daily_loss: config.trading.max_daily_loss_usd,
            recent_trades,
            safety_enabled: config.safety.enable_safety_checks,
        }
    }

    /// Emergency stop
    pub async fn emergency_stop(&self, reason: &str) -> Result<()> {
        error!("🚨 EMERGENCY STOP: {}", reason);
        
        // Open circuit breaker
        self.circuit_breaker.lock().await.state = CircuitState::Open;
        
        // Record emergency event
        let record = TradeRecord {
            timestamp: Utc::now(),
            profit_usd: Decimal::ZERO,
            amount_sol: Decimal::ZERO,
            success: false,
            dex: "EMERGENCY".to_string(),
            signature: Some(reason.to_string()),
        };
        
        self.trade_history.lock().await.push_back(record);
        
        Ok(())
    }

    /// Reset daily limits (for new trading day)
    pub async fn reset_daily_limits(&self) {
        let mut stats = self.daily_stats.lock().await;
        stats.trades_today = 0;
        stats.profit_today = Decimal::ZERO;
        stats.loss_streak = 0;
        stats.last_reset = Utc::now();
        
        info!("📅 Daily limits reset for new trading day");
    }
}

/// Safety status for monitoring
#[derive(Debug, Clone)]
pub struct SafetyStatus {
    pub circuit_breaker_state: CircuitState,
    pub trades_today: u32,
    pub profit_today: Decimal,
    pub loss_streak: u32,
    pub max_daily_trades: u32,
    pub max_daily_loss: Decimal,
    pub recent_trades: Vec<TradeRecord>,
    pub safety_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout_seconds: 1,
            cooldown_minutes: 1,
        };

        let mut breaker = CircuitBreaker::new(config);
        
        // Should start closed
        assert!(breaker.can_execute());
        
        // Record failures
        breaker.record_failure();
        breaker.record_failure();
        breaker.record_failure();
        
        // Should be open now
        assert!(!breaker.can_execute());
        
        // Wait for timeout
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Should be half-open
        assert!(breaker.can_execute());
    }

    #[tokio::test]
    async fn test_safety_guard() {
        let config = Arc::new(RwLock::new(BotConfig::default()));
        let guard = SafetyGuard::new(config);
        
        // Should allow trading initially
        assert!(guard.should_continue_trading().await.unwrap());
        
        // Record some trades
        guard.record_trade(
            Decimal::from(10),
            Decimal::from(1),
            true,
            "Raydium".to_string(),
            None
        ).await.unwrap();
        
        let status = guard.get_status().await;
        assert_eq!(status.trades_today, 1);
        assert_eq!(status.profit_today, Decimal::from(10));
    }
}
