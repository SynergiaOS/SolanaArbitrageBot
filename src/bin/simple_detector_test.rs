//! 🎯 Simple Token Detector Test - Standalone Test
//!
//! Test only our simple detector without the complex dependencies

use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

// Simple types for our test
#[derive(Debug, Clone)]
pub struct NewToken {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub price: f64,
    pub liquidity_usd: f64,
    pub market_cap_usd: f64,
    pub age_minutes: f64,
    pub volume_5m: f64,
    pub top_10_holders_percent: f64,
    pub mint_authority_disabled: bool,
    pub freeze_authority_disabled: bool,
    pub dex: String,
    pub pool_address: String,
    pub detected_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct TokenDetectorConfig {
    pub max_queue_size: usize,
    pub processed_token_ttl: Duration,
    pub min_liquidity_sol: f64,
    pub max_token_age_minutes: f64,
    pub pump_fun_api_url: String,
    pub pump_fun_poll_interval: Duration,
    pub cleanup_interval: Duration,
}

impl Default for TokenDetectorConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 50,
            processed_token_ttl: Duration::from_secs(1800),
            min_liquidity_sol: 1.0,
            max_token_age_minutes: 15.0,
            pump_fun_api_url: "https://frontend-api.pump.fun/coins".to_string(),
            pump_fun_poll_interval: Duration::from_secs(10),
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

#[derive(Clone)]
pub struct SimpleTokenDetector {
    config: TokenDetectorConfig,
    new_tokens: Arc<RwLock<VecDeque<NewToken>>>,
    processed_tokens: Arc<RwLock<HashMap<String, std::time::Instant>>>,
    is_running: Arc<RwLock<bool>>,
}

impl SimpleTokenDetector {
    pub fn new(config: TokenDetectorConfig) -> Result<Self> {
        Ok(Self {
            config,
            new_tokens: Arc::new(RwLock::new(VecDeque::new())),
            processed_tokens: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    pub async fn start_monitoring(&mut self) -> Result<()> {
        info!("🔍 Starting simple token detection...");

        *self.is_running.write().await = true;

        // Start mock token generation
        let detector = self.clone();
        tokio::spawn(async move {
            if let Err(e) = detector.generate_mock_tokens_loop().await {
                error!("Mock token generation error: {}", e);
            }
        });

        Ok(())
    }

    async fn generate_mock_tokens_loop(&self) -> Result<()> {
        let mut iteration = 0;

        loop {
            iteration += 1;

            // Generate mock tokens every few iterations
            if iteration % 2 == 0 {
                let mock_tokens = self.generate_mock_tokens().await;
                for token in mock_tokens {
                    if let Err(e) = self.add_new_token(token).await {
                        warn!("Error adding mock token: {}", e);
                    }
                }
            }

            tokio::time::sleep(self.config.pump_fun_poll_interval).await;

            if !*self.is_running.read().await {
                break;
            }
        }

        Ok(())
    }

    async fn generate_mock_tokens(&self) -> Vec<NewToken> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let token_id = now % 10000;

        let symbols = [
            "DOGE", "PEPE", "SHIB", "BONK", "WIF", "POPCAT", "MEW", "BRETT",
        ];
        let symbol = symbols[(token_id % symbols.len() as u64) as usize];

        vec![NewToken {
            mint: format!("{}{}111111111111111111111111111", symbol, token_id),
            symbol: format!("{}{}", symbol, token_id % 100),
            name: format!("{} Token {}", symbol, token_id),
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
        }]
    }

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
            return Ok(()); // Not enough liquidity
        }

        // Add to processed tokens
        {
            let mut processed = self.processed_tokens.write().await;
            processed.insert(token.mint.clone(), std::time::Instant::now());
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
            token.symbol,
            &token.mint[..8],
            token.age_minutes,
            token.liquidity_usd
        );

        Ok(())
    }

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

