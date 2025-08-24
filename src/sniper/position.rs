//! Position Manager - Tracks and manages open positions

use crate::sniper::SniperConfig;
use chrono::{DateTime, Utc};
use log::{debug, info};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Position {
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub entry_signature: String,
    pub entry_time: DateTime<Utc>,
    pub entry_price: f64,
    pub current_price: f64,
    pub profit_loss_percent: f64,
}

#[derive(Debug)]
pub enum SellAction {
    TakeProfit,
    StopLoss,
    Timeout,
}

pub struct PositionManager {
    positions: Arc<Mutex<HashMap<Pubkey, Position>>>,
    config: SniperConfig,
}

impl Position {
    pub fn new(mint: Pubkey, sol_amount: u64, token_amount: u64, entry_signature: String) -> Self {
        let entry_price = if token_amount > 0 {
            sol_amount as f64 / token_amount as f64
        } else {
            0.0
        };

        Self {
            mint,
            sol_amount,
            token_amount,
            entry_signature,
            entry_time: Utc::now(),
            entry_price,
            current_price: entry_price,
            profit_loss_percent: 0.0,
        }
    }

    pub fn update_price(&mut self, current_price: f64) {
        // current_price in lamports per token unit
        self.current_price = current_price;

        if self.entry_price > 0.0 {
            self.profit_loss_percent =
                ((current_price - self.entry_price) / self.entry_price) * 100.0;
        }
    }

    pub fn get_current_value_sol(&self) -> f64 {
        (self.token_amount as f64 * self.current_price) / 1_000_000_000.0
    }

    pub fn get_entry_value_sol(&self) -> f64 {
        self.sol_amount as f64 / 1_000_000_000.0
    }

    pub fn get_profit_loss_sol(&self) -> f64 {
        self.get_current_value_sol() - self.get_entry_value_sol()
    }

    pub fn get_age_minutes(&self) -> i64 {
        let now = Utc::now();
        now.signed_duration_since(self.entry_time).num_minutes()
    }
}

impl PositionManager {
    pub fn new(config: SniperConfig) -> Self {
        Self {
            positions: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    pub async fn add_position(&self, position: Position) {
        let mut positions = self.positions.lock().await;

        info!(
            "📈 Adding position: {} - {} SOL",
            position.mint,
            position.get_entry_value_sol()
        );

        positions.insert(position.mint, position);

        info!("📊 Total positions: {}", positions.len());
    }

    pub async fn remove_position(&self, mint: &Pubkey) {
        let mut positions = self.positions.lock().await;

        if let Some(position) = positions.remove(mint) {
            info!(
                "📉 Removed position: {} - Final P&L: {:.1}%",
                mint, position.profit_loss_percent
            );
        }

        info!("📊 Remaining positions: {}", positions.len());
    }

    pub async fn get_active_positions(&self) -> Vec<Position> {
        let positions = self.positions.lock().await;
        positions.values().cloned().collect()
    }

    pub async fn get_position(&self, mint: &Pubkey) -> Option<Position> {
        let positions = self.positions.lock().await;
        positions.get(mint).cloned()
    }

    pub async fn update_position_price(&self, mint: &Pubkey, current_price: f64) {
        let mut positions = self.positions.lock().await;

        if let Some(position) = positions.get_mut(mint) {
            position.update_price(current_price);

            debug!(
                "💹 Updated price for {}: {:.8} SOL ({:.1}% P&L)",
                mint,
                current_price / 1_000_000_000.0,
                position.profit_loss_percent
            );
        }
    }

    pub async fn check_sell_conditions(
        &self,
        position: &Position,
        config: &SniperConfig,
    ) -> Option<SellAction> {
        // Check profit target
        if position.profit_loss_percent >= config.profit_target_percent {
            return Some(SellAction::TakeProfit);
        }

        // Check stop loss
        if position.profit_loss_percent <= -config.stop_loss_percent {
            return Some(SellAction::StopLoss);
        }

        // Check timeout (1 hour default)
        if position.get_age_minutes() >= 60 {
            return Some(SellAction::Timeout);
        }

        None
    }

    pub async fn get_total_invested_sol(&self) -> f64 {
        let positions = self.positions.lock().await;

        positions.values().map(|p| p.get_entry_value_sol()).sum()
    }

    pub async fn get_total_current_value_sol(&self) -> f64 {
        let positions = self.positions.lock().await;

        positions.values().map(|p| p.get_current_value_sol()).sum()
    }

    pub async fn get_total_profit_loss_sol(&self) -> f64 {
        let positions = self.positions.lock().await;

        positions.values().map(|p| p.get_profit_loss_sol()).sum()
    }

    pub async fn get_portfolio_summary(&self) -> PortfolioSummary {
        let positions = self.positions.lock().await;

        let total_positions = positions.len();
        let total_invested = positions.values().map(|p| p.get_entry_value_sol()).sum();
        let total_current = positions.values().map(|p| p.get_current_value_sol()).sum();
        let total_pnl = total_current - total_invested;
        let total_pnl_percent = if total_invested > 0.0 {
            (total_pnl / total_invested) * 100.0
        } else {
            0.0
        };

        let profitable_positions = positions
            .values()
            .filter(|p| p.profit_loss_percent > 0.0)
            .count();

        let losing_positions = positions
            .values()
            .filter(|p| p.profit_loss_percent < 0.0)
            .count();

        PortfolioSummary {
            total_positions,
            profitable_positions,
            losing_positions,
            total_invested_sol: total_invested,
            total_current_value_sol: total_current,
            total_pnl_sol: total_pnl,
            total_pnl_percent,
        }
    }
}

#[derive(Debug)]
pub struct PortfolioSummary {
    pub total_positions: usize,
    pub profitable_positions: usize,
    pub losing_positions: usize,
    pub total_invested_sol: f64,
    pub total_current_value_sol: f64,
    pub total_pnl_sol: f64,
    pub total_pnl_percent: f64,
}
