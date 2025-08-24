//! Web Dashboard Module
//! Provides HTTP API and WebSocket connections for the Solana Arbitrage Bot dashboard

pub mod auth;
pub mod database;
pub mod enhanced_websocket;
pub mod handlers;
pub mod mock_sniper;
pub mod server;
pub mod rate_limit;
pub mod metrics;
pub mod websocket;

pub use database::Database;
pub use enhanced_websocket::{enhanced_websocket_handler, EnhancedWebSocketState};
pub use server::WebServer;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeysConfig {
    pub admin_api_key: Option<String>,
    pub config_api_key: Option<String>,
    pub status_api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitsConfig {
    pub default_per_ip_per_minute: Option<u64>,
    pub control_per_ip_per_minute: Option<u64>,
    pub config_per_ip_per_minute: Option<u64>,
    pub status_per_ip_per_minute: Option<u64>,
}


/// Configuration for the web dashboard
#[derive(Debug, Clone, Deserialize)]
pub struct WebConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub auth_token: Option<String>,
    pub database_path: String,
    pub allowed_origins: Option<Vec<String>>, // CORS whitelist for production
    pub rate_limits: Option<RateLimitsConfig>,
    pub api_keys: Option<ApiKeysConfig>,
    pub control_ip_allowlist: Option<Vec<String>>, // CIDR or IP strings
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 3001,
            auth_token: None,
            database_path: "./dashboard.db".to_string(),
            allowed_origins: Some(vec![
                "http://localhost:3000".to_string(),
                "http://127.0.0.1:3000".to_string(),
            ]),
            rate_limits: Some(RateLimitsConfig {
                default_per_ip_per_minute: Some(100),
                control_per_ip_per_minute: Some(10),
                config_per_ip_per_minute: Some(20),
                status_per_ip_per_minute: Some(100),
            }),
            api_keys: None,
            control_ip_allowlist: None,
        }
    }
}

/// Bot status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotStatus {
    pub running: bool,
    pub mode: String, // "LIVE" or "DRY_RUN"
    pub uptime_seconds: u64,
    pub last_update: DateTime<Utc>,
    pub raydium_price: Option<Decimal>,
    pub orca_price: Option<Decimal>,
    pub spread_percent: Option<Decimal>,
    pub trades_today: u32,
    pub profit_today: Decimal,
}

/// Transaction record for the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub id: Option<i64>,
    pub timestamp: DateTime<Utc>,
    pub signature: String,
    pub buy_dex: String,
    pub sell_dex: String,
    pub amount_sol: Decimal,
    pub profit_usd: Decimal,
    pub raydium_price: Decimal,
    pub orca_price: Decimal,
    pub spread_percent: Decimal,
    pub gas_fee: Decimal,
}

/// Price update for WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceUpdate {
    pub timestamp: DateTime<Utc>,
    pub raydium_price: Decimal,
    pub orca_price: Decimal,
    pub spread_percent: Decimal,
}

/// Bot configuration that can be updated via API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub min_profit_usd: Decimal,
    pub max_position_sol: Decimal,
    pub max_daily_trades: u32,
    pub max_daily_loss_usd: Decimal,
    pub enabled: bool,
}

/// API Response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    #[serde(rename = "price_update")]
    PriceUpdate(PriceUpdate),

    #[serde(rename = "bot_status")]
    BotStatus(BotStatus),

    #[serde(rename = "transaction")]
    Transaction(TransactionRecord),

    #[serde(rename = "config_update")]
    ConfigUpdate {
        timestamp: DateTime<Utc>,
        source: String,
        old: BotConfig,
        new: BotConfig,
    },

    #[serde(rename = "error")]
    Error { message: String },

    #[serde(rename = "ping")]
    Ping,

    #[serde(rename = "pong")]
    Pong,
}

/// Enhanced WebSocket message types for transaction handling
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EnhancedWebSocketMessage {
    // Client → Server messages
    #[serde(rename = "register_wallet")]
    RegisterWallet { wallet_address: String },

    #[serde(rename = "enable_auto_snipe")]
    EnableAutoSnipe { wallet_address: String },

    #[serde(rename = "disable_auto_snipe")]
    DisableAutoSnipe { wallet_address: String },

    #[serde(rename = "transaction_response")]
    TransactionResponse {
        transaction_id: String,
        signature: String,
        success: bool,
        error: Option<String>,
        wallet_address: String,
    },

    // Server → Client messages
    #[serde(rename = "transaction_request")]
    TransactionRequest {
        id: String,
        transaction_type: String, // "snipe", "sell", "swap"
        token_symbol: String,
        token_mint: String,
        amount: String,
        transaction_data: String, // Base64 encoded transaction
        metadata: Option<TransactionMetadata>,
    },

    #[serde(rename = "transaction_cancelled")]
    TransactionCancelled {
        transaction_id: String,
        reason: String,
    },

    #[serde(rename = "auto_snipe_status")]
    AutoSnipeStatus {
        enabled: bool,
        wallet_address: String,
    },

    #[serde(rename = "arbitrage_opportunity")]
    ArbitrageOpportunity {
        id: String,
        buy_dex: String,
        sell_dex: String,
        buy_price: f64,
        sell_price: f64,
        profit_usd: f64,
        profit_percentage: f64,
        confidence_score: f64,
        amount_sol: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    #[serde(rename = "error")]
    Error {
        message: String,
        code: Option<String>,
    },

    #[serde(rename = "ping")]
    Ping,

    #[serde(rename = "pong")]
    Pong,
}

/// Transaction metadata for enhanced WebSocket messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMetadata {
    pub estimated_gas: Option<f64>,
    pub price_impact: Option<f64>,
    pub slippage: Option<f64>,
    pub priority_fee: Option<f64>,
    pub max_compute_units: Option<u32>,
}
