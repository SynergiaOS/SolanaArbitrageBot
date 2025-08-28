//! DEX Client Implementations
//! 
//! Concrete implementations of DexClient trait for various Solana DEXs.
//! Each client handles WebSocket connections, HTTP fallbacks, and data parsing
//! specific to their respective DEX APIs.

use super::{DexClient, DexId, PriceData, PriceUpdate, TradingPair, UpdateType};
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// Raydium DEX client implementation
#[derive(Debug)]
pub struct RaydiumClient {
    ws_endpoint: String,
    api_endpoint: String,
    http_client: reqwest::Client,
    update_sender: Arc<RwLock<Option<tokio::sync::mpsc::Sender<PriceUpdate>>>>,
}

impl RaydiumClient {
    pub fn new(ws_endpoint: String, api_endpoint: String) -> Self {
        Self {
            ws_endpoint,
            api_endpoint,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .pool_idle_timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            update_sender: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl DexClient for RaydiumClient {
    fn dex_id(&self) -> DexId {
        DexId::Raydium
    }

    async fn start_websocket(&self, pairs: Vec<TradingPair>) -> Result<()> {
        info!("Starting Raydium WebSocket for {} pairs", pairs.len());

        // Connect to Raydium WebSocket
        let ws_url = &self.ws_endpoint;
        let (ws_stream, _) = connect_async(ws_url)
            .await
            .context("Failed to connect to Raydium WebSocket")?;

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to all pairs
        for pair in &pairs {
            let subscribe_msg = serde_json::json!({
                "id": 1,
                "method": "poolSubscribe",
                "params": [pair.pool_address]
            });
            
            let msg = Message::Text(subscribe_msg.to_string());
            ws_sender.send(msg).await
                .context("Failed to send subscription message")?;
        }

        // Process incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) = self.process_raydium_message(&text, &pairs).await {
                        warn!("Failed to process Raydium message: {}", e);
                    }
                }
                Ok(Message::Ping(payload)) => {
                    // Respond to ping
                    if let Err(e) = ws_sender.send(Message::Pong(payload)).await {
                        error!("Failed to send pong: {}", e);
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("Raydium WebSocket closed");
                    break;
                }
                Err(e) => {
                    error!("Raydium WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn fetch_price(&self, pair: &TradingPair) -> Result<PriceData> {
        let url = format!("{}/pools/{}", self.api_endpoint, pair.pool_address);
        
        let response = self.http_client
            .get(&url)
            .timeout(Duration::from_millis(2000))
            .send()
            .await
            .context("Failed to fetch Raydium pool data")?;

        let json: Value = response.json().await
            .context("Failed to parse Raydium response")?;

        self.parse_raydium_pool_data(&json, pair)
    }

    async fn get_supported_pairs(&self) -> Result<Vec<TradingPair>> {
        let url = format!("{}/pools", self.api_endpoint);
        
        let response = self.http_client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch Raydium pools")?;

        let json: Value = response.json().await
            .context("Failed to parse Raydium pools response")?;

        let mut pairs = Vec::new();
        
        if let Some(pools) = json["data"].as_array() {
            for pool in pools {
                if let (Some(base_mint), Some(quote_mint), Some(pool_address)) = (
                    pool["baseMint"].as_str(),
                    pool["quoteMint"].as_str(),
                    pool["id"].as_str(),
                ) {
                    pairs.push(TradingPair::new(
                        base_mint.to_string(),
                        quote_mint.to_string(),
                        pool_address.to_string(),
                    ));
                }
            }
        }

        Ok(pairs)
    }

    async fn subscribe_updates(&self, sender: tokio::sync::mpsc::Sender<PriceUpdate>) -> Result<()> {
        let mut update_sender = self.update_sender.write().await;
        *update_sender = Some(sender);
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.api_endpoint);
        
        match self.http_client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await 
        {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

impl RaydiumClient {
    async fn process_raydium_message(&self, message: &str, pairs: &[TradingPair]) -> Result<()> {
        let json: Value = serde_json::from_str(message)
            .context("Failed to parse Raydium WebSocket message")?;

        // Handle different message types
        if let Some(method) = json["method"].as_str() {
            match method {
                "poolUpdate" => {
                    if let Some(params) = json["params"].as_object() {
                        self.handle_pool_update(params, pairs).await?;
                    }
                }
                _ => {
                    debug!("Unhandled Raydium message type: {}", method);
                }
            }
        }

        Ok(())
    }

    async fn handle_pool_update(&self, params: &serde_json::Map<String, Value>, pairs: &[TradingPair]) -> Result<()> {
        if let Some(pool_address) = params["poolId"].as_str() {
            // Find the matching trading pair
            if let Some(pair) = pairs.iter().find(|p| p.pool_address == pool_address) {
                let price_data = self.parse_raydium_pool_data(&Value::Object(params.clone()), pair)?;
                
                let update = PriceUpdate {
                    dex_id: DexId::Raydium,
                    pair: pair.clone(),
                    price_data,
                    update_type: UpdateType::Price,
                };

                // Send update if we have a subscriber
                if let Some(sender) = &*self.update_sender.read().await {
                    if let Err(e) = sender.send(update).await {
                        warn!("Failed to send Raydium price update: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_raydium_pool_data(&self, json: &Value, pair: &TradingPair) -> Result<PriceData> {
        let price = json["price"].as_f64()
            .or_else(|| json["basePrice"].as_f64())
            .context("Missing price in Raydium data")?;

        let volume_24h = json["volume24h"].as_f64().unwrap_or(0.0);
        let liquidity = json["liquidity"].as_f64().unwrap_or(0.0);

        Ok(PriceData {
            price: Decimal::try_from(price).unwrap_or_default(),
            volume_24h: Decimal::try_from(volume_24h).unwrap_or_default(),
            liquidity: Decimal::try_from(liquidity).unwrap_or_default(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            dex_id: DexId::Raydium,
            pair: pair.clone(),
        })
    }
}

/// Orca DEX client implementation
#[derive(Debug)]
pub struct OrcaClient {
    ws_endpoint: String,
    api_endpoint: String,
    http_client: reqwest::Client,
    update_sender: Arc<RwLock<Option<tokio::sync::mpsc::Sender<PriceUpdate>>>>,
}

impl OrcaClient {
    pub fn new(ws_endpoint: String, api_endpoint: String) -> Self {
        Self {
            ws_endpoint,
            api_endpoint,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            update_sender: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl OrcaClient for OrcaClient {
    fn dex_id(&self) -> DexId {
        DexId::Orca
    }

    async fn start_websocket(&self, pairs: Vec<TradingPair>) -> Result<()> {
        info!("Starting Orca WebSocket for {} pairs", pairs.len());

        // Connect to Orca WebSocket
        let (ws_stream, _) = connect_async(&self.ws_endpoint)
            .await
            .context("Failed to connect to Orca WebSocket")?;

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to whirlpool updates
        for pair in &pairs {
            let subscribe_msg = serde_json::json!({
                "id": 1,
                "method": "whirlpoolSubscribe",
                "params": [pair.pool_address]
            });
            
            let msg = Message::Text(subscribe_msg.to_string());
            ws_sender.send(msg).await
                .context("Failed to send Orca subscription")?;
        }

        // Process messages
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) = self.process_orca_message(&text, &pairs).await {
                        warn!("Failed to process Orca message: {}", e);
                    }
                }
                Ok(Message::Ping(payload)) => {
                    if let Err(e) = ws_sender.send(Message::Pong(payload)).await {
                        error!("Failed to send pong to Orca: {}", e);
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("Orca WebSocket closed");
                    break;
                }
                Err(e) => {
                    error!("Orca WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn fetch_price(&self, pair: &TradingPair) -> Result<PriceData> {
        // Use GeckoTerminal API as fallback for Orca
        let url = format!(
            "https://api.geckoterminal.com/api/v2/networks/solana/pools/{}",
            pair.pool_address
        );
        
        let response = self.http_client
            .get(&url)
            .timeout(Duration::from_millis(2000))
            .send()
            .await
            .context("Failed to fetch Orca pool data")?;

        let json: Value = response.json().await
            .context("Failed to parse Orca response")?;

        self.parse_orca_pool_data(&json, pair)
    }

    async fn get_supported_pairs(&self) -> Result<Vec<TradingPair>> {
        // For Orca, we'd typically get this from their API
        // This is a simplified implementation
        Ok(vec![])
    }

    async fn subscribe_updates(&self, sender: tokio::sync::mpsc::Sender<PriceUpdate>) -> Result<()> {
        let mut update_sender = self.update_sender.write().await;
        *update_sender = Some(sender);
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        // Simple ping to check if we can reach the endpoint
        match tokio::time::timeout(
            Duration::from_secs(5),
            self.http_client.get(&self.api_endpoint).send()
        ).await {
            Ok(Ok(response)) => Ok(response.status().is_success()),
            _ => Ok(false),
        }
    }
}

impl OrcaClient {
    async fn process_orca_message(&self, message: &str, pairs: &[TradingPair]) -> Result<()> {
        let json: Value = serde_json::from_str(message)
            .context("Failed to parse Orca WebSocket message")?;

        // Handle Orca-specific message format
        if let Some(method) = json["method"].as_str() {
            match method {
                "whirlpoolUpdate" => {
                    if let Some(params) = json["params"].as_object() {
                        self.handle_whirlpool_update(params, pairs).await?;
                    }
                }
                _ => {
                    debug!("Unhandled Orca message type: {}", method);
                }
            }
        }

        Ok(())
    }

    async fn handle_whirlpool_update(&self, params: &serde_json::Map<String, Value>, pairs: &[TradingPair]) -> Result<()> {
        if let Some(pool_address) = params["whirlpool"].as_str() {
            if let Some(pair) = pairs.iter().find(|p| p.pool_address == pool_address) {
                let price_data = self.parse_orca_pool_data(&Value::Object(params.clone()), pair)?;
                
                let update = PriceUpdate {
                    dex_id: DexId::Orca,
                    pair: pair.clone(),
                    price_data,
                    update_type: UpdateType::Price,
                };

                if let Some(sender) = &*self.update_sender.read().await {
                    if let Err(e) = sender.send(update).await {
                        warn!("Failed to send Orca price update: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_orca_pool_data(&self, json: &Value, pair: &TradingPair) -> Result<PriceData> {
        // Try different price field names that Orca might use
        let price = json["data"]["attributes"]["base_token_price_usd"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| json["sqrtPrice"].as_f64())
            .or_else(|| json["price"].as_f64())
            .context("Missing price in Orca data")?;

        let volume_24h = json["data"]["attributes"]["volume_usd"]["h24"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let liquidity = json["data"]["attributes"]["reserve_in_usd"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        Ok(PriceData {
            price: Decimal::try_from(price).unwrap_or_default(),
            volume_24h: Decimal::try_from(volume_24h).unwrap_or_default(),
            liquidity: Decimal::try_from(liquidity).unwrap_or_default(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            dex_id: DexId::Orca,
            pair: pair.clone(),
        })
    }
}

/// Jupiter client for price aggregation
#[derive(Debug)]
pub struct JupiterClient {
    api_url: String,
    price_api: String,
    http_client: reqwest::Client,
    update_sender: Arc<RwLock<Option<tokio::sync::mpsc::Sender<PriceUpdate>>>>,
}

impl JupiterClient {
    pub fn new(api_url: String, price_api: String) -> Self {
        Self {
            api_url,
            price_api,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            update_sender: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl DexClient for JupiterClient {
    fn dex_id(&self) -> DexId {
        DexId::Jupiter
    }

    async fn start_websocket(&self, _pairs: Vec<TradingPair>) -> Result<()> {
        // Jupiter doesn't have WebSocket API, so we'll implement polling
        info!("Starting Jupiter price polling (no WebSocket available)");
        
        // TODO: Implement periodic price polling for Jupiter
        // This would involve setting up a timer to fetch prices every few seconds
        
        Ok(())
    }

    async fn fetch_price(&self, pair: &TradingPair) -> Result<PriceData> {
        // Use Jupiter price API
        let url = format!(
            "{}/price?ids={}",
            self.price_api,
            pair.base_mint
        );

        let response = self.http_client
            .get(&url)
            .timeout(Duration::from_millis(2000))
            .send()
            .await
            .context("Failed to fetch Jupiter price")?;

        let json: Value = response.json().await
            .context("Failed to parse Jupiter response")?;

        self.parse_jupiter_price_data(&json, pair)
    }

    async fn get_supported_pairs(&self) -> Result<Vec<TradingPair>> {
        // Jupiter supports a wide range of tokens
        // This would require calling their tokens API
        Ok(vec![])
    }

    async fn subscribe_updates(&self, sender: tokio::sync::mpsc::Sender<PriceUpdate>) -> Result<()> {
        let mut update_sender = self.update_sender.write().await;
        *update_sender = Some(sender);
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.api_url);
        
        match self.http_client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await 
        {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

impl JupiterClient {
    fn parse_jupiter_price_data(&self, json: &Value, pair: &TradingPair) -> Result<PriceData> {
        let price = json["data"][&pair.base_mint]["price"].as_f64()
            .context("Missing price in Jupiter data")?;

        Ok(PriceData {
            price: Decimal::try_from(price).unwrap_or_default(),
            volume_24h: Decimal::ZERO, // Jupiter doesn't provide volume directly
            liquidity: Decimal::ZERO,  // Jupiter doesn't provide liquidity directly
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            dex_id: DexId::Jupiter,
            pair: pair.clone(),
        })
    }
}