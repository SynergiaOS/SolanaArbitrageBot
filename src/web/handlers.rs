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

use super::{
    server::AppState,
    ApiResponse,
    BotStatus,
    BotConfig,
    WebSocketMessage,
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
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<BotConfig>>, StatusCode> {
    let config = state.runtime_config.read().await.clone();
    Ok(Json(ApiResponse::success(config)))
}

/// Update bot configuration
pub async fn update_bot_config(
    State(state): State<AppState>,
    Json(new_config): Json<BotConfig>,
) -> Result<(StatusCode, Json<ApiResponse<String>>), StatusCode> {
    // Log change with timestamp and source
    let now = Utc::now();
    let old_config = {
        state.runtime_config.read().await.clone()
    };

    // Validate input
    let mut errors: Vec<String> = Vec::new();
    if new_config.min_profit_usd <= Decimal::ZERO {
        errors.push("min_profit_usd must be > 0".to_string());
    }
    if new_config.max_position_sol < Decimal::from_f64_retain(0.001).unwrap() || new_config.max_position_sol > Decimal::from_f64_retain(10.0).unwrap() {
        errors.push("max_position_sol must be between 0.001 and 10.0 SOL".to_string());
    }
    if new_config.max_daily_trades == 0 || new_config.max_daily_trades > 1000 {
        errors.push("max_daily_trades must be between 1 and 1000".to_string());
    }
    if new_config.max_daily_loss_usd <= Decimal::ZERO {
        errors.push("max_daily_loss_usd must be > 0".to_string());
    }

    if !errors.is_empty() {
        warn!("[{}][CONFIG][API] Validation failed: {:?}", now.to_rfc3339(), errors);
        // Return 400 with JSON error body
        let body = ApiResponse::<String>::error(errors.join("; "));
        return Ok((StatusCode::BAD_REQUEST, Json(body)));
    }

    info!(
        "[{}][CONFIG][API] Update accepted: old={:?} -> new={:?}",
        now.to_rfc3339(), old_config, new_config
    );

    // Update runtime config in memory
    {
        let mut cfg = state.runtime_config.write().await;
        *cfg = new_config.clone();
    }

    // Emit WebSocket config update notification
    let msg = WebSocketMessage::ConfigUpdate {
        timestamp: now,
        source: "API".to_string(),
        old: old_config,
        new: new_config,
    };
    super::server::broadcast_websocket_message(&state.websocket_tx, msg).await;

    Ok((StatusCode::OK, Json(ApiResponse::success("Configuration updated successfully".to_string()))))
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
    axum::Json(body): axum::Json<Option<serde_json::Value>>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let now = chrono::Utc::now();
    let (reason, source) = match body {
        Some(v) => (
            v.get("reason").and_then(|x| x.as_str()).unwrap_or("unspecified").to_string(),
            v.get("source").and_then(|x| x.as_str()).unwrap_or("api").to_string(),
        ),
        None => ("unspecified".to_string(), "api".to_string()),
    };

    warn!("[{}][EMERGENCY][{}] reason={}", now.to_rfc3339(), source, reason);

    let mut bot_running = state.bot_running.write().await;
    *bot_running = false;

    // TODO: Implement additional emergency stop logic
    // - Cancel pending transactions
    // - Close positions
    // - Send Discord alert

    Ok(Json(ApiResponse::success("Emergency stop executed".to_string())))
}
