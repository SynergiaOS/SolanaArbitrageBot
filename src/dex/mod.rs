//! DEX Abstraction Layer
//! 
//! This module provides a unified interface for interacting with multiple DEXs
//! on Solana, including Raydium, Orca, and Jupiter. Designed for high-performance
//! real-time price monitoring with WebSocket connections and thread-safe caching.

use anyhow::{Context, Result};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

pub mod clients;
pub use clients::*;

/// Unique identifier for DEX platforms
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum DexId {
    Raydium,
    Orca,
    Jupiter,
    Meteora,
    Phoenix,
    Lifinity,
}

impl DexId {
    pub fn as_str(&self) -> &'static str {
        match self {
            DexId::Raydium => "raydium",
            DexId::Orca => "orca",
            DexId::Jupiter => "jupiter",
            DexId::Meteora => "meteora",
            DexId::Phoenix => "phoenix",
            DexId::Lifinity => "lifinity",
        }
    }
}

/// Trading pair representation
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct TradingPair {
    pub base_mint: String,
    pub quote_mint: String,
    pub pool_address: String,
}

impl TradingPair {
    pub fn new(base_mint: String, quote_mint: String, pool_address: String) -> Self {
        Self {
            base_mint,
            quote_mint,
            pool_address,
        }
    }

    pub fn symbol(&self) -> String {
        format!("{}/{}", 
            self.base_mint.get(0..4).unwrap_or(""), 
            self.quote_mint.get(0..4).unwrap_or("")
        )
    }
}

/// Price data with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub price: Decimal,
    pub volume_24h: Decimal,
    pub liquidity: Decimal,
    pub timestamp: u64,
    pub dex_id: DexId,
    pub pair: TradingPair,
}

/// Price update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceUpdate {
    pub dex_id: DexId,
    pub pair: TradingPair,
    pub price_data: PriceData,
    pub update_type: UpdateType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateType {
    Price,
    Liquidity,
    Volume,
    NewPool,
}

/// Price history buffer storing last 100 updates per pair
#[derive(Debug)]
pub struct PriceHistory {
    updates: VecDeque<PriceData>,
    max_size: usize,
}

impl PriceHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            updates: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn push(&mut self, price_data: PriceData) {
        if self.updates.len() >= self.max_size {
            self.updates.pop_front();
        }
        self.updates.push_back(price_data);
    }

    pub fn latest(&self) -> Option<&PriceData> {
        self.updates.back()
    }

    pub fn get_history(&self) -> &VecDeque<PriceData> {
        &self.updates
    }

    pub fn len(&self) -> usize {
        self.updates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.updates.is_empty()
    }
}

/// Thread-safe price cache with history
pub type PriceCache = Arc<DashMap<TradingPair, Arc<RwLock<PriceHistory>>>>;

/// DEX client trait for unified interface
#[async_trait::async_trait]
pub trait DexClient: Send + Sync {
    /// Get the DEX identifier
    fn dex_id(&self) -> DexId;

    /// Start WebSocket connection for real-time updates
    async fn start_websocket(&self, pairs: Vec<TradingPair>) -> Result<()>;

    /// Fetch current price via HTTP API (fallback method)
    async fn fetch_price(&self, pair: &TradingPair) -> Result<PriceData>;

    /// Get supported trading pairs
    async fn get_supported_pairs(&self) -> Result<Vec<TradingPair>>;

    /// Subscribe to price updates
    async fn subscribe_updates(&self, sender: tokio::sync::mpsc::Sender<PriceUpdate>) -> Result<()>;

    /// Health check for connection status
    async fn health_check(&self) -> Result<bool>;
}

