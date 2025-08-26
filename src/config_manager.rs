//! Enhanced Configuration Module with Full Type Safety
//! Centralized configuration management for the bot

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::Result;

/// Main configuration structure with all settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BotConfig {
    pub network: NetworkConfig,
    pub wallet: WalletConfig,
    pub dex: DexConfig,
    pub trading: TradingConfig,
    pub safety: SafetyConfig,
    pub monitoring: MonitoringConfig,
    pub notifications: NotificationConfig,
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkConfig {
    pub rpc_url: String,
    pub ws_url: String,
    pub network_type: NetworkType,
    pub commitment_level: String,
    pub max_retries: u32,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum NetworkType {
    Mainnet,
    Devnet,
    Testnet,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WalletConfig {
    pub wallet_path: String,
    pub use_ledger: bool,
    pub ledger_derivation_path: String,
    pub max_auto_approve_usd: Decimal,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DexConfig {
    pub raydium: DexInfo,
    pub orca: DexInfo,
    pub jupiter: JupiterConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DexInfo {
    pub program_id: String,
    pub pool_address: String,
    pub fee_percent: Decimal,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JupiterConfig {
    pub api_url: String,
    pub max_slippage_bps: u16,
    pub use_versioned_tx: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradingConfig {
    pub max_position_sol: Decimal,
    pub min_profit_usd: Decimal,
    pub min_profit_percent: Decimal,
    pub max_slippage_percent: Decimal,
    pub max_daily_trades: u32,
    pub max_daily_loss_usd: Decimal,
    pub priority_fee_lamports: u64,
    pub simulation_required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SafetyConfig {
    pub enable_safety_checks: bool,
    pub min_liquidity_sol: Decimal,
    pub max_market_cap_usd: Decimal,
    pub min_holders: u32,
    pub max_dev_percentage: Decimal,
    pub max_token_age_minutes: u32,
    pub max_buy_tax_percent: Decimal,
    pub max_sell_tax_percent: Decimal,
    pub blacklisted_creators: Vec<String>,
    pub blacklist_keywords: Vec<String>,
    pub circuit_breaker: CircuitBreakerConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_seconds: u64,
    pub cooldown_minutes: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringConfig {
    pub enable_monitoring: bool,
    pub price_update_interval_ms: u64,
    pub cache_duration_ms: u64,
    pub enable_post_trade_monitoring: bool,
    pub rug_pull_detection: RugPullConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RugPullConfig {
    pub enable: bool,
    pub lp_drain_threshold_percent: Decimal,
    pub check_interval_seconds: u64,
    pub emergency_sell_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotificationConfig {
    pub discord: Option<DiscordConfig>,
    pub telegram: Option<TelegramConfig>,
    pub email: Option<EmailConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DiscordConfig {
    pub enabled: bool,
    pub webhook_url: String,
    pub alert_on_profit: bool,
    pub alert_on_loss: bool,
    pub alert_on_error: bool,
    pub min_profit_alert_usd: Decimal,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token: String,
    pub chat_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmailConfig {
    pub enabled: bool,
    pub smtp_server: String,
    pub from_address: String,
    pub to_addresses: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    pub enable_optimizations: bool,
    pub parallel_requests: usize,
    pub cache_size_mb: usize,
    pub max_memory_mb: usize,
}

impl BotConfig {
    /// Load configuration from YAML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: BotConfig = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate trading limits
        if self.trading.max_position_sol <= Decimal::ZERO {
            anyhow::bail!("max_position_sol must be positive");
        }
        
        if self.trading.min_profit_percent < Decimal::ZERO {
            anyhow::bail!("min_profit_percent cannot be negative");
        }
        
        // Validate safety thresholds
        if self.safety.max_buy_tax_percent > Decimal::from(100) {
            anyhow::bail!("max_buy_tax_percent cannot exceed 100%");
        }
        
        // Validate network settings
        if self.network.timeout_seconds == 0 {
            anyhow::bail!("timeout_seconds must be greater than 0");
        }
        
        Ok(())
    }

    /// Merge with environment variables
    pub fn with_env_overrides(mut self) -> Self {
        // Override from environment variables
        if let Ok(rpc_url) = std::env::var("BOT_RPC_URL") {
            self.network.rpc_url = rpc_url;
        }
        
        if let Ok(max_position) = std::env::var("BOT_MAX_POSITION_SOL") {
            if let Ok(val) = max_position.parse::<f64>() {
                self.trading.max_position_sol = Decimal::from_f64_retain(val).unwrap_or(self.trading.max_position_sol);
            }
        }
        
        if let Ok(webhook) = std::env::var("DISCORD_WEBHOOK_URL") {
            if let Some(discord) = &mut self.notifications.discord {
                discord.webhook_url = webhook;
            }
        }
        
        self
    }

    /// Get default configuration
    pub fn default() -> Self {
        Self {
            network: NetworkConfig {
                rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
                ws_url: "wss://api.mainnet-beta.solana.com".to_string(),
                network_type: NetworkType::Mainnet,
                commitment_level: "confirmed".to_string(),
                max_retries: 3,
                timeout_seconds: 30,
            },
            wallet: WalletConfig {
                wallet_path: "wallet.json".to_string(),
                use_ledger: false,
                ledger_derivation_path: "m/44'/501'/0'/0'".to_string(),
                max_auto_approve_usd: Decimal::from(100),
            },
            dex: DexConfig {
                raydium: DexInfo {
                    program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string(),
                    pool_address: "".to_string(),
                    fee_percent: Decimal::from_f64_retain(0.25).unwrap(),
                },
                orca: DexInfo {
                    program_id: "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string(),
                    pool_address: "".to_string(),
                    fee_percent: Decimal::from_f64_retain(0.30).unwrap(),
                },
                jupiter: JupiterConfig {
                    api_url: "https://quote-api.jup.ag/v6".to_string(),
                    max_slippage_bps: 50,
                    use_versioned_tx: true,
                },
            },
            trading: TradingConfig {
                max_position_sol: Decimal::from(10),
                min_profit_usd: Decimal::from(1),
                min_profit_percent: Decimal::from_f64_retain(0.3).unwrap(),
                max_slippage_percent: Decimal::from_f64_retain(0.5).unwrap(),
                max_daily_trades: 30,
                max_daily_loss_usd: Decimal::from(100),
                priority_fee_lamports: 10000,
                simulation_required: true,
            },
            safety: SafetyConfig {
                enable_safety_checks: true,
                min_liquidity_sol: Decimal::from(5),
                max_market_cap_usd: Decimal::from(1000000),
                min_holders: 100,
                max_dev_percentage: Decimal::from(10),
                max_token_age_minutes: 60,
                max_buy_tax_percent: Decimal::from(5),
                max_sell_tax_percent: Decimal::from(5),
                blacklisted_creators: vec![],
                blacklist_keywords: vec!["scam".to_string(), "rug".to_string()],
                circuit_breaker: CircuitBreakerConfig {
                    failure_threshold: 5,
                    success_threshold: 3,
                    timeout_seconds: 300,
                    cooldown_minutes: 60,
                },
            },
            monitoring: MonitoringConfig {
                enable_monitoring: true,
                price_update_interval_ms: 100,
                cache_duration_ms: 50,
                enable_post_trade_monitoring: true,
                rug_pull_detection: RugPullConfig {
                    enable: true,
                    lp_drain_threshold_percent: Decimal::from(20),
                    check_interval_seconds: 30,
                    emergency_sell_enabled: true,
                },
            },
            notifications: NotificationConfig {
                discord: None,
                telegram: None,
                email: None,
            },
            performance: PerformanceConfig {
                enable_optimizations: true,
                parallel_requests: 4,
                cache_size_mb: 100,
                max_memory_mb: 512,
            },
        }
    }
}
