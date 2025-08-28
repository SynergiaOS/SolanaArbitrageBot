# Backtesting Data Formats and Sources

## Overview

The backtesting engine supports multiple data sources to provide comprehensive historical market data for strategy validation. This document outlines the expected data formats and integration methods.

## Data Sources

### 1. CSV Files

#### Price Data Format (`*_prices.csv`)
```csv
timestamp,dex,pair,price,volume_24h,liquidity,spread_bps
2024-01-01T00:00:00Z,raydium,SOL/USDC,98.45,1250000.0,2500000.0,25
2024-01-01T00:05:00Z,raydium,SOL/USDC,98.52,1251000.0,2501000.0,26
2024-01-01T00:00:00Z,orca,SOL/USDC,98.48,980000.0,1800000.0,28
2024-01-01T00:05:00Z,orca,SOL/USDC,98.50,981000.0,1801000.0,27
```

**Field Descriptions:**
- `timestamp`: ISO 8601 UTC timestamp
- `dex`: DEX identifier (raydium, orca, jupiter, meteora, etc.)
- `pair`: Trading pair (SOL/USDC, RAY/USDC, etc.)
- `price`: Token price in quote currency
- `volume_24h`: 24-hour trading volume (optional)
- `liquidity`: Total liquidity in pool (optional)
- `spread_bps`: Bid-ask spread in basis points (optional)

#### Market Conditions Format (`YYYY-MM-DD_conditions.csv`)
```csv
timestamp,network_congestion,volatility,success_rate,avg_gas_price
2024-01-01T00:00:00Z,low,0.015,0.98,5000
2024-01-01T00:05:00Z,low,0.016,0.98,5200
2024-01-01T01:00:00Z,medium,0.025,0.95,8000
2024-01-01T02:00:00Z,high,0.045,0.89,15000
```

**Field Descriptions:**
- `timestamp`: ISO 8601 UTC timestamp
- `network_congestion`: Network congestion level (low, medium, high, extreme)
- `volatility`: Market volatility measure (standard deviation of returns)
- `success_rate`: Historical transaction success rate (0.0-1.0)
- `avg_gas_price`: Average gas price in lamports

### 2. Database Schema

#### PostgreSQL Tables

```sql
-- Price data table with high performance indexing
CREATE TABLE historical_prices (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    dex TEXT NOT NULL,
    pair TEXT NOT NULL,
    price DECIMAL(20,9) NOT NULL,
    volume_24h DECIMAL(20,9),
    liquidity DECIMAL(20,9),
    spread_bps INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Optimized indexes for backtesting queries
CREATE INDEX idx_prices_timestamp_dex_pair ON historical_prices (timestamp, dex, pair);
CREATE INDEX idx_prices_pair_timestamp ON historical_prices (pair, timestamp);
CREATE INDEX idx_prices_dex_timestamp ON historical_prices (dex, timestamp);

-- Partitioning for large datasets (optional)
CREATE TABLE historical_prices_2024_01 PARTITION OF historical_prices
    FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');

-- Market conditions table
CREATE TABLE market_conditions (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    network_congestion TEXT NOT NULL CHECK (network_congestion IN ('low', 'medium', 'high', 'extreme')),
    volatility DECIMAL(8,4) NOT NULL CHECK (volatility >= 0),
    success_rate DECIMAL(5,4) NOT NULL CHECK (success_rate >= 0 AND success_rate <= 1),
    avg_gas_price BIGINT NOT NULL CHECK (avg_gas_price > 0),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_conditions_timestamp ON market_conditions (timestamp);

-- Order book depth data (for advanced slippage modeling)
CREATE TABLE order_book_snapshots (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    dex TEXT NOT NULL,
    pair TEXT NOT NULL,
    bids JSONB NOT NULL, -- [{"price": "98.45", "size": "1000"}, ...]
    asks JSONB NOT NULL, -- [{"price": "98.50", "size": "800"}, ...]
    spread_bps INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_orderbook_timestamp_dex_pair ON order_book_snapshots (timestamp, dex, pair);

-- Backtest results storage
CREATE TABLE backtest_runs (
    id SERIAL PRIMARY KEY,
    run_name TEXT NOT NULL,
    strategy_name TEXT NOT NULL,
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ NOT NULL,
    initial_capital DECIMAL(12,2) NOT NULL,
    final_capital DECIMAL(12,2) NOT NULL,
    total_return DECIMAL(8,4) NOT NULL,
    sharpe_ratio DECIMAL(8,4),
    max_drawdown DECIMAL(8,4),
    win_rate DECIMAL(5,4),
    total_trades INTEGER NOT NULL,
    strategy_config JSONB NOT NULL,
    simulation_config JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_backtest_runs_name_date ON backtest_runs (run_name, created_at);
CREATE INDEX idx_backtest_runs_strategy ON backtest_runs (strategy_name, created_at);

-- Individual simulated trades
CREATE TABLE simulated_trades (
    id BIGSERIAL PRIMARY KEY,
    backtest_run_id INTEGER REFERENCES backtest_runs(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL,
    strategy TEXT NOT NULL,
    pair TEXT NOT NULL,
    buy_dex TEXT NOT NULL,
    sell_dex TEXT NOT NULL,
    amount_sol DECIMAL(20,9) NOT NULL,
    expected_profit_usd DECIMAL(10,4) NOT NULL,
    actual_profit_usd DECIMAL(10,4) NOT NULL,
    gas_cost_sol DECIMAL(20,9) NOT NULL,
    slippage_percent DECIMAL(8,4) NOT NULL,
    execution_latency_ms INTEGER NOT NULL,
    success BOOLEAN NOT NULL,
    failure_reason TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_simulated_trades_run_id ON simulated_trades (backtest_run_id);
CREATE INDEX idx_simulated_trades_timestamp ON simulated_trades (timestamp);
```

