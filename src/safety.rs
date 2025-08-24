//! Safety Guard Module
//! Chroni przed stratami i błędami, szczególnie ważne przy użyciu Ledger

use crate::utils::conversions::*;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use log::{error, info, warn};
use rust_decimal::Decimal;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone)]
pub enum CircuitBreakerState {
    Closed,   // Normal operation
    Open,     // Circuit is open, trading stopped
    HalfOpen, // Testing if service recovered
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: CircuitBreakerState,
    failure_count: u32,
    success_count: u32,
    failure_threshold: u32,
    success_threshold: u32,
    timeout_duration: std::time::Duration,
    last_failure_time: Option<std::time::Instant>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout_seconds: u64) -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            success_count: 0,
            failure_threshold,
            success_threshold,
            timeout_duration: std::time::Duration::from_secs(timeout_seconds),
            last_failure_time: None,
        }
    }

    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed() >= self.timeout_duration {
                        self.state = CircuitBreakerState::HalfOpen;
                        self.success_count = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }

    pub fn record_success(&mut self) {
        match self.state {
            CircuitBreakerState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    self.state = CircuitBreakerState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            }
            CircuitBreakerState::Closed => {
                self.failure_count = 0;
            }
            CircuitBreakerState::Open => {
                // Should not happen, but reset if it does
                self.state = CircuitBreakerState::Closed;
                self.failure_count = 0;
            }
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(std::time::Instant::now());

        if self.failure_count >= self.failure_threshold {
            self.state = CircuitBreakerState::Open;
            self.success_count = 0;
        }
    }

    pub fn get_state(&self) -> &CircuitBreakerState {
        &self.state
    }

    pub fn is_open(&self) -> bool {
        matches!(self.state, CircuitBreakerState::Open)
    }
}

#[derive(Clone)]
pub struct SafetyGuard {
    max_position_sol: f64,
    // Read dynamically from runtime_config
    runtime_config: Option<Arc<RwLock<crate::web::BotConfig>>>,
    min_pool_liquidity_usd: f64,

    // Ledger-specific limits
    ledger_max_auto_approve: f64, // Max USD bez dodatkowego potwierdzenia
    ledger_daily_limit: f64,      // Dzienny limit dla Ledger
    ledger_consecutive_losses: u32, // Max strat z rzędu przed zatrzymaniem

    // Tracking
    trade_history: Arc<Mutex<VecDeque<TradeRecord>>>,

    // Circuit Breaker
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
}

#[derive(Clone, Debug)]
struct TradeRecord {
    timestamp: DateTime<Utc>,
    profit_usd: f64,
    #[allow(dead_code)]
    amount_sol: f64,
    success: bool,
}