    pub async fn stop(&self) {
        info!("🛑 Stopping token detection...");
        *self.is_running.write().await = false;
    }
}

/// Calculate a simple score for a token (0-100)
fn calculate_simple_score(token: &NewToken) -> f64 {
    let mut score: f64 = 0.0;

    // Age bonus (newer = better for sniping)
    if token.age_minutes < 5.0 {
        score += 30.0;
    } else if token.age_minutes < 15.0 {
        score += 20.0;
    } else if token.age_minutes < 60.0 {
        score += 10.0;
    }

    // Liquidity health
    let liq_ratio = token.liquidity_usd / token.market_cap_usd.max(1.0);
    if liq_ratio > 0.2 {
        score += 25.0;
    } else if liq_ratio > 0.1 {
        score += 15.0;
    } else if liq_ratio > 0.05 {
        score += 10.0;
    }

    // Volume momentum
    if token.volume_5m > token.liquidity_usd * 0.1 {
        score += 20.0;
    } else if token.volume_5m > token.liquidity_usd * 0.05 {
        score += 10.0;
    }

    // Holder distribution
    if token.top_10_holders_percent < 40.0 {
        score += 15.0;
    } else if token.top_10_holders_percent < 60.0 {
        score += 10.0;
    } else if token.top_10_holders_percent < 80.0 {
        score += 5.0;
    }

    // Safety bonuses
    if token.mint_authority_disabled {
        score += 5.0;
    }
    if token.freeze_authority_disabled {
        score += 5.0;
    }

    // Safety penalties
    if !token.mint_authority_disabled {
        score -= 20.0;
    }
    if !token.freeze_authority_disabled {
        score -= 10.0;
    }

    // Market cap check
    if token.market_cap_usd > 1_000_000.0 {
        score -= 20.0; // Too big for sniping
    } else if token.market_cap_usd < 10_000.0 {
        score += 10.0; // Good size for sniping
    }

    score.clamp(0.0, 100.0)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("🎯 Starting Simple Token Detector Test...");

    // Create detector config
    let config = TokenDetectorConfig {
        max_queue_size: 50,
        processed_token_ttl: Duration::from_secs(1800),
        min_liquidity_sol: 1.0,
        max_token_age_minutes: 15.0,
        pump_fun_poll_interval: Duration::from_secs(5),
        cleanup_interval: Duration::from_secs(60),
        ..Default::default()
    };

    // Create detector
    let mut detector = SimpleTokenDetector::new(config)?;

    info!("✅ Simple token detector created successfully");

    // Start monitoring
    detector.start_monitoring().await?;

    info!("🔍 Token monitoring started...");

    // Test loop - check for new tokens
    let mut total_tokens_found = 0;
    let start_time = std::time::Instant::now();

    for i in 1..=15 {
        // Run for 15 iterations
        info!("📊 Check #{} - Looking for new tokens...", i);

        match detector.get_new_tokens().await {
            Ok(Some(tokens)) => {
                total_tokens_found += tokens.len();

                for token in tokens {
                    info!(
                        "🆕 Found token: {} ({}) - Age: {:.1}m, Liquidity: ${:.0}, DEX: {}",
                        token.symbol,
                        &token.mint[..8],
                        token.age_minutes,
                        token.liquidity_usd,
                        token.dex
                    );

                    // Show token details
                    info!(
                        "   📈 Market Cap: ${:.0}, Price: ${:.6}",
                        token.market_cap_usd, token.price
                    );
                    info!(
                        "   🔒 Mint Auth: {}, Freeze Auth: {}",
                        if token.mint_authority_disabled {
                            "✅ Disabled"
                        } else {
                            "❌ Enabled"
                        },
                        if token.freeze_authority_disabled {
                            "✅ Disabled"
                        } else {
                            "❌ Enabled"
                        }
                    );
                    info!("   👥 Top 10 holders: {:.1}%", token.top_10_holders_percent);

                    // Calculate a simple score
                    let score = calculate_simple_score(&token);
                    info!("   ⭐ Score: {:.1}/100", score);

                    if score >= 60.0 {
                        info!("   🎯 POTENTIAL SNIPE TARGET!");
                    }

                    println!(); // Empty line for readability
                }
            }
            Ok(None) => {
                info!("   No new tokens found");
            }
            Err(e) => {
                error!("   Error getting tokens: {}", e);
            }
        }

        // Show statistics
        let elapsed = start_time.elapsed();
        info!(
            "📊 Stats: {} tokens found in {:.1}s",
            total_tokens_found,
            elapsed.as_secs_f64()
        );

        // Wait before next check
        tokio::time::sleep(Duration::from_secs(4)).await;
    }

    // Stop detector
    detector.stop().await;

    info!("🎉 Test completed!");
    info!("📊 Final stats: {} total tokens found", total_tokens_found);

    if total_tokens_found > 0 {
        info!("✅ Simple token detector is working!");
    } else {
        warn!("⚠️ No tokens found - check the mock token generation");
    }

    Ok(())
}
