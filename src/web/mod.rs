//! Web Dashboard Module
//! Provides HTTP API and WebSocket connections for the Solana Arbitrage Bot dashboard

pub mod server;
pub mod handlers;
pub mod websocket;
pub mod auth;
pub mod database;

pub use server::WebServer;
pub use database::Database;

use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

/// Configuration for the web dashboard
#[derive(Debug, Clone, Deserialize)]
pub struct WebConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub auth_token: Option<String>,
    pub database_path: String,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 3001,
            auth_token: None,
            database_path: "./dashboard.db".to_string(),
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
    
    #[serde(rename = "error")]
    Error { message: String },
    
    #[serde(rename = "ping")]
    Ping,
    
    #[serde(rename = "pong")]
    Pong,
}
