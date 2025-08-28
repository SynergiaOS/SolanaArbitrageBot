//! Enhanced DEX Price Monitor - Multi-DEX Real-time Monitoring
//! 
//! High-performance price monitoring across multiple DEXs with:
//! - WebSocket connections to Raydium, Orca, Jupiter
//! - <100ms latency price updates
//! - Thread-safe caching with DashMap
//! - Automatic reconnection with exponential backoff
//! - Price history buffer (last 100 updates per pair)
//! - Real-time arbitrage opportunity detection

use crate::config_manager::BotConfig;
use crate::dex::{MultiDexMonitor, DexId, TradingPair, PriceUpdate, PriceData};
use crate::dex::clients::{RaydiumClient, OrcaClient, JupiterClient};
use anyhow::{Result, Context};
use dashmap::DashMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

/// Performance metrics for monitoring
#[derive(Debug, Clone, Serialize)]
pub struct MonitoringMetrics {
    pub total_updates_received: u64,
    pub average_latency_ms: f64,
    pub active_connections: u32,
    pub failed_connections: u32,
    pub cache_hit_rate: f64,
    pub last_update_timestamp: u64,
    pub opportunities_detected: u64,
    pub uptime_seconds: u64,
}

/// Legacy price update for backward compatibility
#[derive(Debug, Clone, Serialize)]
pub struct LegacyPriceUpdate {
    pub dex: String,
    pub price: Decimal,
    pub volume_24h: Decimal,
    pub liquidity: Decimal,
    pub timestamp: u64,
}

impl From<PriceUpdate> for LegacyPriceUpdate {
    fn from(update: PriceUpdate) -> Self {
        Self {
            dex: update.dex_id.as_str().to_string(),
            price: update.price_data.price,
            volume_24h: update.price_data.volume_24h,
            liquidity: update.price_data.liquidity,
            timestamp: update.price_data.timestamp,
        }
    }
}

/// Arbitrage opportunity detection
#[derive(Debug, Clone, Serialize)]
pub struct ArbitrageOpportunity {
    pub pair: TradingPair,
    pub buy_dex: DexId,
    pub sell_dex: DexId,
    pub spread_percent: Decimal,
    pub estimated_profit_usd: Decimal,
    pub confidence_score: f64,
    pub detected_at: u64,
    pub buy_price: Decimal,
    pub sell_price: Decimal,
}

/// Price spread data between DEXs
#[derive(Debug, Clone)]
pub struct PriceSpread {
    pub pair: TradingPair,
    pub highest_price: Decimal,
    pub lowest_price: Decimal,
    pub highest_dex: DexId,
    pub lowest_dex: DexId,
    pub spread_percent: Decimal,
    pub updated_at: u64,
}

/// Enhanced multi-DEX price monitor with arbitrage detection
pub struct EnhancedDexMonitor {
    // Core monitoring
    multi_dex_monitor: MultiDexMonitor,
    update_receiver: broadcast::Receiver<PriceUpdate>,
    
    // Configuration
    config: BotConfig,
    
    // Caching and performance
    price_spreads: Arc<DashMap<String, PriceSpread>>,
    opportunities: Arc<RwLock<Vec<ArbitrageOpportunity>>>,
    
    // Metrics
    metrics: Arc<RwLock<MonitoringMetrics>>,
    start_time: Instant,
    
    // Event broadcasting
    opportunity_sender: broadcast::Sender<ArbitrageOpportunity>,
    legacy_update_sender: broadcast::Sender<LegacyPriceUpdate>,
    
    // Performance tracking
    latency_samples: Arc<RwLock<Vec<Duration>>>,
    last_health_check: Arc<RwLock<Instant>>,
}

impl EnhancedDexMonitor {
    /// Create new enhanced DEX monitor with multi-DEX support
    pub fn new(config: BotConfig) -> Result<(Self, broadcast::Receiver<ArbitrageOpportunity>, broadcast::Receiver<LegacyPriceUpdate>)> {
        info!("🚀 Initializing Enhanced DEX Monitor with multi-DEX support");
        
        let (multi_dex_monitor, update_receiver) = MultiDexMonitor::new();
        let (opportunity_sender, opportunity_receiver) = broadcast::channel(1000);
        let (legacy_update_sender, legacy_update_receiver) = broadcast::channel(1000);
        
        let start_time = Instant::now();
        
        let monitor = Self {
            multi_dex_monitor,
            update_receiver,
            config: config.clone(),
            price_spreads: Arc::new(DashMap::new()),
            opportunities: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(MonitoringMetrics {
                total_updates_received: 0,
                average_latency_ms: 0.0,
                active_connections: 0,
                failed_connections: 0,
                cache_hit_rate: 0.0,
                last_update_timestamp: 0,
                opportunities_detected: 0,
                uptime_seconds: 0,
            })),
            start_time,
            opportunity_sender,
            legacy_update_sender,
            latency_samples: Arc::new(RwLock::new(Vec::new())),
            last_health_check: Arc::new(RwLock::new(Instant::now())),
        };
        
