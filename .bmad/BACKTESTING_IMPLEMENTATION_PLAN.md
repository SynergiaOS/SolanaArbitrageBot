# Solana Arbitrage Bot - Backtesting Engine Implementation Plan

## Overview

This document outlines the step-by-step implementation plan for the comprehensive backtesting engine described in ARCHITECTURE.md. The implementation follows a modular approach that integrates seamlessly with existing bot components.

## Implementation Phases

### Phase 1: Core Infrastructure (Week 1-2)

#### 1.1 Module Structure Creation
```
src/backtesting/
├── mod.rs                    # Main module with public API
├── data/
│   ├── mod.rs               # Data source trait and implementations
│   ├── csv.rs               # CSV data source
│   ├── database.rs          # PostgreSQL/SQLite data source
│   ├── rpc.rs               # Solana RPC historical data
│   └── live.rs              # Live data stream integration
├── simulation/
│   ├── mod.rs               # Simulation engine
│   ├── executor.rs          # Mock executor with realistic conditions
│   ├── models/
│   │   ├── mod.rs
│   │   ├── latency.rs       # Latency simulation models
│   │   ├── slippage.rs      # Slippage calculation models
│   │   └── fees.rs          # Fee calculation models
│   └── strategy.rs          # Strategy simulation wrapper
├── market/
│   ├── mod.rs               # Market simulation
│   ├── orderbook.rs         # Order book simulation
│   ├── network.rs           # Network condition simulation
│   └── conditions.rs        # Market condition modeling
├── analytics/
│   ├── mod.rs               # Performance analytics
│   ├── metrics.rs           # Financial metrics calculation
│   ├── risk.rs              # Risk analysis
│   └── comparison.rs        # Strategy comparison
├── reports/
│   ├── mod.rs               # Report generation
│   ├── html.rs              # HTML dashboard generator
│   ├── json.rs              # JSON export
│   ├── csv.rs               # CSV export
│   └── templates/           # HTML templates
│       ├── dashboard.html
│       ├── charts.js
│       └── styles.css
└── runner.rs                # Main backtest runner
```