### 3. Solana RPC Historical Data

#### On-chain Data Extraction

```rust
// Example: Extract historical price data from Raydium pools
pub struct SolanaRpcDataCollector {
    client: RpcClient,
    program_ids: HashMap<String, Pubkey>,
}

impl SolanaRpcDataCollector {
    pub async fn extract_pool_history(
        &self,
        pool_address: &Pubkey,
        start_slot: u64,
        end_slot: u64,
    ) -> Result<Vec<PricePoint>> {
        let mut price_points = Vec::new();
        
        // Get account history for the pool
        let account_history = self.client
            .get_account_history(pool_address, start_slot, end_slot)
            .await?;
            
        for (slot, account_data) in account_history {
            if let Some(price_data) = self.parse_pool_data(&account_data)? {
                let timestamp = self.slot_to_timestamp(slot).await?;
                price_points.push(PricePoint {
                    timestamp,
                    dex: "raydium".to_string(),
                    pair: price_data.pair,
                    price: price_data.price,
                    volume_24h: price_data.volume,
                    liquidity: Some(price_data.liquidity),
                    spread_bps: None,
                });
            }
        }
        
        Ok(price_points)
    }
}
```

#### Transaction Log Analysis

```rust
// Extract trading activity from transaction logs
pub async fn extract_swap_history(
    &self,
    program_id: &Pubkey,
    start_slot: u64,
    end_slot: u64,
) -> Result<Vec<SwapEvent>> {
    let signatures = self.client
        .get_signatures_for_address_with_config(
            program_id,
            GetSignaturesForAddressConfig {
                before: None,
                until: None,
                limit: Some(1000),
            },
        )
        .await?;
        
    let mut swap_events = Vec::new();
    
    for signature_info in signatures {
        let transaction = self.client
            .get_transaction(&signature_info.signature, UiTransactionEncoding::Json)
            .await?;
            
        if let Some(swap_data) = self.parse_swap_transaction(&transaction)? {
            swap_events.push(swap_data);
        }
    }
    
    Ok(swap_events)
}
```

### 4. Live Data Integration

#### WebSocket Data Streaming

