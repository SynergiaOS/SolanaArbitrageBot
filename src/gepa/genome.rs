//! 🧬 Trading Genome - Genetic Representation of Trading Parameters
//! 
//! Defines the genetic structure for trading parameter optimization

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use rand::{Rng, thread_rng};
use rust_decimal::Decimal;

use super::ParameterBounds;

/// Trading genome containing all optimizable parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingGenome {
    /// Position sizing parameters
    pub position_sizing: PositionSizingGenes,
    
    /// Profit threshold parameters
    pub profit_thresholds: ProfitThresholdGenes,
    
    /// Risk management parameters
    pub risk_management: RiskManagementGenes,
    
    /// Timing parameters
    pub timing: TimingGenes,
    
    /// Execution parameters
    pub execution: ExecutionGenes,
    
    /// Sniper-specific parameters
    pub sniper: SniperGenes,
    
    /// Arbitrage-specific parameters
    pub arbitrage: ArbitrageGenes,
}

/// Position sizing genetic parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSizingGenes {
    /// Base position size (percentage of capital)
    pub base_position_percent: ParameterGene<f64>,
    
    /// Maximum position size (percentage of capital)
    pub max_position_percent: ParameterGene<f64>,
    
    /// Risk-based position scaling factor
    pub risk_scaling_factor: ParameterGene<f64>,
    
    /// Volatility-based position adjustment
    pub volatility_adjustment: ParameterGene<f64>,
    
    /// Kelly criterion weight
    pub kelly_weight: ParameterGene<f64>,
}

/// Profit threshold genetic parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitThresholdGenes {
    /// Minimum profit threshold (percentage)
    pub min_profit_percent: ParameterGene<f64>,
    
    /// Dynamic profit scaling
    pub dynamic_scaling: ParameterGene<bool>,
    
    /// Profit scaling factor based on volatility
    pub volatility_scaling: ParameterGene<f64>,
    
    /// Market condition adjustment
    pub market_condition_factor: ParameterGene<f64>,
    
    /// Time-based profit adjustment
    pub time_decay_factor: ParameterGene<f64>,
}

/// Risk management genetic parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskManagementGenes {
    /// Stop loss percentage
    pub stop_loss_percent: ParameterGene<f64>,
    
    /// Maximum daily loss percentage
    pub max_daily_loss_percent: ParameterGene<f64>,
    
    /// Maximum consecutive losses
    pub max_consecutive_losses: ParameterGene<u32>,
    
    /// Drawdown threshold for position reduction
    pub drawdown_threshold: ParameterGene<f64>,
    
    /// Risk-off mode trigger
    pub risk_off_trigger: ParameterGene<f64>,
    
    /// Recovery mode parameters
    pub recovery_factor: ParameterGene<f64>,
}

/// Timing genetic parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingGenes {
    /// Execution timeout (milliseconds)
    pub execution_timeout_ms: ParameterGene<u64>,
    
    /// Price update frequency (milliseconds)
    pub price_update_frequency_ms: ParameterGene<u64>,
    
    /// Opportunity window (milliseconds)
    pub opportunity_window_ms: ParameterGene<u64>,
    
    /// Market hours preference
    pub market_hours_weight: ParameterGene<f64>,
    
    /// Volatility timing factor
    pub volatility_timing: ParameterGene<f64>,
}

/// Execution genetic parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionGenes {
    /// Slippage tolerance (percentage)
    pub slippage_tolerance: ParameterGene<f64>,
    
    /// Priority fee multiplier
    pub priority_fee_multiplier: ParameterGene<f64>,
    
    /// Retry attempts
    pub max_retries: ParameterGene<u32>,
    
    /// Parallel execution preference
    pub parallel_execution: ParameterGene<bool>,
    
    /// Preflight simulation preference
    pub skip_preflight: ParameterGene<bool>,
}

/// Sniper-specific genetic parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniperGenes {
    /// Risk tolerance for new tokens
    pub risk_tolerance: ParameterGene<f64>,
    
    /// Minimum liquidity requirement
    pub min_liquidity_multiplier: ParameterGene<f64>,
    
    /// Rug detection sensitivity
    pub rug_detection_sensitivity: ParameterGene<f64>,
    
    /// Profit taking aggressiveness
    pub profit_taking_aggressiveness: ParameterGene<f64>,
    
    /// Maximum hold time multiplier
    pub max_hold_time_multiplier: ParameterGene<f64>,
}

