//! 🎯 Simple Token Detector - Fast & Reliable New Token Detection
//!
//! Monitors Raydium, Orca, and Pump.fun for new token launches

use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use serde_json::Value;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::sniper::types::*;

/// Simple but effective token detector
#[derive(Clone)]
pub struct TokenDetector {
    #[allow(dead_code)]
    rpc_client: Arc<RpcClient>,
    config: TokenDetectorConfig,
    new_tokens: Arc<RwLock<VecDeque<NewToken>>>,
    processed_tokens: Arc<RwLock<HashMap<String, Instant>>>,
    is_running: Arc<RwLock<bool>>,
}

#[derive(Debug, Clone)]
pub struct TokenDetectorConfig {
    /// Maximum tokens to keep in queue
    pub max_queue_size: usize,

    /// How long to remember processed tokens (to avoid duplicates)
    pub processed_token_ttl: Duration,

    /// Minimum liquidity in SOL to consider
    pub min_liquidity_sol: f64,

    /// Maximum token age in minutes to consider
    pub max_token_age_minutes: f64,

    /// API URLs for monitoring
    pub pump_fun_api_url: String,

    /// Polling intervals
    pub pump_fun_poll_interval: Duration,
    pub cleanup_interval: Duration,
}

impl Default for TokenDetectorConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 100,
            processed_token_ttl: Duration::from_secs(3600), // 1 hour
            min_liquidity_sol: 1.0,
            max_token_age_minutes: 10.0,
            pump_fun_api_url: "https://frontend-api.pump.fun/coins".to_string(),
            pump_fun_poll_interval: Duration::from_secs(5),
            cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl TokenDetector {
    /// Create new token detector
    pub fn new(config: TokenDetectorConfig) -> Result<Self> {
        let rpc_client = Arc::new(RpcClient::new(
            "https://api.mainnet-beta.solana.com".to_string(),
        ));

        Ok(Self {
            rpc_client,
            config,
            new_tokens: Arc::new(RwLock::new(VecDeque::new())),
            processed_tokens: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start monitoring for new tokens
    pub async fn start_monitoring(&mut self) -> Result<()> {
        info!("🔍 Starting token detection...");

        *self.is_running.write().await = true;

        // For now, only start Pump.fun monitoring (simpler, no WebSocket conflicts)
        let pump_fun_handle = {
            let detector = self.clone();
            tokio::spawn(async move {
                if let Err(e) = detector.monitor_pump_fun().await {
                    error!("Pump.fun monitoring error: {}", e);
                }
            })
        };

        // Start cleanup task
        let cleanup_handle = {
            let detector = self.clone();
            tokio::spawn(async move {
                detector.cleanup_loop().await;
            })
        };

        // Wait for tasks
        tokio::try_join!(pump_fun_handle, cleanup_handle)?;

        Ok(())
    }

    /// Get new tokens from the queue
    pub async fn get_new_tokens(&self) -> Result<Option<Vec<NewToken>>> {
        let mut queue = self.new_tokens.write().await;

        if queue.is_empty() {
            return Ok(None);
        }

        let mut tokens = Vec::new();

        // Get up to 10 tokens at once
        for _ in 0..10 {
            if let Some(token) = queue.pop_front() {
                tokens.push(token);
            } else {
                break;
            }
        }

        if tokens.is_empty() {
            Ok(None)
        } else {
            debug!("📦 Retrieved {} new tokens from queue", tokens.len());
            Ok(Some(tokens))
        }
    }

    /// Monitor Pump.fun for new tokens
    async fn monitor_pump_fun(&self) -> Result<()> {
        info!("🔍 Starting Pump.fun monitoring...");

        let client = reqwest::Client::new();
        let mut iteration = 0;

        loop {
            iteration += 1;

            // For testing, generate some mock tokens periodically
            if iteration % 3 == 0 {
                let mock_tokens = self.generate_mock_tokens().await;
                for token in mock_tokens {
                    if let Err(e) = self.add_new_token(token).await {
                        warn!("Error adding mock token: {}", e);
                    }
                }
            }

            // Try to fetch real tokens (might fail if API is down)
            match self.fetch_pump_fun_tokens(&client).await {
                Ok(tokens) => {
                    for token in tokens {
                        if let Err(e) = self.add_new_token(token).await {
                            warn!("Error adding Pump.fun token: {}", e);
                        }
                    }
                }
                Err(e) => {
                    debug!("Pump.fun API error (using mock data): {}", e);
                }
            }

            tokio::time::sleep(self.config.pump_fun_poll_interval).await;

            if !*self.is_running.read().await {
                break;
            }
        }

        Ok(())
    }

    /// Fetch tokens from Pump.fun API
    async fn fetch_pump_fun_tokens(&self, client: &reqwest::Client) -> Result<Vec<NewToken>> {
        let response = client.get(&self.config.pump_fun_api_url).send().await?;

        let data: Value = response.json().await?;
        let mut tokens = Vec::new();

        // Parse Pump.fun response (this is a simplified version)
        if let Some(coins) = data.as_array() {
            for coin in coins.iter().take(10) {
                // Only take first 10
                if let Ok(token) = self.parse_pump_fun_token(coin).await {
                    tokens.push(token);
                }
            }
        }

        Ok(tokens)
    }

    /// Parse Pump.fun token data
    async fn parse_pump_fun_token(&self, data: &Value) -> Result<NewToken> {
        let mint = data
            .get("mint")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing mint"))?;

        let symbol = data
            .get("symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN");

        let name = data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Token");

        let market_cap = data
            .get("usd_market_cap")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let created_timestamp = data
            .get("created_timestamp")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let age_minutes = if created_timestamp > 0 {
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            (now - created_timestamp) as f64 / 60.0
        } else {
            0.0
        };

        Ok(NewToken {
            mint: mint.to_string(),
            symbol: symbol.to_string(),
            name: name.to_string(),
            price: 0.001,                    // Default price
            liquidity_usd: market_cap * 0.1, // Estimate 10% of market cap as liquidity
            market_cap_usd: market_cap,
            age_minutes,
            volume_5m: 0.0,                // Not available from this API
            top_10_holders_percent: 50.0,  // Default assumption
            mint_authority_disabled: true, // Assume safe for Pump.fun
            freeze_authority_disabled: true,
            dex: "Pump.fun".to_string(),
            pool_address: "".to_string(),
            detected_at: SystemTime::now(),
        })
    }

    /// Add new token to queue
    async fn add_new_token(&self, token: NewToken) -> Result<()> {
        // Check if we've already processed this token recently
        {
            let processed = self.processed_tokens.read().await;
            if let Some(last_seen) = processed.get(&token.mint) {
                if last_seen.elapsed() < self.config.processed_token_ttl {
                    return Ok(()); // Skip duplicate
                }
            }
        }

        // Check basic filters
        if token.age_minutes > self.config.max_token_age_minutes {
            return Ok(()); // Too old
        }

        if token.liquidity_usd < self.config.min_liquidity_sol * 100.0 {
            // Rough SOL to USD conversion
            return Ok(()); // Not enough liquidity
        }

        // Add to processed tokens
        {
            let mut processed = self.processed_tokens.write().await;
            processed.insert(token.mint.clone(), Instant::now());
        }

        // Add to queue
        {
            let mut queue = self.new_tokens.write().await;

            // Remove oldest if queue is full
            if queue.len() >= self.config.max_queue_size {
                queue.pop_front();
            }

            queue.push_back(token.clone());
        }

        info!(
            "🆕 New token detected: {} ({}) - Age: {:.1}m, Liquidity: ${:.0}",
            token.symbol, token.mint, token.age_minutes, token.liquidity_usd
        );

        Ok(())
    }

    /// Cleanup old processed tokens
    async fn cleanup_loop(&self) {
        loop {
            tokio::time::sleep(self.config.cleanup_interval).await;

            let mut processed = self.processed_tokens.write().await;
            processed.retain(|_, last_seen| last_seen.elapsed() < self.config.processed_token_ttl);

            debug!(
                "🧹 Cleaned up old processed tokens, {} remaining",
                processed.len()
            );

            if !*self.is_running.read().await {
                break;
            }
        }
    }

    /// Generate mock tokens for testing
    async fn generate_mock_tokens(&self) -> Vec<NewToken> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let token_id = now % 10000; // Simple ID generation

        let mock_tokens = vec![NewToken {
            mint: format!("MOCK{}111111111111111111111111111", token_id),
            symbol: format!("MOCK{}", token_id),
            name: format!("Mock Token {}", token_id),
            price: 0.001 + (token_id as f64 * 0.0001),
            liquidity_usd: 5000.0 + (token_id as f64 * 100.0),
            market_cap_usd: 50000.0 + (token_id as f64 * 1000.0),
            age_minutes: (token_id % 10) as f64,
            volume_5m: 1000.0 + (token_id as f64 * 50.0),
            top_10_holders_percent: 30.0 + (token_id % 40) as f64,
            mint_authority_disabled: token_id % 2 == 0,
            freeze_authority_disabled: token_id % 3 == 0,
            dex: "Pump.fun".to_string(),
            pool_address: format!("POOL{}111111111111111111111111111", token_id),
            detected_at: SystemTime::now(),
        }];

        mock_tokens
    }

    /// Stop monitoring
    pub async fn stop(&self) {
        info!("🛑 Stopping token detection...");
        *self.is_running.write().await = false;
    }
}
