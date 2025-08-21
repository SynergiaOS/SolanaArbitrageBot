//! Token Monitor - Real-time detection of new tokens
//! Uses WebSocket subscriptions to monitor Raydium pool creations

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use log::{info, debug, error, warn};
use reqwest::Client;
use tokio::sync::mpsc;
use regex::Regex;
use std::sync::Arc;
use std::str::FromStr;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Debug, Clone)]
pub struct NewToken {
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub name: String,
    pub symbol: String,
    pub initial_liquidity: u64,
    pub pool_address: Pubkey,
    pub timestamp: u64,
    pub market_cap_estimate: f64,
}

pub struct TokenMonitor {
    rpc_client: Arc<RpcClient>,
    ws_url: String,
}

impl TokenMonitor {
    pub fn new(rpc_client: Arc<RpcClient>, ws_url: String) -> Self {
        Self { 
            rpc_client,
            ws_url,
        }
    }
    
    pub async fn subscribe_to_new_tokens(&self) -> Result<mpsc::Receiver<NewToken>> {
        let (tx, rx) = mpsc::channel(100);
        
        info!("🔍 Starting WebSocket connection to monitor new tokens...");
        
        let ws_url = self.ws_url.clone();
        let tx_clone = tx.clone();
        
        tokio::spawn(async move {
            if let Err(e) = Self::monitor_raydium_pools(ws_url, tx_clone).await {
                error!("❌ WebSocket monitoring failed: {}", e);
            }
        });
        
        Ok(rx)
    }
    
    async fn monitor_raydium_pools(ws_url: String, tx: mpsc::Sender<NewToken>) -> Result<()> {
        let (ws_stream, _) = connect_async(&ws_url).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        
        // Subscribe to Raydium program logs
        let raydium_program = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
        let subscription_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "logsSubscribe",
            "params": [
                {
                    "mentions": [raydium_program]
                },
                {
                    "commitment": "confirmed"
                }
            ]
        });
        
        ws_sender.send(Message::Text(subscription_request.to_string())).await?;
        info!("📡 Subscribed to Raydium program logs");
        
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Some(new_token) = Self::parse_log_message(&text).await {
                        info!("🆕 New token detected: {} ({})", new_token.symbol, new_token.mint);
                        
                        if tx.send(new_token).await.is_err() {
                            warn!("Channel closed, stopping monitor");
                            break;
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    warn!("WebSocket connection closed");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    async fn parse_log_message(message: &str) -> Option<NewToken> {
        let parsed: serde_json::Value = serde_json::from_str(message).ok()?;
        
        // Check if this is a logs notification
        if parsed.get("method")?.as_str()? != "logsNotification" {
            return None;
        }
        
        let params = parsed.get("params")?;
        let result = params.get("result")?;
        let logs = result.get("value")?.get("logs")?.as_array()?;
        
        // Look for pool initialization logs
        for log in logs {
            let log_str = log.as_str()?;
            
            if log_str.contains("initialize") && log_str.contains("mint") {
                if let Some(token) = Self::extract_token_from_log(log_str).await {
                    return Some(token);
                }
            }
        }
        
        None
    }
    
    async fn extract_token_from_log(log: &str) -> Option<NewToken> {
        // Parse mint address from log
        let mint_pattern = Regex::new(r"mint:\s*([A-Za-z0-9]{32,44})").ok()?;
        let mint_str = mint_pattern.captures(log)?.get(1)?.as_str();
        let mint = Pubkey::from_str(mint_str).ok()?;
        
        // For now, return basic token info
        // In production, you'd fetch metadata from chain
        Some(NewToken {
            mint,
            creator: Pubkey::default(), // Would parse from transaction
            name: "Unknown".to_string(),
            symbol: "UNK".to_string(),
            initial_liquidity: 0,
            pool_address: Pubkey::default(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            market_cap_estimate: 0.0,
        })
    }
    
    pub async fn get_token_metadata(&self, mint: &Pubkey) -> Result<TokenMetadata> {
        // Try Jupiter token metadata API
        let url = format!("https://tokens.jup.ag/token/{}", mint);
        let client = Client::new();
        let resp = client.get(&url).send().await?;
        if resp.status().is_success() {
            let v: serde_json::Value = resp.json().await?;
            let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("Unknown").to_string();
            let symbol = v.get("symbol").and_then(|x| x.as_str()).unwrap_or("UNK").to_string();
            let decimals = v.get("decimals").and_then(|x| x.as_u64()).unwrap_or(9) as u8;
            let supply = v.get("supply").and_then(|x| x.as_u64()).unwrap_or(0);
            return Ok(TokenMetadata { name, symbol, decimals, supply });
        }

        // Fallback defaults
        Ok(TokenMetadata { name: "Unknown".to_string(), symbol: "UNK".to_string(), decimals: 9, supply: 0 })
    }
    
    pub async fn calculate_liquidity(&self, pool_address: &Pubkey) -> Result<f64> {
        // Calculate pool liquidity in SOL
        // This would analyze pool accounts
        Ok(0.0)
    }
    
    pub async fn estimate_market_cap(&self, mint: &Pubkey, liquidity_sol: f64) -> Result<f64> {
        // Estimate market cap based on liquidity and token supply
        // This is a rough calculation
        Ok(liquidity_sol * 2.0 * 187.0) // Assume 2x liquidity = MC, SOL at $187
    }
}

#[derive(Debug, Clone)]
pub struct TokenMetadata {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub supply: u64,
}

#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub address: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_reserve: u64,
    pub quote_reserve: u64,
}