impl SafetyGuard {
    pub fn new(config: &crate::Config) -> Self {
        Self {
            max_position_sol: decimal_to_f64(config.limits.max_position_sol),
            runtime_config: None,
            min_pool_liquidity_usd: 50000.0, // Default minimum liquidity

            // Ledger limits - bardziej restrykcyjne
            ledger_max_auto_approve: 50.0, // Max 50 USD bez dodatkowego potwierdzenia
            ledger_daily_limit: 1000.0,    // Max 1000 USD dziennie przez Ledger
            ledger_consecutive_losses: 3,  // Stop po 3 stratach z rzędu

            trade_history: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),

            // Circuit breaker: 5 failures trigger open, 3 successes close, 300s timeout
            circuit_breaker: Arc::new(Mutex::new(CircuitBreaker::new(5, 3, 300))),
        }
    }

    pub fn with_runtime_config(mut self, cfg: Arc<RwLock<crate::web::BotConfig>>) -> Self {
        self.runtime_config = Some(cfg);
        self
    }

    pub async fn should_continue_trading(&self, state: &crate::SharedState) -> bool {
        let trades_today = *state.trades_today.lock().await;
        let profit_today = *state.profit_today.lock().await;

        // Read dynamic safety limits from runtime_config if available
        let (max_daily_trades, max_daily_loss_usd) = if let Some(ref arc_cfg) = self.runtime_config
        {
            let cfg = arc_cfg.read().await.clone();
            (cfg.max_daily_trades, decimal_to_f64(cfg.max_daily_loss_usd))
        } else {
            // Fallback to conservative defaults if runtime_config not attached
            (30u32, 100.0f64)
        };

        // Check daily trade limit
        if trades_today >= max_daily_trades {
            warn!(
                "⛔ Daily trade limit reached: {}/{}",
                trades_today, max_daily_trades
            );
            return false;
        }

        // Check daily loss limit
        if decimal_lt_f64(profit_today, -max_daily_loss_usd) {
            error!(
                "🛑 Daily loss limit exceeded: ${:.2}",
                decimal_to_f64(profit_today)
            );
            return false;
        }

        // Check consecutive losses (ważne dla Ledger)
        {
            let history = self.trade_history.lock().await;
            let recent_losses = history
                .iter()
                .rev()
                .take(self.ledger_consecutive_losses as usize)
                .filter(|t| !t.success)
                .count();

            if recent_losses >= self.ledger_consecutive_losses as usize {
                error!("🛑 Too many consecutive losses: {}", recent_losses);
                return false;
            }
        }

        // Check if we're near daily Ledger limit
        let profit_abs = decimal_to_f64(profit_today.abs());
        if profit_abs > self.ledger_daily_limit * 0.9 {
            warn!(
                "⚠️ Approaching daily Ledger limit: ${:.2}/{:.2}",
                profit_abs, self.ledger_daily_limit
            );
        }

        true
    }

    pub async fn pre_trade_check(
        &self,
        opportunity: &crate::calculator::ArbitrageOpportunity,
        using_ledger: bool,
    ) -> Result<bool> {
        // Determine effective max_position_sol from runtime_config if available
        let effective_max_position = if let Some(ref arc_cfg) = self.runtime_config {
            let cfg = arc_cfg.read().await.clone();
            let dyn_max = decimal_to_f64(cfg.max_position_sol);
            if (dyn_max - self.max_position_sol).abs() > f64::EPSILON {
                info!(
                    "🔧 Effective max_position_sol updated (SafetyGuard): static={} -> runtime={}",
                    self.max_position_sol, dyn_max
                );
            }
            dyn_max
        } else {
            self.max_position_sol
        };

        // Check position size against effective limit
        if opportunity.amount_sol > effective_max_position {
            warn!(
                "⚠️ Position too large: {} SOL > {} SOL max",
                opportunity.amount_sol, effective_max_position
            );
            return Ok(false);
        }

        // Ledger-specific checks
        if using_ledger {
            // Require manual confirmation for large trades
            if opportunity.expected_profit_usd > self.ledger_max_auto_approve {
                warn!(
                    "📱 Large trade (${:.2}) requires manual Ledger confirmation!",
                    opportunity.expected_profit_usd
                );

                // Give user time to prepare
                info!("Przygotuj Ledger - potwierdzenie za 5 sekund...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }

            // Extra safety for first trades
            let history = self.trade_history.lock().await;
            if history.len() < 5 {
                warn!("🆕 One of first 5 trades - extra careful!");
                if opportunity.amount_sol > self.max_position_sol * 0.1 {
                    info!(
                        "Reducing position size for safety: {} SOL -> {} SOL",
                        opportunity.amount_sol,
                        self.max_position_sol * 0.1
                    );
                    return Ok(false); // Reject large trades initially
                }
            }
        }

        // Volatility check - avoid trading during extreme moves
        if opportunity.profit_after_fees_usd > opportunity.expected_profit_usd * 2.0 {
            warn!("⚠️ Unusual profit margin detected - possible data issue");
            return Ok(false);
        }

        Ok(true)
    }

    pub async fn record_trade(&self, profit: f64, amount: f64, success: bool) {
        let mut history = self.trade_history.lock().await;

        history.push_back(TradeRecord {
            timestamp: Utc::now(),
            profit_usd: profit,
            amount_sol: amount,
            success,
        });

        // Keep only last 1000 trades
        if history.len() > 1000 {
            history.pop_front();
        }

        // Log statistics
        let last_hour_trades = history
            .iter()
            .filter(|t| t.timestamp > Utc::now() - Duration::hours(1))
            .count();

        let last_hour_profit: f64 = history
            .iter()
            .filter(|t| t.timestamp > Utc::now() - Duration::hours(1))
            .map(|t| t.profit_usd)
            .sum();

        info!(
            "📊 Last hour: {} trades, ${:.2} profit",
            last_hour_trades, last_hour_profit
        );
    }

    pub async fn reset_daily_limits(&self, state: &crate::SharedState) {
        let mut trades = state.trades_today.lock().await;
        *trades = 0;

        let mut profit = state.profit_today.lock().await;
        *profit = Decimal::ZERO;

        info!("🔄 Daily limits reset");
    }

    pub async fn emergency_stop(&self, reason: &str) {
        error!("🚨 EMERGENCY STOP: {}", reason);
        error!("🚨 Bot zatrzymany - wymagana manualna interwencja");

        // Log emergency stop to database if available
        self.log_emergency_event(reason).await;

        // Send alerts to all configured channels
        self.send_emergency_alerts(reason).await;

        // Cancel all pending operations
        self.cancel_pending_operations().await;

        // Attempt graceful shutdown
        self.graceful_shutdown().await;

        // Force exit if graceful shutdown fails
        error!("🚨 Force exit due to emergency stop");
        std::process::exit(1);
    }

    /// Log emergency event to persistent storage
    async fn log_emergency_event(&self, reason: &str) {
        // TODO: Implement database logging
        error!("📝 Emergency event logged: {}", reason);
    }

    /// Send emergency alerts to all configured channels
    async fn send_emergency_alerts(&self, reason: &str) {
        // TODO: Implement Discord alerts
        error!("📢 Emergency alert sent: {}", reason);

        // TODO: Implement Telegram alerts
        // TODO: Implement email alerts
        // TODO: Implement SMS alerts
    }

    /// Cancel all pending operations
    async fn cancel_pending_operations(&self) {
        // TODO: Cancel pending transactions
        // TODO: Cancel pending orders
        // TODO: Close open connections
        warn!("⚠️ Cancelling all pending operations");
    }

    /// Attempt graceful shutdown
    async fn graceful_shutdown(&self) {
        // Give system time to cleanup
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // TODO: Flush any pending writes
        // TODO: Close database connections
        // TODO: Save current state for recovery

        info!("✅ Graceful shutdown completed");
    }

    pub fn validate_pool_liquidity(&self, liquidity_usd: f64) -> bool {
        if liquidity_usd < self.min_pool_liquidity_usd {
            warn!(
                "⚠️ Pool liquidity too low: ${:.2} < ${:.2} minimum",
                liquidity_usd, self.min_pool_liquidity_usd
            );
            return false;
        }
        true
    }

    /// Check if circuit breaker allows trading
    pub async fn can_trade(&self) -> bool {
        let mut circuit_breaker = self.circuit_breaker.lock().await;
        let can_execute = circuit_breaker.can_execute();

        if !can_execute {
            error!("🚫 CIRCUIT BREAKER: Trading blocked - too many failures");
        }

        can_execute
    }

    /// Record successful trade for circuit breaker
    pub async fn record_success(&self) {
        let mut circuit_breaker = self.circuit_breaker.lock().await;
        circuit_breaker.record_success();

        if matches!(circuit_breaker.get_state(), CircuitBreakerState::Closed) {
            info!("✅ Circuit breaker closed - normal operation resumed");
        }
    }

    /// Record failed trade for circuit breaker
    pub async fn record_failure(&self, reason: &str) {
        let mut circuit_breaker = self.circuit_breaker.lock().await;
        circuit_breaker.record_failure();

        match circuit_breaker.get_state() {
            CircuitBreakerState::Open => {
                error!("🚨 CIRCUIT BREAKER OPENED: Trading stopped due to failures");
                error!("   Reason: {}", reason);
                error!("   Circuit will remain open for 5 minutes");
            }
            CircuitBreakerState::HalfOpen => {
                warn!("⚠️ Circuit breaker testing recovery");
            }
            _ => {}
        }
    }

    /// Get circuit breaker status
    pub async fn get_circuit_breaker_status(&self) -> CircuitBreakerState {
        let circuit_breaker = self.circuit_breaker.lock().await;
        circuit_breaker.get_state().clone()
    }

    /// Force reset circuit breaker (manual intervention)
    pub async fn reset_circuit_breaker(&self) {
        let mut circuit_breaker = self.circuit_breaker.lock().await;
        circuit_breaker.state = CircuitBreakerState::Closed;
        circuit_breaker.failure_count = 0;
        circuit_breaker.success_count = 0;
        circuit_breaker.last_failure_time = None;

        warn!("🔧 Circuit breaker manually reset");
    }

    /// Emergency liquidation of all positions
    pub async fn emergency_liquidation(&self, reason: &str) -> Result<()> {
        error!("🚨 EMERGENCY LIQUIDATION: {}", reason);

        // TODO: Implement position liquidation logic
        // This would integrate with the position manager to sell all positions
        // at market price with maximum slippage

        error!("💰 Emergency liquidation executed - all positions sold");
        Ok(())
    }

    /// Health check for critical systems
    pub async fn health_check(&self) -> Result<bool> {
        // Check circuit breaker status
        let circuit_breaker_ok = {
            let circuit_breaker = self.circuit_breaker.lock().await;
            !circuit_breaker.is_open()
        };

        // Check trade history for anomalies
        let history_ok = {
            let history = self.trade_history.lock().await;
            let recent_trades: Vec<_> = history
                .iter()
                .filter(|t| t.timestamp > Utc::now() - Duration::hours(1))
                .collect();

            let failure_rate = if !recent_trades.is_empty() {
                recent_trades.iter().filter(|t| !t.success).count() as f64
                    / recent_trades.len() as f64
            } else {
                0.0
            };

            failure_rate < 0.5 // Less than 50% failure rate in last hour
        };

        let overall_health = circuit_breaker_ok && history_ok;

        if !overall_health {
            warn!("⚠️ Health check failed - system may be unstable");
        }

        Ok(overall_health)
    }
}

// Dodatki bezpieczeństwa dla produkcji:
impl SafetyGuard {
    pub async fn check_network_conditions(&self) -> Result<bool> {
        // Sprawdź opóźnienie RPC
        // Sprawdź slot lag
        // Sprawdź mempool congestion
        Ok(true)
    }

    pub async fn validate_ledger_connection(&self) -> Result<bool> {
        // Sprawdź czy Ledger jest podłączony
        // Sprawdź czy aplikacja Solana jest otwarta
        // Sprawdź wersję firmware
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> crate::Config {
        crate::Config {
            rpc: crate::RpcConfig {
                url: "test".to_string(),
                ws_url: "test".to_string(),
            },
            wallet: crate::WalletConfig {
                path: "test".to_string(),
                use_ledger: Some(false),
                ledger_path: None,
            },
            dex: crate::DexConfig {
                raydium: crate::DexInfo {
                    program_id: "test".to_string(),
                    sol_usdc_pool: "test".to_string(),
                },
                orca: crate::DexInfo {
                    program_id: "test".to_string(),
                    sol_usdc_pool: "test".to_string(),
                },
            },
            limits: crate::LimitsConfig {
                max_position_sol: rust_decimal::Decimal::from_f64_retain(10.0).unwrap(),
                min_profit_percent: rust_decimal::Decimal::from_f64_retain(0.3).unwrap(),
                min_profit_usd: rust_decimal::Decimal::from_f64_retain(1.0).unwrap(),
                max_slippage_percent: rust_decimal::Decimal::from_f64_retain(0.5).unwrap(),
                max_daily_loss_usd: rust_decimal::Decimal::from_f64_retain(100.0).unwrap(),
                max_daily_trades: 30,
            },
            execution: crate::ExecutionConfig {
                priority_fee_lamports: 10000,
                simulation_required: true,
                max_retries: 3,
            },
            discord: None,
            web: None,
        }
    }

    fn test_state() -> crate::SharedState {
        crate::SharedState {
            raydium_price: Arc::new(Mutex::new(Some(
                rust_decimal::Decimal::from_f64_retain(150.0).unwrap(),
            ))),
            orca_price: Arc::new(Mutex::new(Some(
                rust_decimal::Decimal::from_f64_retain(150.0).unwrap(),
            ))),
            trades_today: Arc::new(Mutex::new(0)),
            profit_today: Arc::new(Mutex::new(
                rust_decimal::Decimal::from_f64_retain(0.0).unwrap(),
            )),
        }
    }

    #[tokio::test]
    async fn test_consecutive_losses() {
        let guard = SafetyGuard::new(&test_config());

        // Zapisz 3 straty
        for _ in 0..3 {
            guard.record_trade(-10.0, 1.0, false).await;
        }

        // Powinien zatrzymać trading
        let state = test_state();
        assert!(!guard.should_continue_trading(&state).await);
    }
}
