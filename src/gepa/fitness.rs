//! 🧬 GEPA Fitness Evaluation System
//! 
//! Advanced fitness evaluation for genetic algorithm optimization
//! of trading parameters with multi-objective optimization support

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use anyhow::{Result, anyhow};
use log::{debug, info, warn};

use super::genome::TradingGenome;

/// Comprehensive fitness metrics for trading strategy evaluation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FitnessMetrics {
    /// Overall fitness score (0.0 - 1.0)
    pub overall_score: f64,
    
    /// Sharpe ratio (risk-adjusted return)
    pub sharpe_ratio: f64,
    
    /// Total return percentage
    pub total_return: f64,
    
    /// Maximum drawdown percentage (negative)
    pub max_drawdown: f64,
    
    /// Win rate (0.0 - 1.0)
    pub win_rate: f64,
    
    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,
    
    /// Trade frequency (trades per day)
    pub trade_frequency: f64,
    
    /// Risk-adjusted return
    pub risk_adjusted_return: f64,
    
    /// Additional metrics
    pub volatility: f64,
    pub sortino_ratio: f64,
    pub calmar_ratio: f64,
    pub recovery_factor: f64,
    
    /// Trade statistics
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub average_trade_duration_minutes: f64,
    
    /// Risk metrics
    pub value_at_risk_95: f64,
    pub expected_shortfall: f64,
    pub beta: f64,
    pub alpha: f64,
    
    /// Evaluation metadata
    pub evaluation_period_days: u32,
    pub evaluation_timestamp: SystemTime,
    pub market_conditions: MarketCondition,
}

/// Market condition assessment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MarketCondition {
    Bull,
    Bear,
    Sideways,
    HighVolatility,
    LowVolatility,
    Unknown,
}

/// Fitness weights for multi-objective optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessWeights {
    /// Sharpe ratio weight
    pub sharpe_ratio: f64,
    
    /// Total return weight
    pub total_return: f64,
    
    /// Maximum drawdown weight (negative)
    pub max_drawdown: f64,
    
    /// Win rate weight
    pub win_rate: f64,
    
    /// Profit factor weight
    pub profit_factor: f64,
    
    /// Trade frequency weight
    pub trade_frequency: f64,
    
    /// Risk-adjusted return weight
    pub risk_adjusted_return: f64,
    
    /// Volatility penalty weight
    pub volatility_penalty: f64,
}

impl Default for FitnessWeights {
    fn default() -> Self {
        Self {
            sharpe_ratio: 0.3,
            total_return: 0.25,
            max_drawdown: -0.2, // Negative because lower is better
            win_rate: 0.15,
            profit_factor: 0.1,
            trade_frequency: 0.05,
            risk_adjusted_return: 0.35,
            volatility_penalty: -0.1,
        }
    }
}

/// Historical performance data for fitness evaluation
#[derive(Debug, Clone)]
pub struct PerformanceData {
    pub returns: Vec<f64>,
    pub timestamps: Vec<SystemTime>,
    pub trade_results: Vec<TradeResult>,
    pub portfolio_values: Vec<f64>,
}

/// Individual trade result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeResult {
    pub profit_loss: f64,
    pub return_percentage: f64,
    pub duration_minutes: f64,
    pub timestamp: SystemTime,
    pub was_profitable: bool,
    pub slippage: f64,
    pub fees: f64,
}

/// Fitness evaluator with configurable weights and methods
pub struct FitnessEvaluator {
    weights: FitnessWeights,
    benchmark_performance: Option<FitnessMetrics>,
    evaluation_cache: Arc<std::sync::RwLock<HashMap<String, FitnessMetrics>>>,
}

