//! 🎯 Sniper Bot Types - Core Data Structures
//!
//! Common types used across the sniper bot modules

use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::collections::HashSet;
use std::time::SystemTime;

/// New token detected by the detector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewToken {
    /// Token mint address
    pub mint: String,

    /// Token symbol (e.g., "BONK")
    pub symbol: String,

    /// Token name (e.g., "Bonk Inu")
    pub name: String,

    /// Current price in USD
    pub price: f64,

    /// Liquidity in USD
    pub liquidity_usd: f64,

    /// Market cap in USD
    pub market_cap_usd: f64,

    /// Token age in minutes since creation
    pub age_minutes: f64,

    /// 5-minute volume in USD
    pub volume_5m: f64,

    /// Percentage held by top 10 holders
    pub top_10_holders_percent: f64,

    /// Whether mint authority is disabled (safer)
    pub mint_authority_disabled: bool,

    /// Whether freeze authority is disabled (safer)
    pub freeze_authority_disabled: bool,

    /// DEX where token was found
    pub dex: String,

    /// Pool address
    pub pool_address: String,

    /// When this token was detected
    pub detected_at: SystemTime,
}

/// Sniper bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniperConfig {
    /// Maximum position size in SOL
    pub position_size_sol: f64,

    /// Minimum liquidity required in USD
    pub min_liquidity_usd: f64,

    /// Maximum market cap to consider in USD
    pub max_market_cap_usd: f64,

    /// Maximum token age in minutes
    pub max_token_age_minutes: f64,

    /// Minimum score required to snipe
    pub min_score: f64,

    /// Take profit levels (multiplier, percentage to sell)
    pub take_profit_levels: Vec<TakeProfitLevel>,

    /// Stop loss multiplier (e.g., 0.7 = -30%)
    pub stop_loss: f64,

    /// Maximum daily loss in SOL
    pub max_daily_loss_sol: f64,

    /// Maximum snipes per hour
    pub max_snipes_per_hour: u32,

    /// Whether to use Jito bundles
    pub use_jito: bool,

    /// Jito tip in SOL
    pub jito_tip_sol: f64,

    /// Maximum concurrent snipes
    pub max_concurrent_snipes: u32,
}

impl Default for SniperConfig {
    fn default() -> Self {
        Self {
            position_size_sol: 1.0,
            min_liquidity_usd: 5000.0,
            max_market_cap_usd: 100000.0,
            max_token_age_minutes: 10.0,
            min_score: 60.0,
            take_profit_levels: vec![
                TakeProfitLevel {
                    multiplier: 2.0,
                    sell_percentage: 40.0,
                },
                TakeProfitLevel {
                    multiplier: 5.0,
                    sell_percentage: 30.0,
                },
                TakeProfitLevel {
                    multiplier: 10.0,
                    sell_percentage: 20.0,
                },
                TakeProfitLevel {
                    multiplier: 20.0,
                    sell_percentage: 10.0,
                },
            ],
            stop_loss: 0.7, // -30%
            max_daily_loss_sol: 5.0,
            max_snipes_per_hour: 10,
            use_jito: true,
            jito_tip_sol: 0.05,
            max_concurrent_snipes: 3,
        }
    }
}

/// Take profit level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeProfitLevel {
    /// Price multiplier (e.g., 2.0 = 2x)
    pub multiplier: f64,

    /// Percentage of position to sell (e.g., 40.0 = 40%)
    pub sell_percentage: f64,
}

/// Active sniper position
#[derive(Debug, Clone)]
pub struct SniperPosition {
    /// Token information
    pub token: NewToken,

    /// Entry transaction signature
    pub entry_signature: Signature,

    /// Entry price
    pub entry_price: f64,

    /// Entry time
    pub entry_time: chrono::DateTime<chrono::Utc>,

    /// Position size in SOL
    pub amount: f64,

    /// Take profit levels
    pub take_profit_levels: Vec<TakeProfitLevel>,

    /// Stop loss level
    pub stop_loss: f64,

    /// Which take profit levels have been triggered
    pub tp_triggered: HashSet<String>, // Using string keys for multipliers
}

/// Sniper execution result
#[derive(Debug, Clone)]
pub struct SniperResult {
    /// Transaction signature
    pub signature: Signature,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// Amount of tokens received
    pub tokens_received: u64,

    /// SOL amount spent
    pub sol_spent: f64,

    /// Whether execution was successful
    pub success: bool,

    /// Error message if failed
    pub error: Option<String>,
}

/// Token safety assessment
#[derive(Debug, Clone)]
pub struct SafetyAssessment {
    /// Overall safety score (0.0 - 1.0, higher = safer)
    pub safety_score: f64,

    /// Individual check results
    pub checks: SafetyChecks,

    /// Whether token is considered safe to trade
    pub is_safe: bool,

    /// Reasons why token might be unsafe
    pub warnings: Vec<String>,
}

/// Individual safety checks
#[derive(Debug, Clone)]
pub struct SafetyChecks {
    /// Mint authority disabled
    pub mint_authority_ok: bool,

    /// Freeze authority disabled
    pub freeze_authority_ok: bool,

    /// Reasonable holder distribution
    pub holder_distribution_ok: bool,

    /// Sufficient liquidity
    pub liquidity_ok: bool,

    /// Liquidity locked
    pub liquidity_locked: bool,

    /// No honeypot detected
    pub no_honeypot: bool,

    /// Creator not blacklisted
    pub creator_ok: bool,
}

/// Profit taking action
#[derive(Debug, Clone)]
pub enum ProfitAction {
    /// Take profit at specified level
    TakeProfit { level: f64, percentage: f64 },

    /// Stop loss triggered
    StopLoss,

    /// Hold position
    Hold,

    /// Emergency exit
    EmergencyExit { reason: String },
}

/// Sniper bot statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniperStats {
    /// Total snipes attempted
    pub total_snipes: u32,

    /// Successful snipes
    pub successful_snipes: u32,

    /// Total profit in SOL
    pub total_profit_sol: f64,

    /// Total loss in SOL
    pub total_loss_sol: f64,

    /// Win rate percentage
    pub win_rate: f64,

    /// Average execution time in ms
    pub avg_execution_time_ms: f64,

    /// Best multiplier achieved
    pub best_multiplier: f64,

    /// Rugs avoided
    pub rugs_avoided: u32,

    /// Today's stats
    pub today: DailyStats,
}

/// Daily statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    /// Snipes today
    pub snipes: u32,

    /// Profit today in SOL
    pub profit_sol: f64,

    /// Loss today in SOL
    pub loss_sol: f64,

    /// Win rate today
    pub win_rate: f64,
}

impl Default for SniperStats {
    fn default() -> Self {
        Self {
            total_snipes: 0,
            successful_snipes: 0,
            total_profit_sol: 0.0,
            total_loss_sol: 0.0,
            win_rate: 0.0,
            avg_execution_time_ms: 0.0,
            best_multiplier: 0.0,
            rugs_avoided: 0,
            today: DailyStats {
                snipes: 0,
                profit_sol: 0.0,
                loss_sol: 0.0,
                win_rate: 0.0,
            },
        }
    }
}
