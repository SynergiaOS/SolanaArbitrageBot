//! HTTP API handlers for the web dashboard

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use log::{info, warn};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::HashMap;

use super::{
    server::AppState,
    ApiResponse,
    BotStatus,
    BotConfig,
    TransactionRecord,
    database::DailyStats,
};

/// Query parameters for transactions endpoint
#[derive(Deserialize)]
pub struct TransactionQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Query parameters for stats endpoint
#[derive(Deserialize)]
pub struct StatsQuery {
    pub date: Option<String>,
}

/// Get current bot status
pub async fn get_bot_status(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<BotStatus>>, StatusCode> {
    let raydium_price = *state.bot_state.raydium_price.lock().await;
    let orca_price = *state.bot_state.orca_price.lock().await;
    let trades_today = *state.bot_state.trades_today.lock().await;
    let profit_today = *state.bot_state.profit_today.lock().await;
    let bot_running = *state.bot_running.read().await;

    // Calculate spread if both prices are available
    let spread_percent = if let (Some(raydium), Some(orca)) = (raydium_price, orca_price) {
        if raydium > Decimal::ZERO {
            Some(((orca - raydium).abs() / raydium) * Decimal::from(100))
        } else {
            None
        }
    } else {
        None
    };

    let status = BotStatus {
        running: bot_running,
        mode: if cfg!(debug_assertions) { "DRY_RUN".to_string() } else { "LIVE".to_string() },
        uptime_seconds: state.bot_start_time.elapsed().as_secs(),
        last_update: Utc::now(),
        raydium_price,
        orca_price,
        spread_percent,
        trades_today,
        profit_today,
    };

    Ok(Json(ApiResponse::success(status)))
}

/// Get current bot configuration
pub async fn get_bot_config(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<BotConfig>>, StatusCode> {
    // TODO: Load from actual config
    let config = BotConfig {
        min_profit_usd: Decimal::from(5),
        max_position_sol: Decimal::from(1),
        max_daily_trades: 50,
        max_daily_loss_usd: Decimal::from(100),
        enabled: true,
    };

    Ok(Json(ApiResponse::success(config)))
}

/// Update bot configuration
pub async fn update_bot_config(
    State(_state): State<AppState>,
    Json(config): Json<BotConfig>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    info!("📝 Updating bot configuration: {:?}", config);
    
    // TODO: Implement actual config update
    // This would need to update the bot's runtime configuration
    
    Ok(Json(ApiResponse::success("Configuration updated successfully".to_string())))
}

/// Get transaction history
pub async fn get_transactions(
    State(state): State<AppState>,
    Query(params): Query<TransactionQuery>,
) -> Result<Json<ApiResponse<Vec<TransactionRecord>>>, StatusCode> {
    let limit = params.limit.unwrap_or(50).min(1000); // Max 1000 records
    let offset = params.offset.unwrap_or(0);

    match state.database.get_transactions(limit, offset).await {
        Ok(transactions) => Ok(Json(ApiResponse::success(transactions))),
        Err(e) => {
            warn!("Failed to get transactions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get daily statistics
pub async fn get_daily_stats(
    State(state): State<AppState>,
    Query(params): Query<StatsQuery>,
) -> Result<Json<ApiResponse<Option<DailyStats>>>, StatusCode> {
    let date = params.date.unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());

    match state.database.get_daily_stats(&date).await {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(e) => {
            warn!("Failed to get daily stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Start the bot
pub async fn start_bot(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    info!("🚀 Starting bot via API");
    
    let mut bot_running = state.bot_running.write().await;
    *bot_running = true;
    
    Ok(Json(ApiResponse::success("Bot started successfully".to_string())))
}

/// Stop the bot
pub async fn stop_bot(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    info!("⏹️ Stopping bot via API");
    
    let mut bot_running = state.bot_running.write().await;
    *bot_running = false;
    
    Ok(Json(ApiResponse::success("Bot stopped successfully".to_string())))
}

/// Pause the bot
pub async fn pause_bot(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    info!("⏸️ Pausing bot via API");
    
    let mut bot_running = state.bot_running.write().await;
    *bot_running = false;
    
    Ok(Json(ApiResponse::success("Bot paused successfully".to_string())))
}

/// Emergency stop the bot
pub async fn emergency_stop(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    warn!("🚨 EMERGENCY STOP triggered via API");
    
    let mut bot_running = state.bot_running.write().await;
    *bot_running = false;
    
    // TODO: Implement additional emergency stop logic
    // - Cancel pending transactions
    // - Close positions
    // - Send Discord alert
    
    Ok(Json(ApiResponse::success("Emergency stop executed".to_string())))
}
