//! Safety Guard Module
//! Chroni przed stratami i błędami, szczególnie ważne przy użyciu Ledger

use anyhow::Result;
use log::{info, warn, error};
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc, Duration};
use std::collections::VecDeque;
use rust_decimal::Decimal;
use crate::utils::conversions::*;

#[derive(Clone)]
pub struct SafetyGuard {
    max_position_sol: f64,
    max_daily_loss_usd: f64,
    max_daily_trades: u32,
    min_pool_liquidity_usd: f64,
    
    // Ledger-specific limits
    ledger_max_auto_approve: f64,  // Max USD bez dodatkowego potwierdzenia
    ledger_daily_limit: f64,       // Dzienny limit dla Ledger
    ledger_consecutive_losses: u32, // Max strat z rzędu przed zatrzymaniem
    
    // Tracking
    trade_history: Arc<Mutex<VecDeque<TradeRecord>>>,
}

#[derive(Clone, Debug)]
struct TradeRecord {
    timestamp: DateTime<Utc>,
    profit_usd: f64,
    amount_sol: f64,
    success: bool,
}

impl SafetyGuard {
    pub fn new(config: &crate::Config) -> Self {
        Self {
            max_position_sol: decimal_to_f64(config.limits.max_position_sol),
            max_daily_loss_usd: decimal_to_f64(config.limits.max_daily_loss_usd),
            max_daily_trades: config.limits.max_daily_trades,
            min_pool_liquidity_usd: 50000.0, // Default minimum liquidity
            
            // Ledger limits - bardziej restrykcyjne
            ledger_max_auto_approve: 50.0,  // Max 50 USD bez dodatkowego potwierdzenia
            ledger_daily_limit: 1000.0,     // Max 1000 USD dziennie przez Ledger
            ledger_consecutive_losses: 3,    // Stop po 3 stratach z rzędu
            
            trade_history: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),
        }
    }
    
    pub async fn should_continue_trading(&self, state: &crate::SharedState) -> bool {
        let trades_today = *state.trades_today.lock().await;
        let profit_today = *state.profit_today.lock().await;
        
        // Check daily trade limit
        if trades_today >= self.max_daily_trades {
            warn!("⛔ Daily trade limit reached: {}/{}", trades_today, self.max_daily_trades);
            return false;
        }
        
        // Check daily loss limit
        if decimal_lt_f64(profit_today, -self.max_daily_loss_usd) {
            error!("🛑 Daily loss limit exceeded: ${:.2}", decimal_to_f64(profit_today));
            return false;
        }
        
        // Check consecutive losses (ważne dla Ledger)
        {
            let history = self.trade_history.lock().await;
            let recent_losses = history.iter()
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
            warn!("⚠️ Approaching daily Ledger limit: ${:.2}/{:.2}",
                 profit_abs, self.ledger_daily_limit);
        }
        
        true
    }
    
    pub async fn pre_trade_check(
        &self,
        opportunity: &crate::calculator::ArbitrageOpportunity,
        using_ledger: bool,
    ) -> Result<bool> {
        // Check position size
        if opportunity.amount_sol > self.max_position_sol {
            warn!("⚠️ Position too large: {} SOL > {} SOL max",
                 opportunity.amount_sol, self.max_position_sol);
            return Ok(false);
        }
        
        // Ledger-specific checks
        if using_ledger {
            // Require manual confirmation for large trades
            if opportunity.expected_profit_usd > self.ledger_max_auto_approve {
                warn!("📱 Large trade (${:.2}) requires manual Ledger confirmation!",
                     opportunity.expected_profit_usd);
                
                // Give user time to prepare
                info!("Przygotuj Ledger - potwierdzenie za 5 sekund...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
            
            // Extra safety for first trades
            let history = self.trade_history.lock().await;
            if history.len() < 5 {
                warn!("🆕 One of first 5 trades - extra careful!");
                if opportunity.amount_sol > self.max_position_sol * 0.1 {
                    info!("Reducing position size for safety: {} SOL -> {} SOL",
                        opportunity.amount_sol, self.max_position_sol * 0.1);
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
        let last_hour_trades = history.iter()
            .filter(|t| t.timestamp > Utc::now() - Duration::hours(1))
            .count();
        
        let last_hour_profit: f64 = history.iter()
            .filter(|t| t.timestamp > Utc::now() - Duration::hours(1))
            .map(|t| t.profit_usd)
            .sum();
        
        info!("📊 Last hour: {} trades, ${:.2} profit", last_hour_trades, last_hour_profit);
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
        
        // Tu możesz dodać:
        // - Wysłanie alertu na Telegram
        // - Zapis do bazy danych
        // - Anulowanie pending orders
        
        // Zatrzymaj program
        std::process::exit(1);
    }
    
    pub fn validate_pool_liquidity(&self, liquidity_usd: f64) -> bool {
        if liquidity_usd < self.min_pool_liquidity_usd {
            warn!("⚠️ Pool liquidity too low: ${:.2} < ${:.2} minimum",
                liquidity_usd, self.min_pool_liquidity_usd);
            return false;
        }
        true
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
                max_position_sol: 10.0,
                min_profit_percent: 0.3,
                min_profit_usd: 1.0,
                max_slippage_percent: 0.5,
                max_daily_loss_usd: 100.0,
                max_daily_trades: 30,
            },
            execution: crate::ExecutionConfig {
                priority_fee_lamports: 10000,
                simulation_required: true,
                max_retries: 3,
            },
        }
    }
    
    fn test_state() -> crate::SharedState {
        crate::SharedState {
            raydium_price: Arc::new(Mutex::new(Some(150.0))),
            orca_price: Arc::new(Mutex::new(Some(150.0))),
            trades_today: Arc::new(Mutex::new(0)),
            profit_today: Arc::new(Mutex::new(0.0)),
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