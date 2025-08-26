use log::{debug, warn};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Atomic price state that prevents race conditions
#[derive(Debug, Clone)]
pub struct AtomicPriceState {
    // Prices stored as atomic u64 (scaled by 1e9 for precision)
    raydium_price_scaled: Arc<AtomicU64>,
    orca_price_scaled: Arc<AtomicU64>,
    
    // Timestamps for price updates
    raydium_timestamp: Arc<AtomicU64>,
    orca_timestamp: Arc<AtomicU64>,
    
    // Metadata
    last_arbitrage_check: Arc<AtomicU64>,
    price_update_count: Arc<AtomicU64>,
}

impl AtomicPriceState {
    pub fn new() -> Self {
        Self {
            raydium_price_scaled: Arc::new(AtomicU64::new(0)),
            orca_price_scaled: Arc::new(AtomicU64::new(0)),
            raydium_timestamp: Arc::new(AtomicU64::new(0)),
            orca_timestamp: Arc::new(AtomicU64::new(0)),
            last_arbitrage_check: Arc::new(AtomicU64::new(0)),
            price_update_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Update Raydium price atomically
    pub fn update_raydium_price(&self, price: Decimal) {
        let scaled_price = self.decimal_to_scaled_u64(price);
        let timestamp = self.current_timestamp();
        
        self.raydium_price_scaled.store(scaled_price, Ordering::SeqCst);
        self.raydium_timestamp.store(timestamp, Ordering::SeqCst);
        self.price_update_count.fetch_add(1, Ordering::Relaxed);
        
        debug!("📊 Raydium price updated: {} (scaled: {})", price, scaled_price);
    }

    /// Update Orca price atomically
    pub fn update_orca_price(&self, price: Decimal) {
        let scaled_price = self.decimal_to_scaled_u64(price);
        let timestamp = self.current_timestamp();
        
        self.orca_price_scaled.store(scaled_price, Ordering::SeqCst);
        self.orca_timestamp.store(timestamp, Ordering::SeqCst);
        self.price_update_count.fetch_add(1, Ordering::Relaxed);
        
        debug!("🐋 Orca price updated: {} (scaled: {})", price, scaled_price);
    }

    /// Get consistent price snapshot for arbitrage calculation
    pub fn get_price_snapshot(&self) -> PriceSnapshot {
        // Record arbitrage check
        self.last_arbitrage_check.store(self.current_timestamp(), Ordering::Relaxed);

        // Get all values atomically
        let raydium_scaled = self.raydium_price_scaled.load(Ordering::SeqCst);
        let orca_scaled = self.orca_price_scaled.load(Ordering::SeqCst);
        let raydium_ts = self.raydium_timestamp.load(Ordering::SeqCst);
        let orca_ts = self.orca_timestamp.load(Ordering::SeqCst);

        let raydium_price = if raydium_scaled > 0 {
            Some(self.scaled_u64_to_decimal(raydium_scaled))
        } else {
            None
        };

        let orca_price = if orca_scaled > 0 {
            Some(self.scaled_u64_to_decimal(orca_scaled))
        } else {
            None
        };

        PriceSnapshot {
            raydium_price,
            orca_price,
            raydium_timestamp: raydium_ts,
            orca_timestamp: orca_ts,
            snapshot_timestamp: self.current_timestamp(),
            is_valid: raydium_price.is_some() && orca_price.is_some(),
        }
    }

    /// Check if prices are fresh enough for arbitrage
    pub fn are_prices_fresh(&self, max_age_seconds: u64) -> bool {
        let current_time = self.current_timestamp();
        let raydium_ts = self.raydium_timestamp.load(Ordering::Acquire);
        let orca_ts = self.orca_timestamp.load(Ordering::Acquire);

        let raydium_age = current_time.saturating_sub(raydium_ts);
        let orca_age = current_time.saturating_sub(orca_ts);

        raydium_age <= max_age_seconds && orca_age <= max_age_seconds
    }

    /// Get statistics about price updates
    pub fn get_stats(&self) -> PriceStats {
        let current_time = self.current_timestamp();
        let raydium_ts = self.raydium_timestamp.load(Ordering::Acquire);
        let orca_ts = self.orca_timestamp.load(Ordering::Acquire);
        let last_check = self.last_arbitrage_check.load(Ordering::Acquire);
        let update_count = self.price_update_count.load(Ordering::Acquire);

        PriceStats {
            raydium_age_seconds: current_time.saturating_sub(raydium_ts),
            orca_age_seconds: current_time.saturating_sub(orca_ts),
            last_arbitrage_check_age: current_time.saturating_sub(last_check),
            total_updates: update_count,
        }
    }

    // Helper methods
    fn decimal_to_scaled_u64(&self, decimal: Decimal) -> u64 {
        // Scale by 1e9 to preserve precision
        let scaled = decimal * Decimal::from(1_000_000_000u64);
        scaled.to_u64().unwrap_or(0)
    }

    fn scaled_u64_to_decimal(&self, scaled: u64) -> Decimal {
        Decimal::from(scaled) / Decimal::from(1_000_000_000u64)
    }

    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

/// Consistent price snapshot for arbitrage calculations
#[derive(Debug, Clone)]
pub struct PriceSnapshot {
    pub raydium_price: Option<Decimal>,
    pub orca_price: Option<Decimal>,
    pub raydium_timestamp: u64,
    pub orca_timestamp: u64,
    pub snapshot_timestamp: u64,
    pub is_valid: bool,
}

impl PriceSnapshot {
    /// Calculate spread between DEXes
    pub fn calculate_spread(&self) -> Option<Decimal> {
        if let (Some(raydium), Some(orca)) = (self.raydium_price, self.orca_price) {
            Some((raydium - orca).abs())
        } else {
            None
        }
    }

    /// Calculate spread percentage
    pub fn calculate_spread_percent(&self) -> Option<Decimal> {
        if let (Some(raydium), Some(orca)) = (self.raydium_price, self.orca_price) {
            let avg_price = (raydium + orca) / Decimal::from(2);
            if avg_price > Decimal::ZERO {
                Some(((raydium - orca).abs() / avg_price) * Decimal::from(100))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get the better buy/sell prices for arbitrage
    pub fn get_arbitrage_prices(&self) -> Option<(Decimal, Decimal, String, String)> {
        if let (Some(raydium), Some(orca)) = (self.raydium_price, self.orca_price) {
            if raydium > orca {
                // Buy on Orca, sell on Raydium
                Some((orca, raydium, "Orca".to_string(), "Raydium".to_string()))
            } else {
                // Buy on Raydium, sell on Orca
                Some((raydium, orca, "Raydium".to_string(), "Orca".to_string()))
            }
        } else {
            None
        }
    }

    /// Check if snapshot is too old
    pub fn is_stale(&self, max_age_seconds: u64) -> bool {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let age = current_time.saturating_sub(self.snapshot_timestamp);
        age > max_age_seconds
    }
}

/// Statistics about price updates
#[derive(Debug, Clone)]
pub struct PriceStats {
    pub raydium_age_seconds: u64,
    pub orca_age_seconds: u64,
    pub last_arbitrage_check_age: u64,
    pub total_updates: u64,
}

/// Bounded memory manager for trade history
#[derive(Debug)]
pub struct BoundedTradeHistory {
    trades: Arc<RwLock<Vec<TradeRecord>>>,
    max_size: usize,
    cleanup_threshold: usize,
}

impl BoundedTradeHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            trades: Arc::new(RwLock::new(Vec::with_capacity(max_size))),
            max_size,
            cleanup_threshold: max_size * 90 / 100, // Cleanup at 90% capacity
        }
    }

    /// Add trade record with automatic cleanup
    pub async fn add_trade(&self, trade: TradeRecord) {
        let mut trades = self.trades.write().await;
        
        trades.push(trade);
        
        // Cleanup if threshold reached
        if trades.len() >= self.cleanup_threshold {
            self.cleanup_old_trades(&mut trades).await;
        }
    }

    /// Get recent trades
    pub async fn get_recent_trades(&self, count: usize) -> Vec<TradeRecord> {
        let trades = self.trades.read().await;
        trades.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get trades from last N hours
    pub async fn get_trades_since(&self, hours: u64) -> Vec<TradeRecord> {
        let cutoff = SystemTime::now() - Duration::from_secs(hours * 3600);
        let trades = self.trades.read().await;
        
        trades.iter()
            .filter(|trade| trade.timestamp >= cutoff)
            .cloned()
            .collect()
    }

    /// Cleanup old trades, keeping only recent ones
    async fn cleanup_old_trades(&self, trades: &mut Vec<TradeRecord>) {
        // Keep only the most recent 50% of trades
        let keep_count = self.max_size / 2;
        
        if trades.len() > keep_count {
            let remove_count = trades.len() - keep_count;
            trades.drain(0..remove_count);
            
            warn!("🧹 Cleaned up {} old trade records, keeping {}", remove_count, trades.len());
        }
    }

    /// Get memory usage statistics
    pub async fn get_memory_stats(&self) -> MemoryStats {
        let trades = self.trades.read().await;
        
        MemoryStats {
            current_count: trades.len(),
            max_capacity: self.max_size,
            memory_usage_bytes: trades.len() * std::mem::size_of::<TradeRecord>(),
            cleanup_threshold: self.cleanup_threshold,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TradeRecord {
    pub timestamp: SystemTime,
    pub profit_usd: f64,
    pub amount_sol: f64,
    pub success: bool,
    pub dex_buy: String,
    pub dex_sell: String,
}

#[derive(Debug)]
pub struct MemoryStats {
    pub current_count: usize,
    pub max_capacity: usize,
    pub memory_usage_bytes: usize,
    pub cleanup_threshold: usize,
}