/// Connection manager for WebSocket connections with auto-reconnection
#[derive(Debug)]
pub struct ConnectionManager {
    reconnect_attempts: u32,
    max_reconnect_attempts: u32,
    backoff_base: Duration,
    max_backoff: Duration,
    last_connection_time: Option<Instant>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            reconnect_attempts: 0,
            max_reconnect_attempts: 10,
            backoff_base: Duration::from_millis(100),
            max_backoff: Duration::from_secs(60),
            last_connection_time: None,
        }
    }

    /// Calculate exponential backoff delay
    pub fn backoff_delay(&self) -> Duration {
        let delay = self.backoff_base.as_millis() as u64 
            * 2_u64.pow(self.reconnect_attempts.min(10));
        
        Duration::from_millis(delay.min(self.max_backoff.as_millis() as u64))
    }

    /// Handle connection failure with exponential backoff
    pub async fn handle_connection_failure(&mut self) -> Result<()> {
        self.reconnect_attempts += 1;
        
        if self.reconnect_attempts > self.max_reconnect_attempts {
            error!("Max reconnection attempts reached ({})", self.max_reconnect_attempts);
            return Err(anyhow::anyhow!(
                "Max reconnection attempts reached after {} tries", 
                self.max_reconnect_attempts
            ));
        }

        let delay = self.backoff_delay();
        warn!(
            "Connection failed, attempt {}/{}, retrying in {:?}",
            self.reconnect_attempts, self.max_reconnect_attempts, delay
        );

        tokio::time::sleep(delay).await;
        Ok(())
    }

    /// Reset connection state on successful connection
    pub fn reset_on_success(&mut self) {
        self.reconnect_attempts = 0;
        self.last_connection_time = Some(Instant::now());
        info!("Connection successfully established");
    }

    /// Check if connection is stale
    pub fn is_connection_stale(&self, max_age: Duration) -> bool {
        if let Some(last_time) = self.last_connection_time {
            last_time.elapsed() > max_age
        } else {
            true
        }
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-DEX price monitor coordinator
pub struct MultiDexMonitor {
    clients: Vec<Box<dyn DexClient>>,
    price_cache: PriceCache,
    update_sender: tokio::sync::broadcast::Sender<PriceUpdate>,
    connection_managers: Arc<RwLock<Vec<ConnectionManager>>>,
    is_running: Arc<tokio::sync::RwLock<bool>>,
}

impl MultiDexMonitor {
    /// Create new multi-DEX monitor
    pub fn new() -> (Self, tokio::sync::broadcast::Receiver<PriceUpdate>) {
        let (update_sender, update_receiver) = tokio::sync::broadcast::channel(1000);
        
        (
            Self {
                clients: Vec::new(),
                price_cache: Arc::new(DashMap::new()),
                update_sender,
                connection_managers: Arc::new(RwLock::new(Vec::new())),
                is_running: Arc::new(tokio::sync::RwLock::new(false)),
            },
            update_receiver,
        )
    }

    /// Add a DEX client to the monitor
    pub async fn add_client(&mut self, client: Box<dyn DexClient>) {
        let dex_id = client.dex_id();
        info!("Adding DEX client: {:?}", dex_id);
        
        self.clients.push(client);
        
        // Add connection manager for this client
        let mut managers = self.connection_managers.write().await;
        managers.push(ConnectionManager::new());
    }

    /// Start monitoring all DEXs
    pub async fn start_monitoring(&self, pairs: Vec<TradingPair>) -> Result<()> {
        info!("Starting multi-DEX monitoring for {} pairs", pairs.len());
        
        {
            let mut is_running = self.is_running.write().await;
            *is_running = true;
        }

        // Initialize price cache for all pairs
        for pair in &pairs {
            self.price_cache.insert(
                pair.clone(), 
                Arc::new(RwLock::new(PriceHistory::new(100)))
            );
        }

        // Start monitoring tasks for each DEX client
        let mut tasks = Vec::new();
        
        for (index, client) in self.clients.iter().enumerate() {
            let client_pairs = pairs.clone();
            let cache = self.price_cache.clone();
            let update_sender = self.update_sender.clone();
            let connection_managers = self.connection_managers.clone();
            let is_running = self.is_running.clone();

            let task = tokio::spawn(async move {
                Self::monitor_dex_with_reconnection(
                    client.as_ref(),
                    client_pairs,
                    cache,
                    update_sender,
                    connection_managers,
                    index,
                    is_running,
                ).await
            });
            
            tasks.push(task);
        }

        // Wait for all monitoring tasks
        let results = futures_util::future::join_all(tasks).await;
        
        for (index, result) in results.into_iter().enumerate() {
            match result {
                Ok(Ok(_)) => info!("DEX monitor {} completed successfully", index),
                Ok(Err(e)) => error!("DEX monitor {} failed: {}", index, e),
                Err(e) => error!("DEX monitor {} task panicked: {}", index, e),
            }
        }

        Ok(())
    }

    /// Monitor a single DEX with automatic reconnection
    async fn monitor_dex_with_reconnection(
        client: &dyn DexClient,
        pairs: Vec<TradingPair>,
        cache: PriceCache,
        update_sender: tokio::sync::broadcast::Sender<PriceUpdate>,
        connection_managers: Arc<RwLock<Vec<ConnectionManager>>>,
        client_index: usize,
        is_running: Arc<tokio::sync::RwLock<bool>>,
    ) -> Result<()> {
        let dex_id = client.dex_id();
        info!("Starting monitoring for {:?} with {} pairs", dex_id, pairs.len());

        loop {
            // Check if we should continue running
            {
                let running = is_running.read().await;
                if !*running {
                    info!("Stopping monitor for {:?}", dex_id);
                    break;
                }
            }

            // Get connection manager for this client
            let connection_manager = {
                let mut managers = connection_managers.write().await;
                &mut managers[client_index]
            };

            // Attempt to start WebSocket connection
            match client.start_websocket(pairs.clone()).await {
                Ok(()) => {
                    connection_manager.reset_on_success();
                    
                    // Create update subscription
                    let (price_sender, mut price_receiver) = tokio::sync::mpsc::channel(100);
                    
                    // Subscribe to updates
                    if let Err(e) = client.subscribe_updates(price_sender).await {
                        error!("Failed to subscribe to updates for {:?}: {}", dex_id, e);
                        continue;
                    }

                    // Process price updates
                    while let Some(update) = price_receiver.recv().await {
                        let running = is_running.read().await;
                        if !*running {
                            break;
                        }

                        // Update cache
                        if let Some(history) = cache.get(&update.pair) {
                            let mut history = history.write().await;
                            history.push(update.price_data.clone());
                        }

                        // Broadcast update
                        if let Err(e) = update_sender.send(update) {
                            debug!("No active update receivers: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("WebSocket connection failed for {:?}: {}", dex_id, e);
                    
                    if let Err(reconnect_err) = connection_manager.handle_connection_failure().await {
                        error!("Failed to handle reconnection for {:?}: {}", dex_id, reconnect_err);
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Get latest price for a trading pair
    pub async fn get_latest_price(&self, pair: &TradingPair) -> Option<PriceData> {
        if let Some(history) = self.price_cache.get(pair) {
            let history = history.read().await;
            history.latest().cloned()
        } else {
            None
        }
    }

    /// Get price history for a trading pair
    pub async fn get_price_history(&self, pair: &TradingPair) -> Vec<PriceData> {
        if let Some(history) = self.price_cache.get(pair) {
            let history = history.read().await;
            history.get_history().iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Stop monitoring
    pub async fn stop(&self) {
        info!("Stopping multi-DEX monitoring");
        let mut is_running = self.is_running.write().await;
        *is_running = false;
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let total_pairs = self.price_cache.len();
        let mut total_updates = 0;

        for entry in self.price_cache.iter() {
            let history = entry.value().read().await;
            total_updates += history.len();
        }

        (total_pairs, total_updates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    fn create_test_pair() -> TradingPair {
        TradingPair::new(
            "So11111111111111111111111111111111111111112".to_string(),
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            "test_pool_address".to_string(),
        )
    }

    fn create_test_price_data() -> PriceData {
        PriceData {
            price: Decimal::new(12345, 2), // 123.45
            volume_24h: Decimal::new(100000, 0),
            liquidity: Decimal::new(500000, 0),
            timestamp: std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            dex_id: DexId::Raydium,
            pair: create_test_pair(),
        }
    }

    #[test]
    fn test_trading_pair_symbol() {
        let pair = create_test_pair();
        let symbol = pair.symbol();
        assert!(symbol.contains("So11"));
        assert!(symbol.contains("EPjF"));
    }

    #[test]
    fn test_price_history_buffer() {
        let mut history = PriceHistory::new(3);
        
        // Add prices
        for i in 1..=5 {
            let mut price_data = create_test_price_data();
            price_data.price = Decimal::new(i, 0);
            history.push(price_data);
        }

        // Should only keep last 3 updates
        assert_eq!(history.len(), 3);
        assert_eq!(history.latest().unwrap().price, Decimal::new(5, 0));
        
        // Check that oldest entries were removed
        let history_vec: Vec<_> = history.get_history().iter().collect();
        assert_eq!(history_vec[0].price, Decimal::new(3, 0));
    }

    #[test]
    fn test_connection_manager_backoff() {
        let mut manager = ConnectionManager::new();
        
        // First attempt should have base delay
        let delay1 = manager.backoff_delay();
        assert_eq!(delay1, Duration::from_millis(100));
        
        // Simulate failures
        manager.reconnect_attempts = 3;
        let delay2 = manager.backoff_delay();
        assert_eq!(delay2, Duration::from_millis(800)); // 100 * 2^3
        
        // Test max backoff
        manager.reconnect_attempts = 20;
        let delay3 = manager.backoff_delay();
        assert_eq!(delay3, Duration::from_secs(60));
    }

    #[test]
    fn test_dex_id_serialization() {
        let dex_id = DexId::Raydium;
        assert_eq!(dex_id.as_str(), "raydium");
        
        // Test serialization/deserialization
        let json = serde_json::to_string(&dex_id).unwrap();
        let deserialized: DexId = serde_json::from_str(&json).unwrap();
        assert_eq!(dex_id, deserialized);
    }

    #[tokio::test]
    async fn test_multi_dex_monitor_creation() {
        let (monitor, _receiver) = MultiDexMonitor::new();
        let stats = monitor.get_cache_stats().await;
        assert_eq!(stats.0, 0); // No pairs initially
        assert_eq!(stats.1, 0); // No updates initially
    }
}