impl FitnessEvaluator {
    /// Create new fitness evaluator with custom weights
    pub fn new(weights: FitnessWeights) -> Self {
        Self {
            weights,
            benchmark_performance: None,
            evaluation_cache: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }
    
    /// Create with default weights
    pub fn with_default_weights() -> Self {
        Self::new(FitnessWeights::default())
    }
    
    /// Set benchmark performance for relative evaluation
    pub fn set_benchmark(&mut self, benchmark: FitnessMetrics) {
        self.benchmark_performance = Some(benchmark);
    }
    
    /// Evaluate fitness of a trading genome
    pub async fn evaluate_genome(&self, genome: &TradingGenome) -> Result<FitnessMetrics> {
        // Generate cache key from genome parameters
        let cache_key = self.generate_cache_key(genome);
        
        // Check cache first
        if let Ok(cache) = self.evaluation_cache.read() {
            if let Some(cached_metrics) = cache.get(&cache_key) {
                debug!("Using cached fitness evaluation for genome");
                return Ok(cached_metrics.clone());
            }
        }
        
        info!("Evaluating fitness for new genome parameters");
        
        // Simulate backtesting or use historical data
        let performance_data = self.simulate_trading_performance(genome).await?;
        
        // Calculate comprehensive metrics
        let metrics = self.calculate_fitness_metrics(&performance_data, genome)?;
        
        // Cache the result
        if let Ok(mut cache) = self.evaluation_cache.write() {
            cache.insert(cache_key, metrics.clone());
        }
        
        Ok(metrics)
    }
    
    /// Simulate trading performance for a given genome
    async fn simulate_trading_performance(&self, genome: &TradingGenome) -> Result<PerformanceData> {
        // This would normally run backtests or analyze historical data
        // For now, we'll generate realistic-looking simulated data
        
        let num_trades = 50 + (genome.get_parameter("aggressiveness").unwrap_or(0.5) * 100.0) as usize;
        let mut trade_results = Vec::new();
        let mut returns = Vec::new();
        let mut portfolio_values = Vec::new();
        let mut timestamps = Vec::new();
        
        let mut portfolio_value = 10000.0; // Starting with $10k
        let start_time = SystemTime::now() - Duration::from_secs(30 * 24 * 3600); // 30 days ago
        
        for i in 0..num_trades {
            let time_offset = Duration::from_secs((i as u64) * 3600); // 1 hour between trades
            let timestamp = start_time + time_offset;
            
            // Simulate trade based on genome parameters
            let profit_threshold = genome.get_parameter("profit_threshold").unwrap_or(0.02);
            let position_size = genome.get_parameter("position_size").unwrap_or(0.1);
            let stop_loss = genome.get_parameter("stop_loss").unwrap_or(-0.05);
            
            // Generate trade result with some randomness but influenced by parameters
            let random_factor = (i as f64 * 0.1).sin() * 0.02; // Deterministic "randomness"
            let base_return = if random_factor > 0.0 {
                profit_threshold * (0.5 + random_factor)
            } else {
                stop_loss * (0.5 - random_factor.abs())
            };
            
            let trade_return = base_return * position_size;
            let trade_profit = portfolio_value * trade_return;
            
            portfolio_value += trade_profit;
            
            let trade_result = TradeResult {
                profit_loss: trade_profit,
                return_percentage: trade_return * 100.0,
                duration_minutes: 15.0 + (random_factor.abs() * 60.0),
                timestamp,
                was_profitable: trade_profit > 0.0,
                slippage: 0.001 * random_factor.abs(),
                fees: portfolio_value * 0.0001, // 0.01% fee
            };
            
            trade_results.push(trade_result);
            returns.push(trade_return);
            portfolio_values.push(portfolio_value);
            timestamps.push(timestamp);
        }
        
        Ok(PerformanceData {
            returns,
            timestamps,
            trade_results,
            portfolio_values,
        })
    }
    
    /// Calculate comprehensive fitness metrics from performance data
    fn calculate_fitness_metrics(&self, data: &PerformanceData, genome: &TradingGenome) -> Result<FitnessMetrics> {
        if data.returns.is_empty() {
            return Err(anyhow!("No performance data available for fitness calculation"));
        }
        
        // Basic statistics
        let total_return = ((data.portfolio_values.last().unwrap() / data.portfolio_values.first().unwrap()) - 1.0) * 100.0;
        let returns_mean = data.returns.iter().sum::<f64>() / data.returns.len() as f64;
        let returns_std = self.calculate_standard_deviation(&data.returns, returns_mean);
        
        // Sharpe ratio (annualized)
        let risk_free_rate = 0.02 / 365.0; // 2% annual risk-free rate
        let sharpe_ratio = if returns_std > 0.0 {
            (returns_mean - risk_free_rate) / returns_std * (365.0_f64).sqrt()
        } else {
            0.0
        };
        
        // Maximum drawdown
        let max_drawdown = self.calculate_max_drawdown(&data.portfolio_values);
        
        // Win rate
        let winning_trades = data.trade_results.iter().filter(|t| t.was_profitable).count() as u32;
        let total_trades = data.trade_results.len() as u32;
        let win_rate = if total_trades > 0 {
            winning_trades as f64 / total_trades as f64
        } else {
            0.0
        };
        
        // Profit factor
        let gross_profit: f64 = data.trade_results.iter()
            .filter(|t| t.was_profitable)
            .map(|t| t.profit_loss)
            .sum();
        let gross_loss: f64 = data.trade_results.iter()
            .filter(|t| !t.was_profitable)
            .map(|t| t.profit_loss.abs())
            .sum();
        let profit_factor = if gross_loss > 0.0 { gross_profit / gross_loss } else { 0.0 };
        
        // Trade frequency (trades per day)
        let evaluation_days = 30.0; // Assuming 30-day evaluation period
        let trade_frequency = total_trades as f64 / evaluation_days;
        
        // Risk-adjusted return (Calmar ratio)
        let risk_adjusted_return = if max_drawdown.abs() > 0.01 {
            total_return / max_drawdown.abs()
        } else {
            total_return
        };
        
        // Additional risk metrics
        let volatility = returns_std * (365.0_f64).sqrt() * 100.0; // Annualized volatility %
        let sortino_ratio = self.calculate_sortino_ratio(&data.returns);
        let calmar_ratio = if max_drawdown.abs() > 0.01 {
            (total_return / 100.0) / max_drawdown.abs()
        } else {
            0.0
        };
        
        // Calculate overall fitness score using weights
        let overall_score = self.calculate_weighted_score(
            sharpe_ratio,
            total_return,
            max_drawdown,
            win_rate,
            profit_factor,
            trade_frequency,
            risk_adjusted_return,
            volatility,
        );
        
        let metrics = FitnessMetrics {
            overall_score,
            sharpe_ratio,
            total_return,
            max_drawdown,
            win_rate,
            profit_factor,
            trade_frequency,
            risk_adjusted_return,
            volatility,
            sortino_ratio,
            calmar_ratio,
            recovery_factor: if max_drawdown.abs() > 0.01 { total_return / max_drawdown.abs() } else { 0.0 },
            total_trades,
            winning_trades,
            losing_trades: total_trades - winning_trades,
            average_trade_duration_minutes: data.trade_results.iter()
                .map(|t| t.duration_minutes)
                .sum::<f64>() / total_trades as f64,
            value_at_risk_95: self.calculate_var_95(&data.returns),
            expected_shortfall: 0.0, // Simplified
            beta: 1.0, // Simplified - would need market data
            alpha: sharpe_ratio * 0.1, // Simplified approximation
            evaluation_period_days: evaluation_days as u32,
            evaluation_timestamp: SystemTime::now(),
            market_conditions: MarketCondition::Unknown, // Would analyze market data
        };
        
        Ok(metrics)
    }
    
    /// Calculate weighted overall fitness score
    fn calculate_weighted_score(
        &self,
        sharpe_ratio: f64,
        total_return: f64,
        max_drawdown: f64,
        win_rate: f64,
        profit_factor: f64,
        trade_frequency: f64,
        risk_adjusted_return: f64,
        volatility: f64,
    ) -> f64 {
        // Normalize metrics to 0-1 scale
        let norm_sharpe = (sharpe_ratio / 3.0).min(1.0).max(0.0); // Good Sharpe is 1-3
        let norm_return = (total_return / 100.0).min(1.0).max(0.0); // 100% return = 1.0
        let norm_drawdown = (max_drawdown.abs() / 50.0).min(1.0).max(0.0); // 50% DD = 1.0 penalty
        let norm_win_rate = win_rate;
        let norm_profit_factor = (profit_factor / 3.0).min(1.0).max(0.0); // Good PF is 1.5-3
        let norm_trade_freq = (trade_frequency / 10.0).min(1.0).max(0.0); // 10 trades/day = 1.0
        let norm_risk_adj = (risk_adjusted_return / 10.0).min(1.0).max(0.0); // 10 = very good
        let norm_volatility = (volatility / 100.0).min(1.0).max(0.0); // 100% vol = 1.0 penalty
        
        let score = 
            self.weights.sharpe_ratio * norm_sharpe +
            self.weights.total_return * norm_return +
            self.weights.max_drawdown * norm_drawdown + // Negative weight
            self.weights.win_rate * norm_win_rate +
            self.weights.profit_factor * norm_profit_factor +
            self.weights.trade_frequency * norm_trade_freq +
            self.weights.risk_adjusted_return * norm_risk_adj +
            self.weights.volatility_penalty * norm_volatility; // Negative weight
        
        // Ensure score is between 0 and 1
        score.min(1.0).max(0.0)
    }
    
    /// Calculate standard deviation
    fn calculate_standard_deviation(&self, values: &[f64], mean: f64) -> f64 {
        if values.len() <= 1 {
            return 0.0;
        }
        
        let variance = values.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / (values.len() - 1) as f64;
        
        variance.sqrt()
    }
    
    /// Calculate maximum drawdown
    fn calculate_max_drawdown(&self, portfolio_values: &[f64]) -> f64 {
        if portfolio_values.is_empty() {
            return 0.0;
        }
        
        let mut max_value = portfolio_values[0];
        let mut max_drawdown = 0.0;
        
        for &value in portfolio_values {
            if value > max_value {
                max_value = value;
            }
            
            let drawdown = (value - max_value) / max_value;
            if drawdown < max_drawdown {
                max_drawdown = drawdown;
            }
        }
        
        max_drawdown * 100.0 // Convert to percentage
    }
    
    /// Calculate Sortino ratio (downside deviation)
    fn calculate_sortino_ratio(&self, returns: &[f64]) -> f64 {
        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        let target_return = 0.0; // Use 0% as target
        
        let downside_returns: Vec<f64> = returns.iter()
            .filter(|&&r| r < target_return)
            .map(|&r| (r - target_return).powi(2))
            .collect();
        
        if downside_returns.is_empty() {
            return 0.0;
        }
        
        let downside_deviation = (downside_returns.iter().sum::<f64>() / downside_returns.len() as f64).sqrt();
        
        if downside_deviation > 0.0 {
            (mean_return - target_return) / downside_deviation
        } else {
            0.0
        }
    }
    
    /// Calculate Value at Risk (95% confidence)
    fn calculate_var_95(&self, returns: &[f64]) -> f64 {
        if returns.is_empty() {
            return 0.0;
        }
        
        let mut sorted_returns = returns.to_vec();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let index = ((sorted_returns.len() as f64) * 0.05) as usize;
        if index < sorted_returns.len() {
            sorted_returns[index] * 100.0 // Convert to percentage
        } else {
            0.0
        }
    }
    
    /// Generate cache key from genome parameters
    fn generate_cache_key(&self, genome: &TradingGenome) -> String {
        // Simple hash of key parameters
        format!("fitness_{:?}", genome.get_all_parameters())
    }
    
    /// Clear evaluation cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.evaluation_cache.write() {
            cache.clear();
        }
    }
    
    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize) {
        if let Ok(cache) = self.evaluation_cache.read() {
            (cache.len(), cache.capacity())
        } else {
            (0, 0)
        }
    }
}

impl Default for FitnessMetrics {
    fn default() -> Self {
        Self {
            overall_score: 0.0,
            sharpe_ratio: 0.0,
            total_return: 0.0,
            max_drawdown: 0.0,
            win_rate: 0.0,
            profit_factor: 0.0,
            trade_frequency: 0.0,
            risk_adjusted_return: 0.0,
            volatility: 0.0,
            sortino_ratio: 0.0,
            calmar_ratio: 0.0,
            recovery_factor: 0.0,
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            average_trade_duration_minutes: 0.0,
            value_at_risk_95: 0.0,
            expected_shortfall: 0.0,
            beta: 1.0,
            alpha: 0.0,
            evaluation_period_days: 30,
            evaluation_timestamp: SystemTime::now(),
            market_conditions: MarketCondition::Unknown,
        }
    }
}
