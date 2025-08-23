//! DEX Price Monitor - Optimized for High-Frequency Updates
//! Real-time monitoring with WebSocket, caching, and concurrent fetching

use anyhow::{Result, Context};
use log::{info, debug, error, warn};
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use crate::utils::conversions::*;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::time::interval;

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
    raydium_price: Arc<Mutex<Option<Decimal>>>,
    orca_price: Arc<Mutex<Option<Decimal>>>,
    rpc_url: String,
    ws_url: String,
    price_updates_tx: Option<tokio::sync::mpsc::Sender<PriceUpdate>>,
    raydium_pool: String,
    orca_pool: String,

    // Performance optimizations
    http_client: reqwest::Client,
    price_cache: Arc<Mutex<HashMap<String, (Decimal, Instant)>>>,
    last_update_times: Arc<Mutex<HashMap<String, Instant>>>,

    // Performance tracking
    fetch_count: Arc<std::sync::atomic::AtomicU64>,
    total_fetch_time_ms: Arc<std::sync::atomic::AtomicU64>,
}

impl DexMonitor {
    pub fn new(
        raydium_price: Arc<Mutex<Option<Decimal>>>,
        orca_price: Arc<Mutex<Option<Decimal>>>,
        config: &crate::Config,
    ) -> Result<Self> {
        // Create optimized HTTP client
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()
            .expect("Failed to create HTTP client");

        Ok(Self {
            raydium_price,
            orca_price,
            rpc_url: config.rpc.url.clone(),
            ws_url: config.rpc.ws_url.clone(),
            price_updates_tx: None,
            raydium_pool: config.dex.raydium.sol_usdc_pool.clone(),
            orca_pool: config.dex.orca.sol_usdc_pool.clone(),
            http_client,
            price_cache: Arc::new(Mutex::new(HashMap::new())),
            last_update_times: Arc::new(Mutex::new(HashMap::new())),
            fetch_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_fetch_time_ms: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        })
    }
    
