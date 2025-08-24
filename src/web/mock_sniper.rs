//! Mock sniper for testing transaction flow

use base64::Engine;
use log::{info, warn};
use std::sync::Arc;
use tokio::time::{interval, Duration};

use super::{enhanced_websocket::EnhancedWebSocketState, TransactionMetadata};

/// Mock sniper that generates test transaction requests
pub struct MockSniper {
    enhanced_ws_state: EnhancedWebSocketState,
    enabled: Arc<tokio::sync::RwLock<bool>>,
}

impl MockSniper {
    pub fn new(enhanced_ws_state: EnhancedWebSocketState) -> Self {
        Self {
            enhanced_ws_state,
            enabled: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }

    /// Start the mock sniper
    pub async fn start(&self) {
        *self.enabled.write().await = true;

        let enhanced_ws_state = self.enhanced_ws_state.clone();
        let enabled = self.enabled.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(15)); // Generate transaction every 15 seconds
            let mut counter = 0u32;

            info!("🎯 Mock sniper started - will generate test transactions");

            loop {
                interval.tick().await;

                if !*enabled.read().await {
                    break;
                }

                // Check if any wallets have auto-snipe enabled
                let auto_snipe_wallets = enhanced_ws_state.get_auto_snipe_wallets().await;

                if auto_snipe_wallets.is_empty() {
                    continue;
                }

                // Generate mock transaction for first wallet
                if let Some(wallet_address) = auto_snipe_wallets.first() {
                    counter += 1;

                    if let Err(e) = Self::generate_mock_transaction(
                        &enhanced_ws_state,
                        wallet_address.clone(),
                        counter,
                    )
                    .await
                    {
                        warn!("Failed to generate mock transaction: {}", e);
                    }
                }
            }

            info!("🛑 Mock sniper stopped");
        });
    }

    /// Stop the mock sniper
    pub async fn stop(&self) {
        *self.enabled.write().await = false;
    }

    /// Generate a mock transaction request
    async fn generate_mock_transaction(
        enhanced_ws_state: &EnhancedWebSocketState,
        wallet_address: String,
        counter: u32,
    ) -> Result<(), String> {
        // Mock token data
        let tokens = [
            ("BONK", "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"),
            ("PEPE", "BzKR1FjdKhGhJhHKjKjKjKjKjKjKjKjKjKjKjKjKjKjK"),
            ("WIF", "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm"),
            ("POPCAT", "7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr"),
            ("MEW", "MEW1gQWJ3nEXg2qgERiKu7FAFj79PHvQVREQUzScPP5"),
        ];

        let token_index = (counter as usize) % tokens.len();
        let (symbol, mint) = tokens[token_index];

        // Generate mock transaction data (base64 encoded)
        let mock_transaction_bytes = vec![
            0x01, 0x00, 0x01, 0x03, // Transaction header
            0x44, 0x44, 0x44, 0x44, // Mock signature
            0x55, 0x55, 0x55, 0x55, // Mock pubkey
            0x66, 0x66, 0x66, 0x66, // Mock instruction data
        ];
        let transaction_data =
            base64::engine::general_purpose::STANDARD.encode(&mock_transaction_bytes);

        // Mock metadata
        let metadata = TransactionMetadata {
            estimated_gas: Some(0.001),
            price_impact: Some(2.5),
            slippage: Some(1.0),
            priority_fee: Some(0.0001),
            max_compute_units: Some(200_000),
        };

        // Determine transaction type based on counter
        let (transaction_type, amount) = match counter % 3 {
            0 => ("snipe", "1.0 SOL"),
            1 => ("sell", "50%"),
            _ => ("swap", "0.5 SOL"),
        };

        info!(
            "🎯 Generating mock {} transaction for {} ({})",
            transaction_type,
            symbol,
            &wallet_address[..8]
        );

        // Create transaction request
        enhanced_ws_state
            .create_transaction_request(
                wallet_address,
                transaction_type.to_string(),
                format!("{}#{}", symbol, counter),
                mint.to_string(),
                amount.to_string(),
                transaction_data,
                Some(metadata),
            )
            .await?;

        Ok(())
    }
}

/// Start mock sniper for testing
pub async fn start_mock_sniper(enhanced_ws_state: EnhancedWebSocketState) {
    let mock_sniper = MockSniper::new(enhanced_ws_state);
    mock_sniper.start().await;

    // Keep the mock sniper running
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        mock_sniper.stop().await;
    });
}