```rust
// Real-time price feed integration
pub struct LiveDataCollector {
    price_buffer: Arc<Mutex<VecDeque<PricePoint>>>,
    condition_buffer: Arc<Mutex<VecDeque<MarketCondition>>>,
    buffer_duration: Duration,
}

impl LiveDataCollector {
    pub async fn start_collection(&mut self) -> Result<()> {
        let raydium_stream = self.connect_raydium_websocket().await?;
        let orca_stream = self.connect_orca_websocket().await?;
        let jupiter_stream = self.connect_jupiter_websocket().await?;
        
        // Spawn tasks for each data stream
        tokio::spawn(self.process_raydium_stream(raydium_stream));
        tokio::spawn(self.process_orca_stream(orca_stream));
        tokio::spawn(self.process_jupiter_stream(jupiter_stream));
        
        // Start buffer management task
        tokio::spawn(self.manage_buffer_cleanup());
        
        Ok(())
    }
}
```

## Data Validation and Quality

### Data Quality Checks

```rust
pub struct DataValidator {
    price_range_limits: HashMap<String, (Decimal, Decimal)>,
    volume_limits: HashMap<String, (Decimal, Decimal)>,
    timestamp_tolerance: Duration,
}

impl DataValidator {
    pub fn validate_price_data(&self, data: &[PricePoint]) -> ValidationResult {
        let mut issues = Vec::new();
        
        for point in data {
            // Check price ranges
            if let Some((min_price, max_price)) = self.price_range_limits.get(&point.pair) {
                if point.price < *min_price || point.price > *max_price {
                    issues.push(ValidationIssue::PriceOutOfRange {
                        timestamp: point.timestamp,
                        pair: point.pair.clone(),
                        price: point.price,
                    });
                }
            }
            
            // Check for gaps in data
            // Check for duplicate entries
            // Validate timestamp ordering
        }
        
        ValidationResult { issues }
    }
    
    pub fn fill_data_gaps(&self, data: &mut Vec<PricePoint>) -> Result<()> {
        data.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        
        let mut filled_data = Vec::new();
        
        for window in data.windows(2) {
            filled_data.push(window[0].clone());
            
            let gap = window[1].timestamp - window[0].timestamp;
            if gap > Duration::minutes(10) { // 10 minute gap threshold
                // Fill gap with interpolated values
                let interpolated_points = self.interpolate_price_data(&window[0], &window[1])?;
                filled_data.extend(interpolated_points);
            }
        }
        
        if let Some(last) = data.last() {
            filled_data.push(last.clone());
        }
        
        *data = filled_data;
        Ok(())
    }
}
```

### Data Preprocessing

```rust
pub struct DataPreprocessor {
    outlier_threshold: f64,
    smoothing_window: usize,
}

impl DataPreprocessor {
    pub fn preprocess_price_data(&self, data: &mut Vec<PricePoint>) -> Result<()> {
        // 1. Remove outliers
        self.remove_price_outliers(data)?;
        
        // 2. Smooth noise (optional)
        if self.smoothing_window > 1 {
            self.apply_moving_average_smoothing(data)?;
        }
        
        // 3. Calculate additional metrics
        self.calculate_volatility_indicators(data)?;
        self.calculate_liquidity_metrics(data)?;
        
        Ok(())
    }
    
    fn remove_price_outliers(&self, data: &mut Vec<PricePoint>) -> Result<()> {
        for pair_data in data.chunks_mut(100) { // Process in chunks
            let prices: Vec<f64> = pair_data.iter()
                .map(|p| p.price.to_f64().unwrap_or(0.0))
                .collect();
                
            let (q1, q3) = self.calculate_quartiles(&prices);
            let iqr = q3 - q1;
            let lower_bound = q1 - 1.5 * iqr;
            let upper_bound = q3 + 1.5 * iqr;
            
            // Mark outliers for removal or correction
            for point in pair_data.iter_mut() {
                let price_f64 = point.price.to_f64().unwrap_or(0.0);
                if price_f64 < lower_bound || price_f64 > upper_bound {
                    // Option 1: Remove the point
                    // Option 2: Replace with interpolated value
                    // Option 3: Flag for manual review
                }
            }
        }
        
        Ok(())
    }
}
```

## Usage Examples

### Loading Historical Data