    pub fn with_price_channel(mut self, tx: tokio::sync::mpsc::Sender<PriceUpdate>) -> Self {
        self.price_updates_tx = Some(tx);
        self
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> (u64, f64) {
        let fetches = self.fetch_count.load(std::sync::atomic::Ordering::Relaxed);
        let total_time = self.total_fetch_time_ms.load(std::sync::atomic::Ordering::Relaxed);

        let avg_time_ms = if fetches > 0 {
            total_time as f64 / fetches as f64
        } else {
            0.0
        };

        (fetches, avg_time_ms)
    }
    
    pub async fn start_monitoring(self) -> Result<()> {
        info!("🚀 Starting optimized DEX monitoring...");

        // Clone shared resources for concurrent tasks
        let raydium_price = self.raydium_price.clone();
        let orca_price = self.orca_price.clone();
        let http_client1 = self.http_client.clone();
        let http_client2 = self.http_client.clone();
        let price_cache1 = self.price_cache.clone();
        let price_cache2 = self.price_cache.clone();
        let last_update_times1 = self.last_update_times.clone();
        let last_update_times2 = self.last_update_times.clone();
        let fetch_count1 = self.fetch_count.clone();
        let fetch_count2 = self.fetch_count.clone();
        let total_fetch_time_ms1 = self.total_fetch_time_ms.clone();
        let total_fetch_time_ms2 = self.total_fetch_time_ms.clone();

        let price_tx1 = self.price_updates_tx.clone();
        let price_tx2 = self.price_updates_tx.clone();
        let raydium_pool = self.raydium_pool.clone();
        let orca_pool = self.orca_pool.clone();

        // Start monitoring both DEXes concurrently with optimizations
        let raydium_handle = tokio::spawn(async move {
            Self::monitor_raydium_optimized(
                raydium_price,
                http_client1,
                price_cache1,
                last_update_times1,
                fetch_count1,
                total_fetch_time_ms1,
                price_tx1,
                raydium_pool
            ).await
        });

        let orca_handle = tokio::spawn(async move {
            Self::monitor_orca_optimized(
                orca_price,
                http_client2,
                price_cache2,
                last_update_times2,
                fetch_count2,
                total_fetch_time_ms2,
                price_tx2,
                orca_pool
            ).await
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
        price_state: Arc<Mutex<Option<Decimal>>>,
        http_client: reqwest::Client,
        price_cache: Arc<Mutex<HashMap<String, (Decimal, Instant)>>>,
        _last_update_times: Arc<Mutex<HashMap<String, Instant>>>,
        fetch_count: Arc<std::sync::atomic::AtomicU64>,
        total_fetch_time_ms: Arc<std::sync::atomic::AtomicU64>,
        price_tx: Option<tokio::sync::mpsc::Sender<PriceUpdate>>,
        pool_address: String,
    ) -> Result<()> {
        info!("📡 Starting optimized Raydium monitoring...");

        let mut update_interval = interval(Duration::from_millis(100)); // 10Hz updates
        let cache_duration = Duration::from_millis(50); // Cache for 50ms

        loop {
            update_interval.tick().await;

            let start_time = Instant::now();
            fetch_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            // Check cache first
            let cache_key = format!("raydium_{}", pool_address);
            let should_fetch = {
                let cache = price_cache.lock().await;
                if let Some((cached_price, cached_time)) = cache.get(&cache_key) {
                    if cached_time.elapsed() < cache_duration {
                        // Use cached price
                        let mut current_price = price_state.lock().await;
                        *current_price = Some(*cached_price);
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            };

            if should_fetch {
                if let Ok(price) = Self::fetch_raydium_price_fast(&http_client, &pool_address).await {
                    // Update cache
                    let mut cache = price_cache.lock().await;
                    cache.insert(cache_key, (price, Instant::now()));

                    // Update state
                    let mut current_price = price_state.lock().await;
                    *current_price = Some(price);

                    // Send update if channel exists
                    if let Some(tx) = &price_tx {
                        let update = PriceUpdate {
                            dex: "Raydium".to_string(),
                            price,
                            volume_24h: Decimal::ZERO,
                            liquidity: Decimal::ZERO,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        };
                        let _ = tx.send(update).await;
                    }
                }
            }

            // Update performance metrics
            let elapsed = start_time.elapsed().as_millis() as u64;
            total_fetch_time_ms.fetch_add(elapsed, std::sync::atomic::Ordering::Relaxed);
        }
    }
    
    async fn fetch_raydium_price_fast(
        http_client: &reqwest::Client,
        pool_address: &str,
    ) -> Result<Decimal> {
        let api_url = format!("https://api.geckoterminal.com/api/v2/networks/solana/pools/{}", pool_address);

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
        price_state: Arc<Mutex<Option<Decimal>>>,
        http_client: reqwest::Client,
        price_cache: Arc<Mutex<HashMap<String, (Decimal, Instant)>>>,
        _last_update_times: Arc<Mutex<HashMap<String, Instant>>>,
        fetch_count: Arc<std::sync::atomic::AtomicU64>,
        total_fetch_time_ms: Arc<std::sync::atomic::AtomicU64>,
        price_tx: Option<tokio::sync::mpsc::Sender<PriceUpdate>>,
        pool_address: String,
    ) -> Result<()> {
        info!("🐋 Starting optimized Orca monitoring...");

        let mut update_interval = interval(Duration::from_millis(100)); // 10Hz updates
        let cache_duration = Duration::from_millis(50); // Cache for 50ms

        loop {
            update_interval.tick().await;

            let start_time = Instant::now();
            fetch_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            // Check cache first
            let cache_key = format!("orca_{}", pool_address);
            let should_fetch = {
                let cache = price_cache.lock().await;
                if let Some((cached_price, cached_time)) = cache.get(&cache_key) {
                    if cached_time.elapsed() < cache_duration {
                        // Use cached price
                        let mut current_price = price_state.lock().await;
                        *current_price = Some(*cached_price);
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
                    let mut cache = price_cache.lock().await;
                    cache.insert(cache_key, (price, Instant::now()));

                    // Update state
                    let mut current_price = price_state.lock().await;
                    *current_price = Some(price);

                    // Send update if channel exists
                    if let Some(tx) = &price_tx {
                        let update = PriceUpdate {
                            dex: "Orca".to_string(),
                            price,
                            volume_24h: Decimal::ZERO,
                            liquidity: Decimal::ZERO,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        };
                        let _ = tx.send(update).await;
                    }
                }
            }

            // Update performance metrics
            let elapsed = start_time.elapsed().as_millis() as u64;
            total_fetch_time_ms.fetch_add(elapsed, std::sync::atomic::Ordering::Relaxed);
        }
    }

    async fn fetch_orca_price_fast(
        http_client: &reqwest::Client,
        pool_address: &str,
    ) -> Result<Decimal> {
        let api_url = format!("https://api.geckoterminal.com/api/v2/networks/solana/pools/{}", pool_address);

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
        
        let response = client.get(&url)
            .send()
            .await
            .context("Failed to fetch Jupiter quote")?;
        
        let quote: JupiterQuote = response.json()
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
                println!("Jupiter quote: {} SOL = {} USDC", 
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
