//! Enhanced WebSocket handler for transaction requests and wallet management

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{IntoResponse, Response},
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use log::{debug, error, info, warn};

// Security constants
const MAX_MESSAGE_SIZE: usize = 2048; // 2KB max message size for enhanced WS
const MAX_MESSAGES_PER_MINUTE: u32 = 120; // Higher rate limit for enhanced features
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, Duration};
use uuid;

use super::{server::AppState, EnhancedWebSocketMessage, TransactionMetadata};

/// Connected wallet information
#[derive(Debug, Clone)]
pub struct ConnectedWallet {
    pub address: String,
    pub auto_snipe_enabled: bool,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Enhanced WebSocket state for transaction management
#[derive(Clone)]
pub struct EnhancedWebSocketState {
    pub app_state: AppState,
    pub connected_wallets: Arc<RwLock<HashMap<String, ConnectedWallet>>>,
    pub pending_transactions: Arc<RwLock<HashMap<String, PendingTransaction>>>,
    pub enhanced_tx: broadcast::Sender<EnhancedWebSocketMessage>,
}

/// Pending transaction information
#[derive(Debug, Clone)]
pub struct PendingTransaction {
    pub id: String,
    pub wallet_address: String,
    pub transaction_type: String,
    pub token_symbol: String,
    pub token_mint: String,
    pub amount: String,
    pub transaction_data: String,
    pub metadata: Option<TransactionMetadata>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl EnhancedWebSocketState {
    pub fn new(app_state: AppState) -> Self {
        let (enhanced_tx, _) = broadcast::channel(1000);

        Self {
            app_state,
            connected_wallets: Arc::new(RwLock::new(HashMap::new())),
            pending_transactions: Arc::new(RwLock::new(HashMap::new())),
            enhanced_tx,
        }
    }

    /// Register a wallet connection
    pub async fn register_wallet(&self, address: String) {
        let wallet = ConnectedWallet {
            address: address.clone(),
            auto_snipe_enabled: false,
            connected_at: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
        };

        let mut wallets = self.connected_wallets.write().await;
        wallets.insert(address.clone(), wallet);

        info!("🔗 Wallet registered: {}", &address[..8]);
    }

    /// Enable auto-snipe for a wallet
    pub async fn enable_auto_snipe(&self, address: String) -> Result<(), String> {
        let mut wallets = self.connected_wallets.write().await;

        if let Some(wallet) = wallets.get_mut(&address) {
            wallet.auto_snipe_enabled = true;
            wallet.last_seen = chrono::Utc::now();

            info!("🎯 Auto-snipe enabled for wallet: {}", &address[..8]);

            // Broadcast status update
            let status_msg = EnhancedWebSocketMessage::AutoSnipeStatus {
                enabled: true,
                wallet_address: address,
            };

            if let Err(e) = self.enhanced_tx.send(status_msg) {
                warn!("Failed to broadcast auto-snipe status: {}", e);
            }

            Ok(())
        } else {
            Err("Wallet not registered".to_string())
        }
    }

    /// Disable auto-snipe for a wallet
    pub async fn disable_auto_snipe(&self, address: String) -> Result<(), String> {
        let mut wallets = self.connected_wallets.write().await;

        if let Some(wallet) = wallets.get_mut(&address) {
            wallet.auto_snipe_enabled = false;
            wallet.last_seen = chrono::Utc::now();

            info!("🛑 Auto-snipe disabled for wallet: {}", &address[..8]);

            // Broadcast status update
            let status_msg = EnhancedWebSocketMessage::AutoSnipeStatus {
                enabled: false,
                wallet_address: address,
            };

            if let Err(e) = self.enhanced_tx.send(status_msg) {
                warn!("Failed to broadcast auto-snipe status: {}", e);
            }

            Ok(())
        } else {
            Err("Wallet not registered".to_string())
        }
    }

    /// Create a transaction request for a wallet
    pub async fn create_transaction_request(
        &self,
        wallet_address: String,
        transaction_type: String,
        token_symbol: String,
        token_mint: String,
        amount: String,
        transaction_data: String,
        metadata: Option<TransactionMetadata>,
    ) -> Result<String, String> {
        // Check if wallet is connected and has auto-snipe enabled
        {
            let wallets = self.connected_wallets.read().await;
            if let Some(wallet) = wallets.get(&wallet_address) {
                if !wallet.auto_snipe_enabled {
                    return Err("Auto-snipe not enabled for this wallet".to_string());
                }
            } else {
                return Err("Wallet not connected".to_string());
            }
        }

        let transaction_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let pending_tx = PendingTransaction {
            id: transaction_id.clone(),
            wallet_address: wallet_address.clone(),
            transaction_type: transaction_type.clone(),
            token_symbol: token_symbol.clone(),
            token_mint: token_mint.clone(),
            amount: amount.clone(),
            transaction_data: transaction_data.clone(),
            metadata: metadata.clone(),
            created_at: now,
            expires_at: now + chrono::Duration::minutes(5), // 5 minute expiry
        };

        // Store pending transaction
        {
            let mut pending = self.pending_transactions.write().await;
            pending.insert(transaction_id.clone(), pending_tx);
        }

        // Send transaction request to frontend
        let request_msg = EnhancedWebSocketMessage::TransactionRequest {
            id: transaction_id.clone(),
            transaction_type,
            token_symbol: token_symbol.clone(),
            token_mint,
            amount,
            transaction_data,
            metadata,
        };

        if let Err(e) = self.enhanced_tx.send(request_msg) {
            error!("Failed to send transaction request: {}", e);
            return Err("Failed to send transaction request".to_string());
        }

        info!(
            "📝 Transaction request created: {} for {} ({})",
            &transaction_id[..8],
            token_symbol,
            &wallet_address[..8]
        );

        Ok(transaction_id)
    }

    /// Handle transaction response from frontend
    pub async fn handle_transaction_response(
        &self,
        transaction_id: String,
        signature: String,
        success: bool,
        error: Option<String>,
        wallet_address: String,
    ) -> Result<(), String> {
        // Remove from pending transactions
        let pending_tx = {
            let mut pending = self.pending_transactions.write().await;
            pending.remove(&transaction_id)
        };

        if let Some(_tx) = pending_tx {
            if success {
                info!(
                    "✅ Transaction completed: {} with signature: {}",
                    &transaction_id[..8],
                    &signature[..8]
                );

                // TODO: Store successful transaction in database
                // TODO: Update bot statistics
                // TODO: Notify other systems
            } else {
                warn!(
                    "❌ Transaction failed: {} - {}",
                    &transaction_id[..8],
                    error.as_deref().unwrap_or("Unknown error")
                );

                // TODO: Handle failed transaction
                // TODO: Update failure statistics
            }

            // Update wallet last seen
            {
                let mut wallets = self.connected_wallets.write().await;
                if let Some(wallet) = wallets.get_mut(&wallet_address) {
                    wallet.last_seen = chrono::Utc::now();
                }
            }

            Ok(())
        } else {
            Err("Transaction not found or already processed".to_string())
        }
    }

    /// Clean up expired transactions
    pub async fn cleanup_expired_transactions(&self) {
        let now = chrono::Utc::now();
        let mut expired_ids = Vec::new();

        {
            let pending = self.pending_transactions.read().await;
            for (id, tx) in pending.iter() {
                if now > tx.expires_at {
                    expired_ids.push(id.clone());
                }
            }
        }

        if !expired_ids.is_empty() {
            let mut pending = self.pending_transactions.write().await;
            for id in &expired_ids {
                if let Some(tx) = pending.remove(id) {
                    warn!(
                        "⏰ Transaction expired: {} for {}",
                        &id[..8],
                        tx.token_symbol
                    );

                    // Send cancellation message
                    let cancel_msg = EnhancedWebSocketMessage::TransactionCancelled {
                        transaction_id: id.clone(),
                        reason: "Transaction expired".to_string(),
                    };

                    if let Err(e) = self.enhanced_tx.send(cancel_msg) {
                        warn!("Failed to send cancellation message: {}", e);
                    }
                }
            }

            info!("🧹 Cleaned up {} expired transactions", expired_ids.len());
        }
    }

    /// Get connected wallets with auto-snipe enabled
    pub async fn get_auto_snipe_wallets(&self) -> Vec<String> {
        let wallets = self.connected_wallets.read().await;
        wallets
            .values()
            .filter(|w| w.auto_snipe_enabled)
            .map(|w| w.address.clone())
            .collect()
    }

    /// Broadcast arbitrage opportunity to all connected clients
    pub async fn broadcast_opportunity(
        &self,
        opportunity: EnhancedWebSocketMessage,
    ) -> Result<(), String> {
        if let Err(e) = self.enhanced_tx.send(opportunity) {
            return Err(format!("Failed to broadcast opportunity: {}", e));
        }
        Ok(())
    }
}

/// Handle individual enhanced WebSocket connection
async fn enhanced_websocket_connection(socket: WebSocket, state: EnhancedWebSocketState) {
    info!("🔌 Enhanced WebSocket connection established");

    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(tokio::sync::Mutex::new(sender));

    // Subscribe to broadcast messages
    let mut enhanced_rx = state.enhanced_tx.subscribe();

    // Spawn task to handle incoming messages from client
    let state_clone = state.clone();
    let sender_clone = sender.clone();
    let incoming_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Validate message size
                    if text.len() > MAX_MESSAGE_SIZE {
                        warn!("Enhanced WebSocket message too large: {} bytes (max: {})", text.len(), MAX_MESSAGE_SIZE);

                        // Send error response for oversized message
                        let error_msg = EnhancedWebSocketMessage::Error {
                            message: "Message too large".to_string(),
                            code: Some("MESSAGE_TOO_LARGE".to_string()),
                        };

                        if let Ok(json) = serde_json::to_string(&error_msg) {
                            let mut sender_guard = sender_clone.lock().await;
                            if let Err(send_err) = sender_guard.send(Message::Text(json)).await {
                                error!("Failed to send error message: {}", send_err);
                            }
                        }
                        break;
                    }

                    if let Err(e) = handle_enhanced_client_message(&text, &state_clone).await {
                        warn!("Error handling enhanced client message: {}", e);

                        // Send error response
                        let error_msg = EnhancedWebSocketMessage::Error {
                            message: e.to_string(),
                            code: None,
                        };

                        if let Ok(json) = serde_json::to_string(&error_msg) {
                            let mut sender_guard = sender_clone.lock().await;
                            if let Err(send_err) = sender_guard.send(Message::Text(json)).await {
                                error!("Failed to send error message: {}", send_err);
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("🔌 Enhanced WebSocket connection closed by client");
                    break;
                }
                Ok(Message::Ping(data)) => {
                    let mut sender_guard = sender_clone.lock().await;
                    if let Err(e) = sender_guard.send(Message::Pong(data)).await {
                        error!("Failed to send pong: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Enhanced WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Spawn task to send broadcast messages to client
    let outgoing_task = tokio::spawn(async move {
        // Set up periodic ping and cleanup
        let mut ping_interval = interval(Duration::from_secs(30));
        let mut cleanup_interval = interval(Duration::from_secs(60));

        loop {
            tokio::select! {
                // Handle broadcast messages
                msg = enhanced_rx.recv() => {
                    match msg {
                        Ok(ws_msg) => {
                            if let Ok(json) = serde_json::to_string(&ws_msg) {
                                let mut sender_guard = sender.lock().await;
                                if let Err(e) = sender_guard.send(Message::Text(json)).await {
                                    error!("Failed to send enhanced WebSocket message: {}", e);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Enhanced WebSocket broadcast receive error: {}", e);
                        }
                    }
                }

                // Send periodic ping
                _ = ping_interval.tick() => {
                    let ping_msg = EnhancedWebSocketMessage::Ping;
                    if let Ok(json) = serde_json::to_string(&ping_msg) {
                        let mut sender_guard = sender.lock().await;
                        if let Err(e) = sender_guard.send(Message::Text(json)).await {
                            error!("Failed to send ping: {}", e);
                            break;
                        }
                    }
                }

                // Periodic cleanup
                _ = cleanup_interval.tick() => {
                    state.cleanup_expired_transactions().await;
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = incoming_task => {
            info!("Enhanced WebSocket incoming task completed");
        }
        _ = outgoing_task => {
            info!("Enhanced WebSocket outgoing task completed");
        }
    }

    info!("🔌 Enhanced WebSocket connection closed");
}

/// Handle incoming messages from enhanced WebSocket client
async fn handle_enhanced_client_message(
    text: &str,
    state: &EnhancedWebSocketState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("Received enhanced WebSocket message: {}", text);

    let msg: EnhancedWebSocketMessage = serde_json::from_str(text)?;

    match msg {
        EnhancedWebSocketMessage::RegisterWallet { wallet_address } => {
            state.register_wallet(wallet_address).await;
        }

        EnhancedWebSocketMessage::EnableAutoSnipe { wallet_address } => {
            if let Err(e) = state.enable_auto_snipe(wallet_address).await {
                return Err(e.into());
            }
        }

        EnhancedWebSocketMessage::DisableAutoSnipe { wallet_address } => {
            if let Err(e) = state.disable_auto_snipe(wallet_address).await {
                return Err(e.into());
            }
        }

        EnhancedWebSocketMessage::TransactionResponse {
            transaction_id,
            signature,
            success,
            error,
            wallet_address,
        } => {
            if let Err(e) = state
                .handle_transaction_response(
                    transaction_id,
                    signature,
                    success,
                    error,
                    wallet_address,
                )
                .await
            {
                return Err(e.into());
            }
        }

        EnhancedWebSocketMessage::Ping => {
            // Client ping - respond with pong
            let pong_msg = EnhancedWebSocketMessage::Pong;
            if let Err(e) = state.enhanced_tx.send(pong_msg) {
                warn!("Failed to send pong response: {}", e);
            }
        }

        EnhancedWebSocketMessage::Pong => {
            // Client pong response
            debug!("Received pong from enhanced client");
        }

        _ => {
            warn!(
                "Unexpected enhanced WebSocket message from client: {:?}",
                msg
            );
        }
    }

    Ok(())
}

/// Enhanced WebSocket handler for transaction management
pub async fn enhanced_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<EnhancedWebSocketState>,
) -> impl IntoResponse {
    info!("🔗 New enhanced WebSocket connection request");
    ws.on_upgrade(move |socket| handle_enhanced_websocket(socket, state))
}

/// Handle enhanced WebSocket connection
async fn handle_enhanced_websocket(socket: WebSocket, state: EnhancedWebSocketState) {
    let (mut sender, mut receiver) = socket.split();
    let connection_id = uuid::Uuid::new_v4().to_string();

    info!("🔗 Enhanced WebSocket connected: {}", &connection_id[..8]);

    // Subscribe to enhanced broadcasts
    let mut enhanced_rx = state.enhanced_tx.subscribe();

    // Spawn task to handle outgoing messages
    let sender_task = {
        let connection_id = connection_id.clone();
        tokio::spawn(async move {
            while let Ok(msg) = enhanced_rx.recv().await {
                let json_msg = match serde_json::to_string(&msg) {
                    Ok(json) => json,
                    Err(e) => {
                        warn!("Failed to serialize enhanced message: {}", e);
                        continue;
                    }
                };

                if sender.send(Message::Text(json_msg)).await.is_err() {
                    info!(
                        "🔌 Enhanced WebSocket disconnected: {}",
                        &connection_id[..8]
                    );
                    break;
                }
            }
        })
    };

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Err(e) = handle_enhanced_client_message(&text, &state).await {
                    warn!("Error handling enhanced WebSocket message: {}", e);
                }
            }
            Ok(Message::Close(_)) => {
                info!("🔌 Enhanced WebSocket closed: {}", &connection_id[..8]);
                break;
            }
            Err(e) => {
                warn!("Enhanced WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    // Cleanup
    sender_task.abort();
    info!(
        "🧹 Enhanced WebSocket cleanup completed: {}",
        &connection_id[..8]
    );
}