```rust
// Example: Load 30 days of SOL/USDC data from multiple sources
use solana_arbitrage_bot::backtesting::data::*;

let csv_source = CsvDataSource::new("./data/historical/")?;
let db_source = DatabaseDataSource::new(db_pool)?;

let end_date = Utc::now();
let start_date = end_date - Duration::days(30);

// Load from CSV
let csv_data = csv_source.load_price_data("SOL/USDC", start_date, end_date).await?;
println!("Loaded {} price points from CSV", csv_data.len());

// Load from database
let db_data = db_source.load_price_data("SOL/USDC", start_date, end_date).await?;
println!("Loaded {} price points from database", db_data.len());

// Merge and deduplicate data
let merged_data = merge_price_data(vec![csv_data, db_data])?;
println!("Merged dataset contains {} unique price points", merged_data.len());
```

### Data Export for Analysis

```rust
// Export backtest data to different formats
pub struct DataExporter;

impl DataExporter {
    pub fn export_to_csv(data: &[PricePoint], path: &Path) -> Result<()> {
        let mut writer = csv::Writer::from_path(path)?;
        
        for point in data {
            writer.serialize(point)?;
        }
        
        writer.flush()?;
        Ok(())
    }
    
    pub fn export_to_parquet(data: &[PricePoint], path: &Path) -> Result<()> {
        // High-performance columnar format for large datasets
        // Implementation using arrow/parquet crates
        todo!("Implement Parquet export for large datasets")
    }
    
    pub fn export_backtest_results(results: &BacktestResults, format: ExportFormat) -> Result<Vec<u8>> {
        match format {
            ExportFormat::Json => serde_json::to_vec_pretty(results).map_err(Into::into),
            ExportFormat::Csv => {
                let mut output = Vec::new();
                let mut writer = csv::Writer::from_writer(&mut output);
                
                // Export summary metrics
                writer.write_record(&["Metric", "Value"])?;
                writer.write_record(&["Total Return", &results.total_return.to_string()])?;
                writer.write_record(&["Sharpe Ratio", &results.sharpe_ratio.to_string()])?;
                writer.write_record(&["Max Drawdown", &results.max_drawdown.to_string()])?;
                
                // Export trade details
                for trade in &results.trades {
                    writer.serialize(trade)?;
                }
                
                writer.flush()?;
                Ok(output)
            }
        }
    }
}
```

## Performance Optimization

### Efficient Data Loading

```rust
// Optimized data loading for large datasets
pub struct OptimizedDataLoader {
    connection_pool: Arc<Pool<PostgresConnectionManager<NoTls>>>,
    cache: Arc<RwLock<LruCache<String, Arc<Vec<PricePoint>>>>>,
}

impl OptimizedDataLoader {
    pub async fn load_price_data_parallel(
        &self,
        pairs: &[String],
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<HashMap<String, Vec<PricePoint>>> {
        let futures: Vec<_> = pairs
            .iter()
            .map(|pair| self.load_single_pair_data(pair, start, end))
            .collect();
            
        let results = futures::future::try_join_all(futures).await?;
        
        let mut data_map = HashMap::new();
        for (pair, data) in pairs.iter().zip(results) {
            data_map.insert(pair.clone(), data);
        }
        
        Ok(data_map)
    }
    
    async fn load_single_pair_data(
        &self,
        pair: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PricePoint>> {
        let cache_key = format!("{}:{}:{}", pair, start.timestamp(), end.timestamp());
        
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached_data) = cache.get(&cache_key) {
                return Ok((**cached_data).clone());
            }
        }
        
        // Load from database with optimized query
        let conn = self.connection_pool.get().await?;
        let query = "
            SELECT timestamp, dex, pair, price, volume_24h, liquidity, spread_bps
            FROM historical_prices 
            WHERE pair = $1 AND timestamp BETWEEN $2 AND $3
            ORDER BY timestamp ASC
        ";
        
        let rows = conn.query(query, &[&pair, &start, &end]).await?;
        let mut price_points = Vec::with_capacity(rows.len());
        
        for row in rows {
            price_points.push(PricePoint {
                timestamp: row.get(0),
                dex: row.get(1),
                pair: row.get(2),
                price: row.get(3),
                volume_24h: row.get(4),
                liquidity: row.get(5),
                spread_bps: row.get(6),
            });
        }
        
        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.put(cache_key, Arc::new(price_points.clone()));
        }
        
        Ok(price_points)
    }
}
```

This comprehensive data format specification provides the foundation for implementing a robust backtesting system that can handle multiple data sources, ensure data quality, and provide optimal performance for large-scale historical analysis.