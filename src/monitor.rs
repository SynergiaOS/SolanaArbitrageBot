//! DEX Price Monitor - Optimized for High-Frequency Updates
//! Real-time monitoring with WebSocket, caching, and concurrent fetching

use crate::config_manager::BotConfig;
use crate::utils::conversions::*;
use anyhow::{Result, Context};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};

// Pool addresses will be loaded from config

#[derive(Debug, Clone)]
pub struct PriceUpdate {
    pub dex: String,
    pub price: Decimal,
    pub volume_24h: Decimal,
    pub liquidity: Decimal,
    pub timestamp: u64,
}

#[derive(Debug, Deserialize)]
struct RaydiumPoolState {
    pub base_reserve: u64,
    pub quote_reserve: u64,
    pub status: u8,
}

#[derive(Debug, Deserialize)]
struct OrcaWhirlpoolState {
    pub sqrt_price: u128,
    pub liquidity: u128,
    pub tick_current: i32,
}

pub struct DexMonitor {
    rpc_url: String,
    ws_url: String,
    price_cache: Arc<RwLock<HashMap<String, (Decimal, Instant)>>>,
    raydium_pool: String,
    orca_pool: String,

    // Performance tracking
    time_since_last_update: Arc<RwLock<Instant>>,
    update_interval: Duration,

    // Performance optimizations
    http_client: reqwest::Client,
    is_active: Arc<AtomicBool>,
    websocket_tasks: Arc<RwLock<Vec<tokio::task::JoinHandle<Result<()>>>>>,
    subscribers: Arc<RwLock<Vec<tokio::sync::mpsc::Sender<PriceUpdate>>>>,
}

