//! WebSocket handler for real-time updates

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use log::{error, info, warn};
use tokio::time::{interval, Duration};

use super::{server::AppState, WebSocketMessage};

// Security constants
const MAX_MESSAGE_SIZE: usize = 1024; // 1KB max message size
const MAX_MESSAGES_PER_MINUTE: u32 = 60; // Rate limiting
const WEBSOCKET_TIMEOUT: Duration = Duration::from_secs(300); // 5 minute timeout

/// WebSocket upgrade handler
pub async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    // Discord alert for dashboard connection
    if let Some(discord) = state.discord.clone() {
        tokio::spawn(async move {
            let _ = discord
                .send_dashboard_status("connected", "A dashboard client connected")
                .await;
        });
    }
    ws.on_upgrade(|socket| websocket_connection(socket, state))
}

/// Handle individual WebSocket connection
async fn websocket_connection(socket: WebSocket, state: AppState) {
    info!("🔌 New WebSocket connection established");

    let (sender, mut receiver) = socket.split();
    let sender = std::sync::Arc::new(tokio::sync::Mutex::new(sender));

    // Subscribe to broadcast messages
    let mut websocket_rx = state.websocket_tx.subscribe();

    // Spawn task to handle incoming messages from client with timeout
    let state_clone = state.clone();
    let sender_clone = sender.clone();
    let incoming_task = tokio::spawn(async move {
        let timeout_future = tokio::time::sleep(WEBSOCKET_TIMEOUT);
        tokio::pin!(timeout_future);
        loop {
            tokio::select! {
                msg = receiver.next() => {
                    match msg {
                        Some(msg) => match msg {
                Ok(Message::Text(text)) => {
                    // Validate message size
                    if text.len() > MAX_MESSAGE_SIZE {
                        warn!("Message too large: {} bytes (max: {})", text.len(), MAX_MESSAGE_SIZE);
                        break;
                    }

                    if let Err(e) = handle_client_message(&text, &state_clone).await {
                        warn!("Error handling client message: {}", e);
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("🔌 WebSocket connection closed by client");
                    break;
                }
                Ok(Message::Ping(data)) => {
                    // Echo back pong
                    let mut sender_guard = sender_clone.lock().await;
                    if let Err(e) = sender_guard.send(Message::Pong(data)).await {
                        error!("Failed to send pong: {}", e);
                        break;
                    }
                }
                            Err(e) => {
                                error!("WebSocket error: {}", e);
                                break;
                            }
                            _ => {}
                        },
                        None => {
                            info!("WebSocket stream ended");
                            break;
                        }
                    }
                }
                _ = &mut timeout_future => {
                    warn!("WebSocket connection timed out after {} seconds", WEBSOCKET_TIMEOUT.as_secs());
                    break;
                }
            }
        }
    });

    // Spawn task to send broadcast messages to client
    let state_for_outgoing = state.clone();
    let outgoing_task = tokio::spawn(async move {
        // Send initial status
        if let Err(e) = send_initial_status(&sender, &state_for_outgoing).await {
            error!("Failed to send initial status: {}", e);
            return;
        }

        // Set up periodic ping
        let mut ping_interval = interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                // Handle broadcast messages
                msg = websocket_rx.recv() => {
                    match msg {
                        Ok(ws_msg) => {
                            if let Ok(json) = serde_json::to_string(&ws_msg) {
                                let mut sender_guard = sender.lock().await;
                                if let Err(e) = sender_guard.send(Message::Text(json)).await {
                                    error!("Failed to send WebSocket message: {}", e);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            warn!("WebSocket broadcast receive error: {}", e);
                        }
                    }
                }

                // Send periodic ping
                _ = ping_interval.tick() => {
                    let ping_msg = WebSocketMessage::Ping;
                    if let Ok(json) = serde_json::to_string(&ping_msg) {
                        let mut sender_guard = sender.lock().await;
                        if let Err(e) = sender_guard.send(Message::Text(json)).await {
                            error!("Failed to send ping: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = incoming_task => {
            info!("WebSocket incoming task completed");
        }
        _ = outgoing_task => {
            info!("WebSocket outgoing task completed");
        }
    }

    // Discord alert for disconnect
    if let Some(ref discord) = state.discord {
        let _ = discord
            .send_dashboard_status("disconnected", "A dashboard client disconnected")
            .await;
    }
    info!("🔌 WebSocket connection closed");
}

/// Send initial status to newly connected client
async fn send_initial_status(
    sender: &std::sync::Arc<
        tokio::sync::Mutex<futures_util::stream::SplitSink<WebSocket, Message>>,
    >,
    state: &AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Send current bot status
    let raydium_price = *state.bot_state.raydium_price.lock().await;
    let orca_price = *state.bot_state.orca_price.lock().await;
    let trades_today = *state.bot_state.trades_today.lock().await;
    let profit_today = *state.bot_state.profit_today.lock().await;
    let bot_running = *state.bot_running.read().await;

    let status = super::BotStatus {
        running: bot_running,
        mode: if cfg!(debug_assertions) {
            "DRY_RUN".to_string()
        } else {
            "LIVE".to_string()
        },
        uptime_seconds: state.bot_start_time.elapsed().as_secs(),
        last_update: chrono::Utc::now(),
        raydium_price,
        orca_price,
        spread_percent: if let (Some(raydium), Some(orca)) = (raydium_price, orca_price) {
            if raydium > rust_decimal::Decimal::ZERO {
                Some(((orca - raydium).abs() / raydium) * rust_decimal::Decimal::from(100))
            } else {
                None
            }
        } else {
            None
        },
        trades_today,
        profit_today,
    };

    let status_msg = WebSocketMessage::BotStatus(status);
    let json = serde_json::to_string(&status_msg)?;
    let mut sender_guard = sender.lock().await;
    sender_guard.send(Message::Text(json)).await?;

    // Send current prices if available
    if let (Some(raydium), Some(orca)) = (raydium_price, orca_price) {
        let spread_percent = if raydium > rust_decimal::Decimal::ZERO {
            ((orca - raydium).abs() / raydium) * rust_decimal::Decimal::from(100)
        } else {
            rust_decimal::Decimal::ZERO
        };

        let price_update = super::PriceUpdate {
            timestamp: chrono::Utc::now(),
            raydium_price: raydium,
            orca_price: orca,
            spread_percent,
        };

        let price_msg = WebSocketMessage::PriceUpdate(price_update);
        let json = serde_json::to_string(&price_msg)?;
        sender_guard.send(Message::Text(json)).await?;
    }

    Ok(())
}

/// Handle incoming messages from client
async fn handle_client_message(
    text: &str,
    _state: &AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse client message
    if let Ok(msg) = serde_json::from_str::<WebSocketMessage>(text) {
        match msg {
            WebSocketMessage::Ping => {
                // Client ping - we'll respond with pong in the main loop
                info!("Received ping from client");
            }
            WebSocketMessage::Pong => {
                // Client pong response
                info!("Received pong from client");
            }
            _ => {
                warn!("Unexpected message from client: {:?}", msg);
            }
        }
    } else {
        warn!("Failed to parse client message: {}", text);
    }

    Ok(())
}
