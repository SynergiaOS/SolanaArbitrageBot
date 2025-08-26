//! Solana Arbitrage Bot Library
//! Exports public types and modules for external use and testing

// Core modules that actually exist and compile cleanly
pub mod calculator;
pub mod config_manager;
pub mod utils;

// Re-export config structures for external use
pub use config_manager::{
    BotConfig, NetworkConfig, WalletConfig, DexConfig, DexInfo, JupiterConfig,
    TradingConfig, SafetyConfig, MonitoringConfig, NotificationConfig, PerformanceConfig,
    CircuitBreakerConfig, RugPullConfig, DiscordConfig, TelegramConfig, EmailConfig,
    NetworkType
};

// Aliases for backward compatibility with existing binaries
pub type Config = BotConfig;
pub type ExecutionConfig = TradingConfig;
pub type LimitsConfig = TradingConfig;
pub type RpcConfig = NetworkConfig;

// Temporarily disabled problematic modules for debugging
#[cfg(feature = "full")]
pub mod discord;

#[cfg(feature = "full")]
pub mod docx_reader;

#[cfg(feature = "full")]  
pub mod executor;

#[cfg(feature = "full")]
pub mod ledger;

#[cfg(any(feature = "monitor", feature = "full"))]
pub mod monitor;

#[cfg(any(feature = "monitor", feature = "full"))]
pub mod post_trade_monitor;

#[cfg(feature = "full")]
pub mod safety;

#[cfg(feature = "full")]
pub mod verification;

// Feature-gated modules
#[cfg(feature = "sniper")]
pub mod sniper;

#[cfg(feature = "gepa")]
pub mod gepa;

#[cfg(feature = "kestra")]
pub mod kestra;

#[cfg(feature = "web")]
pub mod web;

#[cfg(feature = "architecture")]
pub mod architecture;

#[cfg(feature = "performance")]
pub mod performance;

#[cfg(feature = "security")]
pub mod security;

#[cfg(feature = "testing")]
pub mod testing;