/// Arbitrage-specific genetic parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageGenes {
    /// Minimum spread threshold
    pub min_spread_multiplier: ParameterGene<f64>,
    
    /// Speed vs accuracy trade-off
    pub speed_accuracy_balance: ParameterGene<f64>,
    
    /// Cross-DEX preference weights
    pub dex_preference_weights: HashMap<String, ParameterGene<f64>>,
    
    /// Liquidity depth consideration
    pub liquidity_depth_weight: ParameterGene<f64>,
    
    /// Gas price sensitivity
    pub gas_price_sensitivity: ParameterGene<f64>,
}

/// Generic parameter gene with bounds and mutation properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterGene<T> {
    /// Current value
    pub value: T,
    
    /// Minimum allowed value
    pub min_value: T,
    
    /// Maximum allowed value
    pub max_value: T,
    
    /// Mutation rate for this parameter
    pub mutation_rate: f64,
    
    /// Mutation strength (standard deviation for continuous values)
    pub mutation_strength: f64,
    
    /// Whether this parameter is enabled for optimization
    pub enabled: bool,
}

impl<T> ParameterGene<T>
where
    T: Clone + PartialOrd,
{
    /// Create new parameter gene
    pub fn new(value: T, min_value: T, max_value: T) -> Self {
        Self {
            value,
            min_value,
            max_value,
            mutation_rate: 0.1,
            mutation_strength: 0.1,
            enabled: true,
        }
    }
    
    /// Create disabled parameter gene
    pub fn disabled(value: T, min_value: T, max_value: T) -> Self {
        Self {
            value,
            min_value,
            max_value,
            mutation_rate: 0.0,
            mutation_strength: 0.0,
            enabled: false,
        }
    }
}

impl ParameterGene<f64> {
    /// Mutate continuous parameter
    pub fn mutate(&mut self, global_mutation_rate: f64) {
        if !self.enabled || thread_rng().gen::<f64>() > self.mutation_rate * global_mutation_rate {
            return;
        }
        
        let mut rng = thread_rng();
        let mutation = rng.gen_range(-self.mutation_strength..=self.mutation_strength);
        let range = self.max_value - self.min_value;
        let new_value = self.value + mutation * range;
        
        self.value = new_value.clamp(self.min_value, self.max_value);
    }
    
    /// Crossover with another parameter
    pub fn crossover(&self, other: &Self, alpha: f64) -> Self {
        let new_value = self.value * alpha + other.value * (1.0 - alpha);
        
        Self {
            value: new_value.clamp(self.min_value.min(other.min_value), self.max_value.max(other.max_value)),
            min_value: self.min_value.min(other.min_value),
            max_value: self.max_value.max(other.max_value),
            mutation_rate: (self.mutation_rate + other.mutation_rate) / 2.0,
            mutation_strength: (self.mutation_strength + other.mutation_strength) / 2.0,
            enabled: self.enabled && other.enabled,
        }
    }
}

impl ParameterGene<u32> {
    /// Mutate discrete parameter
    pub fn mutate(&mut self, global_mutation_rate: f64) {
        if !self.enabled || thread_rng().gen::<f64>() > self.mutation_rate * global_mutation_rate {
            return;
        }
        
        let mut rng = thread_rng();
        let range = (self.max_value - self.min_value) as f64;
        let mutation_amount = (rng.gen::<f64>() - 0.5) * self.mutation_strength * range;
        let new_value = (self.value as f64 + mutation_amount).round() as u32;
        
        self.value = new_value.clamp(self.min_value, self.max_value);
    }
    
    /// Crossover with another parameter
    pub fn crossover(&self, other: &Self, alpha: f64) -> Self {
        let new_value = if thread_rng().gen::<f64>() < alpha {
            self.value
        } else {
            other.value
        };
        
        Self {
            value: new_value,
            min_value: self.min_value.min(other.min_value),
            max_value: self.max_value.max(other.max_value),
            mutation_rate: (self.mutation_rate + other.mutation_rate) / 2.0,
            mutation_strength: (self.mutation_strength + other.mutation_strength) / 2.0,
            enabled: self.enabled && other.enabled,
        }
    }
}

impl ParameterGene<bool> {
    /// Mutate boolean parameter
    pub fn mutate(&mut self, global_mutation_rate: f64) {
        if !self.enabled || thread_rng().gen::<f64>() > self.mutation_rate * global_mutation_rate {
            return;
        }
        
        self.value = !self.value;
    }
    
    /// Crossover with another parameter
    pub fn crossover(&self, other: &Self, alpha: f64) -> Self {
        let new_value = if thread_rng().gen::<f64>() < alpha {
            self.value
        } else {
            other.value
        };
        
        Self {
            value: new_value,
            min_value: self.min_value,
            max_value: self.max_value,
            mutation_rate: (self.mutation_rate + other.mutation_rate) / 2.0,
            mutation_strength: (self.mutation_strength + other.mutation_strength) / 2.0,
            enabled: self.enabled && other.enabled,
        }
    }
}

