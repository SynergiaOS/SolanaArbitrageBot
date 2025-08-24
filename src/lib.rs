//! Solana Arbitrage Bot Library
//! Exports public types and modules for external use and testing

pub mod calculator;
pub mod discord;
pub mod executor;
pub mod ledger;
pub mod monitor;
pub mod safety;
pub mod sniper;
pub mod utils;
pub mod verification;
pub mod web;

// Advanced trading components (temporarily disabled)
// pub mod gepa;
// pub mod kestra;

// Re-export main types
pub use crate::calculator::{
    ArbitrageOpportunity, MarketConditions, NetworkCongestion, ProfitCalculator,
};
pub use crate::discord::DiscordAlert;
pub use crate::executor::{TransactionExecutor, WalletType};
pub use crate::ledger::LedgerConnection;
pub use crate::monitor::{DexMonitor, PriceUpdate};
pub use crate::safety::SafetyGuard;
pub use crate::web::{WebConfig, WebServer};

// Re-export sniper types
pub use crate::sniper::{
    NewToken, Position, SafetyResult, SellAction, SniperConfig, SniperEngine, TradeResult,
};

// Re-export config types
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub rpc: RpcConfig,
    pub wallet: WalletConfig,
    pub dex: DexConfig,
    pub limits: LimitsConfig,
    pub execution: ExecutionConfig,
    pub discord: Option<DiscordConfig>,
    pub web: Option<WebConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RpcConfig {
    pub url: String,
    pub ws_url: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct WalletConfig {
    pub path: String,
    pub use_ledger: Option<bool>,
    pub ledger_path: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DexConfig {
    pub raydium: DexInfo,
    pub orca: DexInfo,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DexInfo {
    pub program_id: String,
    pub sol_usdc_pool: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LimitsConfig {
    #[serde(with = "rust_decimal::serde::float")]
    pub max_position_sol: rust_decimal::Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub min_profit_percent: rust_decimal::Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub min_profit_usd: rust_decimal::Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub max_slippage_percent: rust_decimal::Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub max_daily_loss_usd: rust_decimal::Decimal,
    pub max_daily_trades: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExecutionConfig {
    pub priority_fee_lamports: u64,
    pub simulation_required: bool,
    pub max_retries: u32,
}

#[derive(Clone, serde::Deserialize)]
pub struct DiscordConfig {
    pub webhook_url: String,
    pub enabled: bool,
    pub alert_on_profit: Option<bool>,
    pub alert_on_error: Option<bool>,
    pub alert_on_startup: Option<bool>,
}

impl std::fmt::Debug for DiscordConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscordConfig")
            .field("webhook_url", &"***MASKED***")
            .field("enabled", &self.enabled)
            .field("alert_on_profit", &self.alert_on_profit)
            .field("alert_on_error", &self.alert_on_error)
            .field("alert_on_startup", &self.alert_on_startup)
            .finish()
    }
}

use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::Mutex;

// Shared state type used in main
#[derive(Clone)]
pub struct SharedState {
    pub raydium_price: Arc<Mutex<Option<Decimal>>>,
    pub orca_price: Arc<Mutex<Option<Decimal>>>,
    pub trades_today: Arc<Mutex<u32>>,
    pub profit_today: Arc<Mutex<Decimal>>,
}