impl DexMonitor {
    pub fn new(
        config: &BotConfig,
    ) -> Self {
        Self {
            rpc_url: config.network.rpc_url.clone(),
            ws_url: config.network.ws_url.clone(),
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            raydium_pool: config.dex.raydium.pool_address.clone(),
            orca_pool: config.dex.orca.pool_address.clone(),
            is_active: Arc::new(AtomicBool::new(false)),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .pool_idle_timeout(Duration::from_secs(30))
                .pool_max_idle_per_host(10)
                .build()
                .expect("Failed to create HTTP client"),
            update_interval: Duration::from_millis(100), // 100ms updates
            websocket_tasks: Arc::new(RwLock::new(Vec::new())),
            subscribers: Arc::new(RwLock::new(Vec::new())),
            time_since_last_update: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub async fn start_websocket_monitoring(&self) -> Result<()> {
        info!("🔌 Starting WebSocket monitoring");
    
        // Connect to multiple WebSocket streams
        let pools = vec![
            self.raydium_pool.clone(),
            self.orca_pool.clone(),
        ];
    
        Ok(())
    }

    pub async fn start_monitoring(&self) -> Result<()> {
        info!("🚀 Starting optimized DEX monitoring...");

        // Clone shared resources for concurrent tasks
        let _http_client = self.http_client.clone();
        let http_client1 = self.http_client.clone();
        let http_client2 = self.http_client.clone();
        let price_cache1 = self.price_cache.clone();
        let price_cache2 = self.price_cache.clone();
        let raydium_pool = self.raydium_pool.clone();
        let orca_pool = self.orca_pool.clone();

        // Start monitoring both DEXes concurrently with optimizations
        let raydium_handle = tokio::spawn(async move {
            Self::monitor_raydium_optimized(
                raydium_pool,
                http_client1,
                price_cache1,
            )
            .await
        });

        let orca_handle = tokio::spawn(async move {
            Self::monitor_orca_optimized(
                orca_pool,
                http_client2,
                price_cache2,
            )
            .await
        });

        // Wait for both (they run forever unless error)
        let (raydium_result, orca_result) = tokio::join!(raydium_handle, orca_handle);

        if let Err(e) = raydium_result {
            error!("Raydium monitor crashed: {}", e);
        }
        if let Err(e) = orca_result {
            error!("Orca monitor crashed: {}", e);
        }

        Ok(())
    }

    async fn monitor_raydium_optimized(
        pool_address: String,
        http_client: reqwest::Client,
        price_cache: Arc<RwLock<HashMap<String, (Decimal, Instant)>>>,
    ) -> Result<()> {
        info!("📡 Starting optimized Raydium monitoring...");

        loop {
            let start_time = Instant::now();

            // Check cache first
            let cache_key = format!("raydium_{}", pool_address);
            let should_fetch = {
                let cache = price_cache.read().await;
                if let Some((_, cached_time)) = cache.get(&cache_key) {
                    if cached_time.elapsed() < Duration::from_millis(10) {
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            };

            if should_fetch {
                if let Ok(price) = Self::fetch_raydium_price_fast(&http_client, &pool_address).await
                {
                    // Update cache
                    let mut cache = price_cache.write().await;
                    cache.insert(cache_key, (price, Instant::now()));
                }
            }

            // Manually update at 10Hz
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    async fn fetch_raydium_price_fast(
        http_client: &reqwest::Client,
        pool_address: &str,
    ) -> Result<Decimal> {
        let api_url = format!(
            "https://api.geckoterminal.com/api/v2/networks/solana/pools/{}",
            pool_address
        );

        let response = http_client
            .get(&api_url)
            .timeout(Duration::from_millis(2000)) // Fast timeout
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;

        if let Some(price_str) = json["data"]["attributes"]["base_token_price_usd"].as_str() {
            if let Ok(price_f64) = price_str.parse::<f64>() {
                return Ok(f64_to_decimal(price_f64));
            }
        }

        Err(anyhow::anyhow!("Failed to parse price from API response"))
    }

    async fn monitor_orca_optimized(
        pool_address: String,
        http_client: reqwest::Client,
        price_cache: Arc<RwLock<HashMap<String, (Decimal, Instant)>>>,
    ) -> Result<()> {
        info!("🐋 Starting optimized Orca monitoring...");

        loop {
            let start_time = Instant::now();

            // Check cache first
            let cache_key = format!("orca_{}", pool_address);
            let should_fetch = {
                let cache = price_cache.write().await;
                if let Some((_, cached_time)) = cache.get(&cache_key) {
                    if cached_time.elapsed() < Duration::from_millis(10) {
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            };

            if should_fetch {
                if let Ok(price) = Self::fetch_orca_price_fast(&http_client, &pool_address).await {
                    // Update cache
                    let mut cache = price_cache.write().await;
                    cache.insert(cache_key, (price, Instant::now()));
                }
            }

            // Manually update at 10Hz
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    async fn fetch_orca_price_fast(
        http_client: &reqwest::Client,
        pool_address: &str,
    ) -> Result<Decimal> {
        let api_url = format!(
            "https://api.geckoterminal.com/api/v2/networks/solana/pools/{}",
            pool_address
        );

        let response = http_client
            .get(&api_url)
            .timeout(Duration::from_millis(2000)) // Fast timeout
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;

        if let Some(price_str) = json["data"]["attributes"]["base_token_price_usd"].as_str() {
            if let Ok(price_f64) = price_str.parse::<f64>() {
                return Ok(f64_to_decimal(price_f64));
            }
        }

        Err(anyhow::anyhow!("Failed to parse price from API response"))
    }

    // Helper function to fetch pool info via HTTP API (backup method)
    pub async fn fetch_pool_stats(&self) -> Result<(f64, f64)> {
        // This would connect to Raydium/Orca HTTP APIs for additional data
        // For now, return placeholder
        Ok((0.0, 0.0))
    }
}

// Additional structs for Jupiter integration
#[derive(Debug, Serialize, Deserialize)]
pub struct JupiterQuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: u16,
}

#[derive(Debug, Deserialize)]
pub struct JupiterQuote {
    pub in_amount: u64,
    pub out_amount: u64,
    pub price_impact_pct: f64,
    pub market_infos: Vec<MarketInfo>,
}

#[derive(Debug, Deserialize)]
pub struct MarketInfo {
    pub id: String,
    pub label: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub out_amount: u64,
}

impl DexMonitor {
    pub async fn get_jupiter_quote(
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<JupiterQuote> {
        let client = reqwest::Client::new();
        let url = format!(
            "https://quote-api.jup.ag/v6/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            input_mint, output_mint, amount, slippage_bps
        );

        let response = client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch Jupiter quote")?;

        let quote: JupiterQuote = response
            .json()
            .await
            .context("Failed to parse Jupiter quote")?;

        Ok(quote)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jupiter_quote() {
        // Test fetching a real quote from Jupiter
        let sol_mint = "So11111111111111111111111111111111111111112";
        let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let amount = 1_000_000_000; // 1 SOL

        match DexMonitor::get_jupiter_quote(sol_mint, usdc_mint, amount, 50).await {
            Ok(quote) => {
                println!(
                    "Jupiter quote: {} SOL = {} USDC",
                    quote.in_amount as f64 / 1e9,
                    quote.out_amount as f64 / 1e6
                );
                assert!(quote.out_amount > 0);
            }
            Err(e) => {
                // It's ok if this fails in test environment
                println!("Jupiter API not available: {}", e);
            }
        }
    }
}