#### 1.2 Configuration Integration
Update `src/config_manager.rs` to include backtesting configuration:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct BacktestingConfig {
    pub data_sources: Vec<DataSourceConfig>,
    pub simulation: SimulationConfig,
    pub analysis: AnalysisConfig,
    pub reporting: ReportingConfig,
    pub optimization: OptimizationConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DataSourceConfig {
    pub source_type: DataSourceType,
    pub path: Option<String>,
    pub url: Option<String>,
    pub pairs: Vec<String>,
    pub cache_size: Option<usize>,
    pub lookback_days: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceType {
    Csv,
    Database,
    Rpc,
    Live,
}
```

### Phase 2: Data Layer Implementation (Week 2-3)

#### 2.1 Core Data Types
```rust
// src/backtesting/data/mod.rs
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    pub timestamp: DateTime<Utc>,
    pub dex: String,
    pub pair: String,
    pub price: Decimal,
    pub volume_24h: Option<Decimal>,
    pub liquidity: Option<Decimal>,
    pub spread_bps: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct MarketCondition {
    pub timestamp: DateTime<Utc>,
    pub network_congestion: NetworkCongestion,
    pub volatility: Decimal,
    pub success_rate: Decimal,
    pub avg_gas_price: u64,
}

#[async_trait::async_trait]
pub trait DataSource: Send + Sync {
    async fn load_price_data(
        &self,
        pair: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PricePoint>>;
    
    async fn load_market_conditions(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MarketCondition>>;
    
    fn source_name(&self) -> &str;
    fn supported_pairs(&self) -> Vec<String>;
}
```

#### 2.2 CSV Data Source Implementation
```rust
// src/backtesting/data/csv.rs
pub struct CsvDataSource {
    base_path: PathBuf,
    price_files: HashMap<String, PathBuf>,
    condition_files: HashMap<String, PathBuf>,
}

impl CsvDataSource {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        
        // Scan for price data files (format: {pair}_{dex}_prices.csv)
        let mut price_files = HashMap::new();
        let mut condition_files = HashMap::new();
        
        for entry in std::fs::read_dir(&base_path)? {
            let entry = entry?;
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();
            
            if filename_str.ends_with("_prices.csv") {
                // Extract pair from filename
                let pair = filename_str
                    .strip_suffix("_prices.csv")
                    .unwrap()
                    .to_string();
                price_files.insert(pair, entry.path());
            } else if filename_str.ends_with("_conditions.csv") {
                let date = filename_str
                    .strip_suffix("_conditions.csv")
                    .unwrap()
                    .to_string();
                condition_files.insert(date, entry.path());
            }
        }
        
        Ok(Self {
            base_path,
            price_files,
            condition_files,
        })
    }
}

#[async_trait::async_trait]
impl DataSource for CsvDataSource {
    async fn load_price_data(
        &self,
        pair: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PricePoint>> {
        let file_path = self.price_files.get(pair)
            .ok_or_else(|| anyhow!("No price data file found for pair: {}", pair))?;
            
        let mut reader = csv::Reader::from_path(file_path)?;
        let mut points = Vec::new();
        
        for result in reader.deserialize() {
            let point: PricePoint = result?;
            if point.timestamp >= start && point.timestamp <= end {
                points.push(point);
            }
        }
        
        points.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(points)
    }
}
```

### Phase 3: Simulation Engine (Week 3-4)

#### 3.1 Mock Executor with Realistic Conditions
```rust
// src/backtesting/simulation/executor.rs
use crate::calculator::ArbitrageOpportunity;
use crate::backtesting::market::MarketCondition;

pub struct MockExecutor {
    latency_model: Box<dyn LatencyModel>,
    slippage_model: Box<dyn SlippageModel>,
    fee_model: Box<dyn FeeModel>,
    success_rate_model: Box<dyn SuccessRateModel>,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub success: bool,
    pub actual_profit_usd: Decimal,
    pub gas_cost_sol: Decimal,
    pub slippage_percent: Decimal,
    pub execution_latency: Duration,
    pub failure_reason: Option<String>,
}

impl MockExecutor {
    pub fn new(config: &SimulationConfig) -> Self {
        Self {
            latency_model: Box::new(RealisticLatencyModel::from_config(config)),
            slippage_model: Box::new(DepthBasedSlippageModel::from_config(config)),
            fee_model: Box::new(DynamicFeeModel::from_config(config)),
            success_rate_model: Box::new(ConditionBasedSuccessModel::from_config(config)),
        }
    }
    
    pub async fn simulate_execution(
        &self,
        opportunity: &ArbitrageOpportunity,
        market_condition: &MarketCondition,
        order_book_data: &OrderBookSnapshot,
    ) -> ExecutionResult {
        // Calculate execution latency
        let latency = self.latency_model.simulate_latency(market_condition);
        
        // Check if execution would succeed
        let success_prob = self.success_rate_model.calculate_success_probability(
            opportunity,
            market_condition,
            latency,
        );
        
        let mut rng = rand::thread_rng();
        let success = rng.gen::<f64>() < success_prob;
        
        if !success {
            return ExecutionResult {
                success: false,
                actual_profit_usd: Decimal::ZERO,
                gas_cost_sol: self.fee_model.calculate_gas_cost(market_condition),
                slippage_percent: Decimal::ZERO,
                execution_latency: latency,
                failure_reason: Some("Execution failed due to network conditions".to_string()),
            };
        }
        
        // Calculate realistic slippage
        let slippage = self.slippage_model.calculate_slippage(
            opportunity.amount_sol.into(),
            order_book_data.liquidity,
            market_condition.volatility.to_f64().unwrap_or(0.01),
        );
        
        // Calculate actual profit after slippage
        let slippage_impact = opportunity.expected_profit_usd * slippage.to_f64().unwrap_or(0.0);
        let actual_profit = opportunity.profit_after_fees_usd - slippage_impact;
        
        ExecutionResult {
            success: true,
            actual_profit_usd: Decimal::from_f64(actual_profit)
                .unwrap_or(Decimal::ZERO),
            gas_cost_sol: self.fee_model.calculate_gas_cost(market_condition),
            slippage_percent: slippage,
            execution_latency: latency,
            failure_reason: None,
        }
    }
}
```

#### 3.2 Latency Models
```rust
// src/backtesting/simulation/models/latency.rs
pub trait LatencyModel: Send + Sync {
    fn simulate_latency(&self, conditions: &MarketCondition) -> Duration;
}

pub struct RealisticLatencyModel {
    base_latency_ms: u64,
    congestion_multipliers: HashMap<NetworkCongestion, f64>,
    volatility_impact_factor: f64,
    jitter_std_dev: f64,
}

impl RealisticLatencyModel {
    pub fn from_config(config: &SimulationConfig) -> Self {
        let mut congestion_multipliers = HashMap::new();
        congestion_multipliers.insert(NetworkCongestion::Low, 1.0);
        congestion_multipliers.insert(NetworkCongestion::Medium, 1.5);
        congestion_multipliers.insert(NetworkCongestion::High, 2.5);
        congestion_multipliers.insert(NetworkCongestion::Extreme, 5.0);
        
        Self {
            base_latency_ms: config.base_latency_ms.unwrap_or(150),
            congestion_multipliers,
            volatility_impact_factor: config.volatility_impact.unwrap_or(0.3),
            jitter_std_dev: config.latency_jitter.unwrap_or(20.0),
        }
    }
}

impl LatencyModel for RealisticLatencyModel {
    fn simulate_latency(&self, conditions: &MarketCondition) -> Duration {
        let base_ms = self.base_latency_ms as f64;
        
        // Apply congestion multiplier
        let congestion_factor = self.congestion_multipliers
            .get(&conditions.network_congestion)
            .unwrap_or(&1.0);
        
        // Apply volatility impact (higher volatility = higher latency due to increased traffic)
        let volatility_factor = 1.0 + (conditions.volatility.to_f64().unwrap_or(0.01) * self.volatility_impact_factor);
        
        // Add random jitter using normal distribution
        let mut rng = rand::thread_rng();
        let jitter = rng.sample(Normal::new(0.0, self.jitter_std_dev).unwrap());
        
        let total_latency_ms = (base_ms * congestion_factor * volatility_factor + jitter).max(10.0);
        
        Duration::from_millis(total_latency_ms as u64)
    }
}
```

### Phase 4: Analytics and Reporting (Week 4-5)

#### 4.1 Performance Analytics
```rust
// src/backtesting/analytics/metrics.rs
pub struct PerformanceAnalyzer {
    trades: Vec<SimulatedTrade>,
    initial_capital: Decimal,
    risk_free_rate: Decimal,
}

impl PerformanceAnalyzer {
    pub fn calculate_sharpe_ratio(&self) -> Decimal {
        if self.trades.is_empty() {
            return Decimal::ZERO;
        }
        
        let returns: Vec<Decimal> = self.trades
            .iter()
            .map(|trade| trade.return_percentage)
            .collect();
            
        let mean_return = returns.iter().sum::<Decimal>() / Decimal::from(returns.len());
        let excess_return = mean_return - self.risk_free_rate / Decimal::from(252); // Daily risk-free rate
        
        if returns.len() < 2 {
            return Decimal::ZERO;
        }
        
        let variance = returns
            .iter()
            .map(|r| (*r - mean_return).powi(2))
            .sum::<Decimal>() / Decimal::from(returns.len() - 1);
            
        let std_dev = variance.sqrt().unwrap_or(Decimal::ZERO);
        
        if std_dev.is_zero() {
            Decimal::ZERO
        } else {
            excess_return / std_dev * Decimal::from(252).sqrt().unwrap_or(Decimal::ONE)
        }
    }
    
    pub fn calculate_max_drawdown(&self) -> Decimal {
        if self.trades.is_empty() {
            return Decimal::ZERO;
        }
        
        let mut cumulative_return = Decimal::ONE;
        let mut peak = Decimal::ONE;
        let mut max_drawdown = Decimal::ZERO;
        
        for trade in &self.trades {
            cumulative_return *= Decimal::ONE + trade.return_percentage;
            
            if cumulative_return > peak {
                peak = cumulative_return;
            }
            
            let drawdown = (peak - cumulative_return) / peak;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }
        
        max_drawdown
    }
    
    pub fn calculate_value_at_risk(&self, confidence_level: f64) -> Decimal {
        if self.trades.is_empty() {
            return Decimal::ZERO;
        }
        
        let mut returns: Vec<f64> = self.trades
            .iter()
            .map(|trade| trade.return_percentage.to_f64().unwrap_or(0.0))
            .collect();
            
        returns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        let index = ((1.0 - confidence_level) * returns.len() as f64) as usize;
        let var = returns.get(index).unwrap_or(&0.0);
        
        Decimal::from_f64(*var).unwrap_or(Decimal::ZERO)
    }
}
```

#### 4.2 HTML Report Generation
```rust
// src/backtesting/reports/html.rs
use askama::Template;

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct BacktestDashboard {
    pub run_name: String,
    pub summary: PerformanceSummary,
    pub charts_data: ChartsData,
    pub trade_details: Vec<TradeDetail>,
    pub strategy_comparison: Vec<StrategyPerformance>,
}

pub struct HtmlReportGenerator {
    template_dir: PathBuf,
    include_charts: bool,
}

impl HtmlReportGenerator {
    pub async fn generate_dashboard(&self, results: &BacktestResults) -> Result<String> {
        let charts_data = self.prepare_charts_data(results)?;
        
        let dashboard = BacktestDashboard {
            run_name: results.run_metadata.name.clone(),
            summary: results.performance_summary.clone(),
            charts_data,
            trade_details: results.trade_details.clone(),
            strategy_comparison: results.strategy_breakdown.clone(),
        };
        
        Ok(dashboard.render()?)
    }
    
    fn prepare_charts_data(&self, results: &BacktestResults) -> Result<ChartsData> {
        // Prepare data for JavaScript charts
        let equity_curve: Vec<(String, f64)> = results.time_series
            .daily_returns
            .iter()
            .scan(100.0, |acc, return_pct| {
                *acc *= 1.0 + return_pct;
                Some((return_pct.date.format("%Y-%m-%d").to_string(), *acc))
            })
            .collect();
            
        let drawdown_series: Vec<(String, f64)> = results.time_series
            .drawdown_series
            .iter()
            .map(|dd| (dd.date.format("%Y-%m-%d").to_string(), dd.drawdown_percent))
            .collect();
            
        Ok(ChartsData {
            equity_curve,
            drawdown_series,
            profit_distribution: self.prepare_profit_distribution(results)?,
            monthly_returns: self.prepare_monthly_returns(results)?,
        })
    }
}
```

### Phase 5: Integration and Optimization (Week 5-6)

#### 5.1 GEPA Integration
```rust
// src/backtesting/optimization/mod.rs
use crate::gepa::{GepaFitness, Genome, FitnessScore};

pub struct BacktestingFitness {
    data_source: Arc<dyn DataSource>,
    simulation_config: SimulationConfig,
    date_range: (DateTime<Utc>, DateTime<Utc>),
    objective_weights: ObjectiveWeights,
}

#[async_trait::async_trait]
impl GepaFitness for BacktestingFitness {
    async fn evaluate(&self, genome: &Genome) -> Result<FitnessScore> {
        // Convert genome to trading configuration
        let trading_config = self.genome_to_config(genome)?;
        
        // Run backtest with the configuration
        let backtest_runner = BacktestRunner::builder()
            .data_source(self.data_source.clone())
            .date_range(self.date_range.0, self.date_range.1)
            .trading_config(trading_config)
            .simulation_config(self.simulation_config.clone())
            .build()?;
            
        let results = backtest_runner.run().await?;
        
        // Calculate multi-objective fitness
        let profit_score = self.calculate_profit_score(&results);
        let risk_score = self.calculate_risk_score(&results);
        let consistency_score = self.calculate_consistency_score(&results);
        let execution_score = self.calculate_execution_score(&results);
        
        let total_score = profit_score * self.objective_weights.profit
            + risk_score * self.objective_weights.risk
            + consistency_score * self.objective_weights.consistency
            + execution_score * self.objective_weights.execution;
            
        Ok(FitnessScore {
            total: total_score,
            components: FitnessComponents {
                profit: profit_score,
                risk: risk_score,
                consistency: consistency_score,
                execution: execution_score,
            },
        })
    }
}
```

#### 5.2 Live Walk-Forward Analysis
```rust
// src/backtesting/live/mod.rs
pub struct LiveBacktester {
    historical_data: Arc<dyn DataSource>,
    live_monitor: Arc<MarketMonitor>,
    simulation_engine: Arc<StrategySimulator>,
    config: LiveBacktestConfig,
    performance_tracker: Arc<Mutex<PerformanceTracker>>,
}

impl LiveBacktester {
    pub async fn start_continuous_validation(&mut self) -> Result<()> {
        let mut interval = tokio::time::interval(self.config.validation_interval);
        
        loop {
            interval.tick().await;
            
            match self.run_validation_cycle().await {
                Ok(validation_results) => {
                    self.process_validation_results(validation_results).await?;
                }
                Err(e) => {
                    log::error!("Validation cycle failed: {}", e);
                    // Continue running despite individual failures
                }
            }
        }
    }
    
    async fn run_validation_cycle(&self) -> Result<ValidationResults> {
        let end_time = Utc::now();
        let start_time = end_time - self.config.validation_window;
        
        // Run backtest on recent historical data
        let backtest_results = self.run_historical_backtest(start_time, end_time).await?;
        
        // Get actual live performance for the same period
        let live_performance = self.get_live_performance(start_time, end_time).await?;
        
        // Compare predictions vs actual results
        let accuracy_metrics = self.calculate_accuracy_metrics(
            &backtest_results,
            &live_performance,
        )?;
        
        Ok(ValidationResults {
            backtest_results,
            live_performance,
            accuracy_metrics,
            validation_timestamp: Utc::now(),
        })
    }
}
```

## Configuration Examples

### Example Backtesting Configuration
```yaml
# config_backtest.yaml
backtesting:
  data_sources:
    - type: csv
      path: "./data/historical/"
      pairs: ["SOL/USDC", "RAY/USDC", "ORCA/USDC"]
    - type: database
      url: "postgresql://localhost/solana_data"
      cache_size: 1000
      
  simulation:
    latency_model: "realistic"
    base_latency_ms: 150
    volatility_impact: 0.3
    latency_jitter: 20.0
    slippage_model: "depth_based"
    success_rate: 0.98
    
  analysis:
    benchmark: "buy_and_hold"
    risk_free_rate: 0.02
    confidence_levels: [0.95, 0.99]
    
  reporting:
    formats: ["html", "json", "csv"]
    include_charts: true
    export_trades: true
    output_directory: "./backtest_results/"
    
  optimization:
    enable_gepa: true
    population_size: 50
    generations: 100
    mutation_rate: 0.1
    walk_forward_months: 3
    optimization_metric: "sharpe_ratio"
    
  live_validation:
    enable: true
    validation_window: "30 days"
    validation_interval: "1 hour"
    alert_threshold: 0.2  # 20% deviation triggers alert
```

### Example Usage Scripts

#### Basic Backtesting Script
```rust
// examples/basic_backtest.rs
use solana_arbitrage_bot::backtesting::*;
use chrono::{Duration, Utc};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    let config = BacktestingConfig::from_file("config_backtest.yaml")?;
    
    let end_date = Utc::now();
    let start_date = end_date - Duration::days(30);
    
    let backtest_runner = BacktestRunner::builder()
        .config(config)
        .date_range(start_date, end_date)
        .strategy("arbitrage")
        .build()?;
        
    println!("Starting backtest for {} to {}", start_date.date(), end_date.date());
    
    let results = backtest_runner.run().await?;
    
    println!("Backtest completed!");
    println!("Total trades: {}", results.total_trades);
    println!("Win rate: {:.2}%", results.win_rate * 100.0);
    println!("Total profit: ${:.2}", results.total_profit_usd);
    println!("Sharpe ratio: {:.2}", results.sharpe_ratio);
    println!("Max drawdown: {:.2}%", results.max_drawdown * 100.0);
    
    // Generate reports
    let html_report = HtmlReportGenerator::new().generate(&results).await?;
    std::fs::write("backtest_report.html", html_report)?;
    
    let json_report = JsonReportGenerator::new().generate(&results).await?;
    std::fs::write("backtest_results.json", json_report)?;
    
    Ok(())
}
```

## Testing Strategy

### Unit Tests
- Data source implementations
- Simulation models (latency, slippage, fees)
- Analytics calculations
- Report generation

### Integration Tests
- End-to-end backtest runs
- GEPA optimization integration
- Live validation workflows
- Performance benchmarks

### Performance Tests
- Large dataset processing
- Memory usage optimization
- Parallel execution scaling
- Report generation speed

## Documentation Requirements

1. **API Documentation**: Complete rustdoc comments for all public APIs
2. **User Guide**: Step-by-step usage examples and tutorials
3. **Configuration Reference**: Comprehensive configuration options
4. **Performance Guide**: Optimization tips and best practices
5. **Troubleshooting Guide**: Common issues and solutions

## Success Criteria

1. **Functionality**:
   - Support for multiple data sources (CSV, DB, RPC, Live)
   - Realistic simulation of trading conditions
   - Comprehensive performance analytics
   - Multiple report formats

2. **Performance**:
   - Process 1M+ data points in under 60 seconds
   - Memory usage under 1GB for typical datasets
   - Parallel execution support
   - Efficient data caching

3. **Integration**:
   - Seamless integration with existing Calculator and Safety modules
   - GEPA optimization compatibility
   - Live validation capabilities
   - Feature flag support for modular compilation

4. **Usability**:
   - Intuitive configuration system
   - Clear error messages and logging
   - Interactive HTML reports
   - Command-line interface

This implementation plan provides a comprehensive roadmap for building a production-ready backtesting engine that integrates seamlessly with the existing Solana Arbitrage Bot architecture while providing powerful analysis and optimization capabilities.