impl TradingGenome {
    /// Create random genome within bounds
    pub fn random(bounds: &ParameterBounds) -> Self {
        let mut rng = thread_rng();
        
        Self {
            position_sizing: PositionSizingGenes {
                base_position_percent: ParameterGene::new(
                    rng.gen_range(bounds.position_size_min..=bounds.position_size_max),
                    bounds.position_size_min,
                    bounds.position_size_max,
                ),
                max_position_percent: ParameterGene::new(
                    rng.gen_range(bounds.position_size_min..=bounds.position_size_max),
                    bounds.position_size_min,
                    bounds.position_size_max,
                ),
                risk_scaling_factor: ParameterGene::new(
                    rng.gen_range(0.5..=2.0),
                    0.5,
                    2.0,
                ),
                volatility_adjustment: ParameterGene::new(
                    rng.gen_range(0.1..=1.0),
                    0.1,
                    1.0,
                ),
                kelly_weight: ParameterGene::new(
                    rng.gen_range(0.0..=1.0),
                    0.0,
                    1.0,
                ),
            },
            profit_thresholds: ProfitThresholdGenes {
                min_profit_percent: ParameterGene::new(
                    rng.gen_range(bounds.profit_threshold_min..=bounds.profit_threshold_max),
                    bounds.profit_threshold_min,
                    bounds.profit_threshold_max,
                ),
                dynamic_scaling: ParameterGene::new(
                    rng.gen_bool(0.5),
                    false,
                    true,
                ),
                volatility_scaling: ParameterGene::new(
                    rng.gen_range(0.5..=2.0),
                    0.5,
                    2.0,
                ),
                market_condition_factor: ParameterGene::new(
                    rng.gen_range(0.8..=1.2),
                    0.8,
                    1.2,
                ),
                time_decay_factor: ParameterGene::new(
                    rng.gen_range(0.9..=1.1),
                    0.9,
                    1.1,
                ),
            },
            risk_management: RiskManagementGenes {
                stop_loss_percent: ParameterGene::new(
                    rng.gen_range(bounds.stop_loss_min..=bounds.stop_loss_max),
                    bounds.stop_loss_min,
                    bounds.stop_loss_max,
                ),
                max_daily_loss_percent: ParameterGene::new(
                    rng.gen_range(5.0..=20.0),
                    5.0,
                    20.0,
                ),
                max_consecutive_losses: ParameterGene::new(
                    rng.gen_range(3..=10),
                    3,
                    10,
                ),
                drawdown_threshold: ParameterGene::new(
                    rng.gen_range(10.0..=30.0),
                    10.0,
                    30.0,
                ),
                risk_off_trigger: ParameterGene::new(
                    rng.gen_range(15.0..=40.0),
                    15.0,
                    40.0,
                ),
                recovery_factor: ParameterGene::new(
                    rng.gen_range(0.5..=1.5),
                    0.5,
                    1.5,
                ),
            },
            timing: TimingGenes {
                execution_timeout_ms: ParameterGene::new(
                    rng.gen_range(bounds.timing_window_min..=bounds.timing_window_max),
                    bounds.timing_window_min,
                    bounds.timing_window_max,
                ),
                price_update_frequency_ms: ParameterGene::new(
                    rng.gen_range(100..=5000),
                    100,
                    5000,
                ),
                opportunity_window_ms: ParameterGene::new(
                    rng.gen_range(1000..=30000),
                    1000,
                    30000,
                ),
                market_hours_weight: ParameterGene::new(
                    rng.gen_range(0.5..=2.0),
                    0.5,
                    2.0,
                ),
                volatility_timing: ParameterGene::new(
                    rng.gen_range(0.1..=1.0),
                    0.1,
                    1.0,
                ),
            },
            execution: ExecutionGenes {
                slippage_tolerance: ParameterGene::new(
                    rng.gen_range(bounds.slippage_tolerance_min..=bounds.slippage_tolerance_max),
                    bounds.slippage_tolerance_min,
                    bounds.slippage_tolerance_max,
                ),
                priority_fee_multiplier: ParameterGene::new(
                    rng.gen_range(1.0..=5.0),
                    1.0,
                    5.0,
                ),
                max_retries: ParameterGene::new(
                    rng.gen_range(1..=5),
                    1,
                    5,
                ),
                parallel_execution: ParameterGene::new(
                    rng.gen_bool(0.7),
                    false,
                    true,
                ),
                skip_preflight: ParameterGene::new(
                    rng.gen_bool(0.8),
                    false,
                    true,
                ),
            },
            sniper: SniperGenes {
                risk_tolerance: ParameterGene::new(
                    rng.gen_range(0.3..=0.8),
                    0.3,
                    0.8,
                ),
                min_liquidity_multiplier: ParameterGene::new(
                    rng.gen_range(0.5..=3.0),
                    0.5,
                    3.0,
                ),
                rug_detection_sensitivity: ParameterGene::new(
                    rng.gen_range(0.6..=0.95),
                    0.6,
                    0.95,
                ),
                profit_taking_aggressiveness: ParameterGene::new(
                    rng.gen_range(0.3..=0.9),
                    0.3,
                    0.9,
                ),
                max_hold_time_multiplier: ParameterGene::new(
                    rng.gen_range(0.5..=2.0),
                    0.5,
                    2.0,
                ),
            },
            arbitrage: ArbitrageGenes {
                min_spread_multiplier: ParameterGene::new(
                    rng.gen_range(0.5..=2.0),
                    0.5,
                    2.0,
                ),
                speed_accuracy_balance: ParameterGene::new(
                    rng.gen_range(0.3..=0.9),
                    0.3,
                    0.9,
                ),
                dex_preference_weights: {
                    let mut weights = HashMap::new();
                    weights.insert("Raydium".to_string(), ParameterGene::new(
                        rng.gen_range(0.3..=1.0), 0.3, 1.0
                    ));
                    weights.insert("Orca".to_string(), ParameterGene::new(
                        rng.gen_range(0.3..=1.0), 0.3, 1.0
                    ));
                    weights
                },
                liquidity_depth_weight: ParameterGene::new(
                    rng.gen_range(0.1..=0.8),
                    0.1,
                    0.8,
                ),
                gas_price_sensitivity: ParameterGene::new(
                    rng.gen_range(0.5..=2.0),
                    0.5,
                    2.0,
                ),
            },
        }
    }
    