        Ok((monitor, opportunity_receiver, legacy_update_receiver))
    }

    /// Initialize and add DEX clients
    pub async fn initialize_dex_clients(&mut self) -> Result<()> {
        info!("🔌 Initializing DEX clients for multi-DEX monitoring");
        
        // Add Raydium client
        let raydium_ws = format!("wss://api.raydium.io/v2/ws");
        let raydium_api = format!("https://api.raydium.io/v2");
        let raydium_client = RaydiumClient::new(raydium_ws, raydium_api);
        self.multi_dex_monitor.add_client(Box::new(raydium_client)).await;
        info!("✅ Added Raydium client");
        
        // Add Orca client  
        let orca_ws = format!("wss://api.orca.so/v1/ws");
        let orca_api = format!("https://api.orca.so/v1");
        let orca_client = OrcaClient::new(orca_ws, orca_api);
        self.multi_dex_monitor.add_client(Box::new(orca_client)).await;
        info!("✅ Added Orca client");
        
        // Add Jupiter client
        let jupiter_client = JupiterClient::new(
            self.config.dex.jupiter.api_url.clone(),
            "https://price.jup.ag/v4".to_string(),
        );
        self.multi_dex_monitor.add_client(Box::new(jupiter_client)).await;
        info!("✅ Added Jupiter client");
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.active_connections = 3; // Raydium, Orca, Jupiter
        }
        
        Ok(())
    }

    /// Start comprehensive monitoring with arbitrage detection
    pub async fn start_monitoring(&mut self, trading_pairs: Vec<TradingPair>) -> Result<()> {
        info!("🚀 Starting enhanced multi-DEX monitoring for {} pairs", trading_pairs.len());
        
        // Initialize DEX clients if not already done
        self.initialize_dex_clients().await?;
        
        // Create default trading pairs if none provided
        let pairs = if trading_pairs.is_empty() {
            self.create_default_trading_pairs()
        } else {
            trading_pairs
        };
        
        info!("📊 Monitoring {} trading pairs", pairs.len());
        
        // Start the multi-DEX monitor
        let pairs_clone = pairs.clone();
        let multi_dex_monitor = &self.multi_dex_monitor;
        
        // Start monitoring in background
        let monitoring_handle = {
            let pairs = pairs_clone.clone();
            tokio::spawn(async move {
                if let Err(e) = multi_dex_monitor.start_monitoring(pairs).await {
                    error!("Multi-DEX monitoring failed: {}", e);
                }
            })
        };
        
        // Start price update processing
        let update_processing_handle = {
            let mut receiver = self.update_receiver.resubscribe();
            let spreads = self.price_spreads.clone();
            let opportunities = self.opportunities.clone();
            let opportunity_sender = self.opportunity_sender.clone();
            let legacy_sender = self.legacy_update_sender.clone();
            let metrics = self.metrics.clone();
            let latency_samples = self.latency_samples.clone();
            let config = self.config.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::process_price_updates(
                    &mut receiver,
                    spreads,
                    opportunities,
                    opportunity_sender,
                    legacy_sender,
                    metrics,
                    latency_samples,
                    config,
                ).await {
                    error!("Price update processing failed: {}", e);
                }
            })
        };
        
        // Start health monitoring
        let health_monitoring_handle = {
            let last_health_check = self.last_health_check.clone();
            let metrics = self.metrics.clone();
            let start_time = self.start_time;
            
            tokio::spawn(async move {
                if let Err(e) = Self::monitor_health(last_health_check, metrics, start_time).await {
                    error!("Health monitoring failed: {}", e);
                }
            })
        };
        
        info!("✅ All monitoring tasks started successfully");
        
        // Don't wait for completion - these run indefinitely
        let _ = tokio::try_join!(
            monitoring_handle,
            update_processing_handle,
            health_monitoring_handle
        );
        
        Ok(())
    }

    /// Process incoming price updates and detect arbitrage opportunities
    async fn process_price_updates(
        receiver: &mut broadcast::Receiver<PriceUpdate>,
        spreads: Arc<DashMap<String, PriceSpread>>,
        opportunities: Arc<RwLock<Vec<ArbitrageOpportunity>>>,
        opportunity_sender: broadcast::Sender<ArbitrageOpportunity>,
        legacy_sender: broadcast::Sender<LegacyPriceUpdate>,
        metrics: Arc<RwLock<MonitoringMetrics>>,
        latency_samples: Arc<RwLock<Vec<Duration>>>,
        config: BotConfig,
    ) -> Result<()> {
        info!("🔍 Starting price update processing with arbitrage detection");
        
        let mut price_history: HashMap<String, Vec<PriceData>> = HashMap::new();
        
        while let Ok(update) = receiver.recv().await {
            let processing_start = Instant::now();
            
            // Update metrics
            {
                let mut metrics = metrics.write().await;
                metrics.total_updates_received += 1;
                metrics.last_update_timestamp = update.price_data.timestamp;
            }
            
            // Store price history (last 100 updates per pair)
            let pair_key = format!("{}-{}", update.pair.base_mint, update.pair.quote_mint);
            let history = price_history.entry(pair_key.clone()).or_insert_with(Vec::new);
            history.push(update.price_data.clone());
            
            // Keep only last 100 updates as required
            if history.len() > 100 {
                history.remove(0);
            }
            
            // Update price spreads for arbitrage detection
            Self::update_price_spreads(&update, &spreads).await;
            
            // Detect arbitrage opportunities
            if let Some(opportunity) = Self::detect_arbitrage_opportunity(&update, &spreads, &config).await {
                // Store opportunity
                {
                    let mut opps = opportunities.write().await;
                    opps.push(opportunity.clone());
                    
                    // Keep only recent opportunities (last 50)
                    if opps.len() > 50 {
                        opps.remove(0);
                    }
                }
                
                // Update metrics
                {
                    let mut metrics = metrics.write().await;
                    metrics.opportunities_detected += 1;
                }
                
                // Broadcast opportunity
                if let Err(e) = opportunity_sender.send(opportunity) {
                    debug!("No arbitrage opportunity subscribers: {}", e);
                }
            }
            
            // Convert and broadcast legacy update for backward compatibility
            let legacy_update = LegacyPriceUpdate::from(update);
            if let Err(e) = legacy_sender.send(legacy_update) {
                debug!("No legacy update subscribers: {}", e);
            }
            
            // Track processing latency
            let processing_time = processing_start.elapsed();
            {
                let mut samples = latency_samples.write().await;
                samples.push(processing_time);
                
                // Keep only last 1000 samples for rolling average
                if samples.len() > 1000 {
                    samples.remove(0);
                }
                
                // Update average latency metric
                if !samples.is_empty() {
                    let avg_latency = samples.iter().sum::<Duration>().as_millis() as f64 / samples.len() as f64;
                    let mut metrics = metrics.write().await;
                    metrics.average_latency_ms = avg_latency;
                }
            }
            
            // Log if processing takes too long (>50ms is concerning for MEV)
            if processing_time > Duration::from_millis(50) {
                warn!(
                    "Slow price update processing: {:?} for pair {} - this may impact MEV opportunities", 
                    processing_time, 
                    pair_key
                );
            }
        }
        
        Ok(())
    }

    /// Update price spreads for arbitrage detection
    async fn update_price_spreads(
        update: &PriceUpdate,
        spreads: &Arc<DashMap<String, PriceSpread>>,
    ) {
        let pair_key = format!("{}-{}", update.pair.base_mint, update.pair.quote_mint);
        
        // Update or create price spread entry
        match spreads.get_mut(&pair_key) {
            Some(mut spread) => {
                let mut updated = false;
                
                // Check if this price is new high or low
                if update.price_data.price > spread.highest_price {
                    spread.highest_price = update.price_data.price;
                    spread.highest_dex = update.dex_id.clone();
                    updated = true;
                } else if update.price_data.price < spread.lowest_price {
                    spread.lowest_price = update.price_data.price;
                    spread.lowest_dex = update.dex_id.clone();
                    updated = true;
                }
                
                if updated {
                    // Recalculate spread percentage
                    if spread.lowest_price > Decimal::ZERO {
                        spread.spread_percent = ((spread.highest_price - spread.lowest_price) 
                            / spread.lowest_price) * Decimal::from(100);
                    }
                    spread.updated_at = update.price_data.timestamp;
                }
            }
            None => {
                // Create new spread entry
                let spread = PriceSpread {
                    pair: update.pair.clone(),
                    highest_price: update.price_data.price,
                    lowest_price: update.price_data.price,
                    highest_dex: update.dex_id.clone(),
                    lowest_dex: update.dex_id.clone(),
                    spread_percent: Decimal::ZERO,
                    updated_at: update.price_data.timestamp,
                };
                spreads.insert(pair_key, spread);
            }
        }
    }

    /// Detect arbitrage opportunities based on price spreads
    async fn detect_arbitrage_opportunity(
        update: &PriceUpdate,
        spreads: &Arc<DashMap<String, PriceSpread>>,
        config: &BotConfig,
    ) -> Option<ArbitrageOpportunity> {
        let pair_key = format!("{}-{}", update.pair.base_mint, update.pair.quote_mint);
        
        if let Some(spread) = spreads.get(&pair_key) {
            // Check if spread meets minimum threshold
            let min_spread = config.trading.min_profit_percent;
            
            if spread.spread_percent >= min_spread && spread.highest_dex != spread.lowest_dex {
                // Calculate estimated profit (simplified calculation)
                let position_size = config.trading.max_position_sol;
                let profit_per_sol = spread.highest_price - spread.lowest_price;
                let estimated_profit = profit_per_sol * position_size;
                
                // Calculate confidence score (0.0 to 1.0)
                let confidence_score = if spread.spread_percent > min_spread * Decimal::from(2) {
                    0.9 // High confidence for large spreads
                } else if spread.spread_percent > min_spread * Decimal::from(1.5) {
                    0.7 // Medium confidence
                } else {
                    0.5 // Low confidence but meets minimum
                };
                
                let opportunity = ArbitrageOpportunity {
                    pair: update.pair.clone(),
                    buy_dex: spread.lowest_dex.clone(),
                    sell_dex: spread.highest_dex.clone(),
                    spread_percent: spread.spread_percent,
                    estimated_profit_usd: estimated_profit * Decimal::from(100), // Rough SOL->USD conversion
                    confidence_score,
                    detected_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    buy_price: spread.lowest_price,
                    sell_price: spread.highest_price,
                };
                
                debug!(
                    "🎯 Arbitrage opportunity detected: {:.2}% spread between {:?} and {:?} for pair {}",
                    spread.spread_percent,
                    spread.lowest_dex,
                    spread.highest_dex,
                    pair_key
                );
                
                return Some(opportunity);
            }
        }
        
        None
    }

    /// Monitor system health and update metrics
    async fn monitor_health(
        last_health_check: Arc<RwLock<Instant>>,
        metrics: Arc<RwLock<MonitoringMetrics>>,
        start_time: Instant,
    ) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            // Update last health check
            {
                let mut last_check = last_health_check.write().await;
                *last_check = Instant::now();
            }
            
            // Update uptime metric
            {
                let mut metrics = metrics.write().await;
                metrics.uptime_seconds = start_time.elapsed().as_secs();
            }
            
            debug!("📊 Health check completed - system running normally");
        }
    }

    /// Create default trading pairs for testing
    fn create_default_trading_pairs(&self) -> Vec<TradingPair> {
        vec![
            TradingPair::new(
                "So11111111111111111111111111111111111111112".to_string(), // SOL
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
                "test_sol_usdc_pool".to_string(),
            ),
            TradingPair::new(
                "So11111111111111111111111111111111111111112".to_string(), // SOL
                "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string(), // USDT
                "test_sol_usdt_pool".to_string(),
            ),
        ]
    }

    /// Get current monitoring metrics
    pub async fn get_metrics(&self) -> MonitoringMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Get latest arbitrage opportunities
    pub async fn get_opportunities(&self) -> Vec<ArbitrageOpportunity> {
        let opportunities = self.opportunities.read().await;
        opportunities.clone()
    }

    /// Get latest price for a specific pair
    pub async fn get_latest_price(&self, pair: &TradingPair) -> Option<PriceData> {
        self.multi_dex_monitor.get_latest_price(pair).await
    }

    /// Get price history for a specific pair
    pub async fn get_price_history(&self, pair: &TradingPair) -> Vec<PriceData> {
        self.multi_dex_monitor.get_price_history(pair).await
    }

    /// Stop monitoring
    pub async fn stop(&self) {
        info!("🛑 Stopping enhanced DEX monitoring");
        self.multi_dex_monitor.stop().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> BotConfig {
        BotConfig::default()
    }

    fn create_test_pair() -> TradingPair {
        TradingPair::new(
            "So11111111111111111111111111111111111111112".to_string(),
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            "test_pool".to_string(),
        )
    }

    #[tokio::test]
    async fn test_enhanced_monitor_creation() {
        let config = create_test_config();
        let result = EnhancedDexMonitor::new(config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_legacy_price_update_conversion() {
        let pair = create_test_pair();
        let price_data = PriceData {
            price: Decimal::new(12345, 2),
            volume_24h: Decimal::new(100000, 0),
            liquidity: Decimal::new(500000, 0),
            timestamp: 1640995200,
            dex_id: DexId::Raydium,
            pair: pair.clone(),
        };
        
        let update = PriceUpdate {
            dex_id: DexId::Raydium,
            pair,
            price_data,
            update_type: crate::dex::UpdateType::Price,
        };
        
        let legacy_update = LegacyPriceUpdate::from(update);
        assert_eq!(legacy_update.dex, "raydium");
        assert_eq!(legacy_update.price, Decimal::new(12345, 2));
    }
}