    /// Mutate genome
    pub fn mutate(&mut self, mutation_rate: f64) {
        // Mutate all parameter genes
        self.position_sizing.base_position_percent.mutate(mutation_rate);
        self.position_sizing.max_position_percent.mutate(mutation_rate);
        self.position_sizing.risk_scaling_factor.mutate(mutation_rate);
        self.position_sizing.volatility_adjustment.mutate(mutation_rate);
        self.position_sizing.kelly_weight.mutate(mutation_rate);
        
        self.profit_thresholds.min_profit_percent.mutate(mutation_rate);
        self.profit_thresholds.dynamic_scaling.mutate(mutation_rate);
        self.profit_thresholds.volatility_scaling.mutate(mutation_rate);
        self.profit_thresholds.market_condition_factor.mutate(mutation_rate);
        self.profit_thresholds.time_decay_factor.mutate(mutation_rate);
        
        // Continue for all other parameter groups...
    }
    
    /// Crossover with another genome
    pub fn crossover(&self, other: &Self, alpha: f64) -> Self {
        Self {
            position_sizing: PositionSizingGenes {
                base_position_percent: self.position_sizing.base_position_percent.crossover(
                    &other.position_sizing.base_position_percent, alpha
                ),
                max_position_percent: self.position_sizing.max_position_percent.crossover(
                    &other.position_sizing.max_position_percent, alpha
                ),
                risk_scaling_factor: self.position_sizing.risk_scaling_factor.crossover(
                    &other.position_sizing.risk_scaling_factor, alpha
                ),
                volatility_adjustment: self.position_sizing.volatility_adjustment.crossover(
                    &other.position_sizing.volatility_adjustment, alpha
                ),
                kelly_weight: self.position_sizing.kelly_weight.crossover(
                    &other.position_sizing.kelly_weight, alpha
                ),
            },
            // Continue crossover for all other parameter groups...
            profit_thresholds: self.profit_thresholds.clone(), // Simplified for brevity
            risk_management: self.risk_management.clone(),
            timing: self.timing.clone(),
            execution: self.execution.clone(),
            sniper: self.sniper.clone(),
            arbitrage: self.arbitrage.clone(),
        }
    }
    
    /// Calculate distance between genomes (for diversity measurement)
    pub fn distance(&self, other: &Self) -> f64 {
        let mut total_distance = 0.0;
        let mut parameter_count = 0;
        
        // Calculate normalized distances for all parameters
        total_distance += (self.position_sizing.base_position_percent.value - 
                         other.position_sizing.base_position_percent.value).abs();
        parameter_count += 1;
        
        // Add distances for all other parameters...
        
        if parameter_count > 0 {
            total_distance / parameter_count as f64
        } else {
            0.0
        }
    }
}

impl Default for TradingGenome {
    fn default() -> Self {
        Self::random(&ParameterBounds::default())
    }
}
