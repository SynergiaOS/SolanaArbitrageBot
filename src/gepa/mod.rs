//! 🧬 GEPA - Genetic Evolution Parameter Adaptation
//! 
//! Advanced genetic algorithm system for optimizing trading parameters
//! to achieve 20-50% improvement in risk-adjusted returns

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use rand::{Rng, thread_rng};
use anyhow::{Result, anyhow};
use log::{info, warn, error, debug};

pub mod genome;
pub mod population;
pub mod fitness;
pub mod evolution;
pub mod optimizer;

use genome::{TradingGenome, ParameterGene};
use population::Population;
use fitness::{FitnessEvaluator, FitnessMetrics};
use evolution::{EvolutionEngine, EvolutionConfig};

/// GEPA optimizer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GepaConfig {
    /// Population size for genetic algorithm
    pub population_size: usize,
    
    /// Number of generations to evolve
    pub max_generations: u32,
    
    /// Mutation rate (0.0 - 1.0)
    pub mutation_rate: f64,
    
    /// Crossover rate (0.0 - 1.0)
    pub crossover_rate: f64,
    
    /// Elite selection percentage
    pub elite_percentage: f64,
    
    /// Tournament selection size
    pub tournament_size: usize,
    
    /// Fitness evaluation period (hours)
    pub evaluation_period_hours: u64,
    
    /// Minimum trades required for fitness evaluation
    pub min_trades_for_evaluation: u32,
    
    /// Enable adaptive mutation rates
    pub enable_adaptive_mutation: bool,
    
    /// Enable multi-objective optimization
    pub enable_multi_objective: bool,
    
    /// Parameter bounds and constraints
    pub parameter_bounds: ParameterBounds,
    
    /// Fitness weights
    pub fitness_weights: FitnessWeights,
}

/// Parameter bounds for genetic optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterBounds {
    /// Position sizing bounds (percentage of capital)
    pub position_size_min: f64,
    pub position_size_max: f64,
    
    /// Profit threshold bounds (percentage)
    pub profit_threshold_min: f64,
    pub profit_threshold_max: f64,
    
    /// Slippage tolerance bounds (percentage)
    pub slippage_tolerance_min: f64,
    pub slippage_tolerance_max: f64,
    
    /// Timing window bounds (milliseconds)
    pub timing_window_min: u64,
    pub timing_window_max: u64,
    
    /// Stop loss bounds (percentage)
    pub stop_loss_min: f64,
    pub stop_loss_max: f64,
    
    /// Take profit levels bounds
    pub take_profit_levels_min: usize,
    pub take_profit_levels_max: usize,
}

/// Fitness function weights
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
}

impl Default for GepaConfig {
    fn default() -> Self {
        Self {
            population_size: 50,
            max_generations: 100,
            mutation_rate: 0.1,
            crossover_rate: 0.8,
            elite_percentage: 0.2,
            tournament_size: 5,
            evaluation_period_hours: 24,
            min_trades_for_evaluation: 10,
            enable_adaptive_mutation: true,
            enable_multi_objective: true,
            parameter_bounds: ParameterBounds::default(),
            fitness_weights: FitnessWeights::default(),
        }
    }
}

impl Default for ParameterBounds {
    fn default() -> Self {
        Self {
            position_size_min: 5.0,   // 5% minimum
            position_size_max: 25.0,  // 25% maximum
            profit_threshold_min: 0.1, // 0.1% minimum
            profit_threshold_max: 2.0,  // 2.0% maximum
            slippage_tolerance_min: 0.1, // 0.1% minimum
            slippage_tolerance_max: 1.0,  // 1.0% maximum
            timing_window_min: 50,    // 50ms minimum
            timing_window_max: 500,   // 500ms maximum
            stop_loss_min: -10.0,     // -10% minimum
            stop_loss_max: -1.0,      // -1% maximum
            take_profit_levels_min: 2,
            take_profit_levels_max: 6,
        }
    }
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
        }
    }
}

/// GEPA optimization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    /// Best genome found
    pub best_genome: TradingGenome,
    
    /// Best fitness achieved
    pub best_fitness: FitnessMetrics,
    
    /// Generation when best was found
    pub best_generation: u32,
    
    /// Total generations evolved
    pub total_generations: u32,
    
    /// Optimization duration
    pub optimization_duration: Duration,
    
    /// Improvement over baseline
    pub improvement_percent: f64,
    
    /// Convergence metrics
    pub convergence_metrics: ConvergenceMetrics,
    
    /// Population diversity at end
    pub final_diversity: f64,
    
    /// Optimization timestamp
    pub timestamp: SystemTime,
}

/// Convergence tracking metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceMetrics {
    /// Fitness improvement per generation
    pub fitness_progression: Vec<f64>,
    
    /// Population diversity over time
    pub diversity_progression: Vec<f64>,
    
    /// Stagnation counter
    pub stagnation_generations: u32,
    
    /// Convergence achieved
    pub converged: bool,
    
    /// Convergence generation
    pub convergence_generation: Option<u32>,
}

/// Main GEPA optimizer
pub struct GepaOptimizer {
    config: GepaConfig,
    
    /// Evolution engine
    evolution_engine: EvolutionEngine,
    
    /// Fitness evaluator
    fitness_evaluator: FitnessEvaluator,
    
    /// Current population
    population: Arc<RwLock<Population>>,
    
    /// Optimization history
    optimization_history: Arc<RwLock<Vec<OptimizationResult>>>,
    
    /// Current best parameters
    current_best: Arc<RwLock<Option<TradingGenome>>>,
    
    /// Performance tracking
    baseline_performance: Arc<RwLock<Option<FitnessMetrics>>>,
    
    /// Optimization state
    is_optimizing: Arc<RwLock<bool>>,
    current_generation: Arc<RwLock<u32>>,
}

impl GepaOptimizer {
    /// Create new GEPA optimizer
    pub fn new(config: GepaConfig) -> Self {
        let evolution_config = EvolutionConfig {
            mutation_rate: config.mutation_rate,
            crossover_rate: config.crossover_rate,
            elite_percentage: config.elite_percentage,
            tournament_size: config.tournament_size,
            enable_adaptive_mutation: config.enable_adaptive_mutation,
            parameter_bounds: config.parameter_bounds.clone(),
        };
        
        let evolution_engine = EvolutionEngine::new(evolution_config);
        let fitness_evaluator = FitnessEvaluator::new(config.fitness_weights.clone());
        
        // Initialize population
        let population = Population::new(config.population_size, &config.parameter_bounds);
        
        Self {
            config,
            evolution_engine,
            fitness_evaluator,
            population: Arc::new(RwLock::new(population)),
            optimization_history: Arc::new(RwLock::new(Vec::new())),
            current_best: Arc::new(RwLock::new(None)),
            baseline_performance: Arc::new(RwLock::new(None)),
            is_optimizing: Arc::new(RwLock::new(false)),
            current_generation: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Start optimization process
    pub async fn start_optimization(&self) -> Result<()> {
        if *self.is_optimizing.read().await {
            return Err(anyhow!("Optimization already in progress"));
        }
        
        *self.is_optimizing.write().await = true;
        *self.current_generation.write().await = 0;
        
        info!("🧬 Starting GEPA optimization with {} individuals over {} generations", 
              self.config.population_size, self.config.max_generations);
        
        let optimization_start = SystemTime::now();
        let mut convergence_metrics = ConvergenceMetrics {
            fitness_progression: Vec::new(),
            diversity_progression: Vec::new(),
            stagnation_generations: 0,
            converged: false,
            convergence_generation: None,
        };
        
        let mut best_fitness = 0.0;
        let mut best_genome = None;
        let mut best_generation = 0;
        
        // Evolution loop
        for generation in 0..self.config.max_generations {
            *self.current_generation.write().await = generation;
            
            info!("🧬 Generation {}/{}", generation + 1, self.config.max_generations);
            
            // Evaluate fitness for current population
            let fitness_results = self.evaluate_population_fitness().await?;
            
            // Update population with fitness scores
            {
                let mut population = self.population.write().await;
                for (i, fitness) in fitness_results.iter().enumerate() {
                    if let Some(individual) = population.individuals.get_mut(i) {
                        individual.fitness = Some(fitness.clone());
                    }
                }
            }
            
            // Find best individual in current generation
            let generation_best = fitness_results.iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.overall_score.partial_cmp(&b.overall_score).unwrap())
                .map(|(i, fitness)| (i, fitness.clone()));
            
            if let Some((best_index, fitness)) = generation_best {
                if fitness.overall_score > best_fitness {
                    best_fitness = fitness.overall_score;
                    best_genome = Some(self.population.read().await.individuals[best_index].genome.clone());
                    best_generation = generation;
                    convergence_metrics.stagnation_generations = 0;
                } else {
                    convergence_metrics.stagnation_generations += 1;
                }
                
                convergence_metrics.fitness_progression.push(fitness.overall_score);
            }
            
            // Calculate population diversity
            let diversity = self.calculate_population_diversity().await;
            convergence_metrics.diversity_progression.push(diversity);
            
            // Check for convergence
            if convergence_metrics.stagnation_generations >= 20 || diversity < 0.01 {
                convergence_metrics.converged = true;
                convergence_metrics.convergence_generation = Some(generation);
                info!("🎯 Convergence achieved at generation {}", generation + 1);
                break;
            }
            
            // Evolve population for next generation
            if generation < self.config.max_generations - 1 {
                self.evolve_population().await?;
            }
            
            // Log progress
            if generation % 10 == 0 {
                info!("Generation {}: Best fitness = {:.4}, Diversity = {:.4}", 
                      generation + 1, best_fitness, diversity);
            }
        }
        
        // Calculate improvement over baseline
        let improvement_percent = if let Some(baseline) = &*self.baseline_performance.read().await {
            ((best_fitness - baseline.overall_score) / baseline.overall_score) * 100.0
        } else {
            0.0
        };
        
        // Create optimization result
        let result = OptimizationResult {
            best_genome: best_genome.clone().unwrap_or_default(),
            best_fitness: FitnessMetrics {
                overall_score: best_fitness,
                sharpe_ratio: 0.0, // Would be filled from actual evaluation
                total_return: 0.0,
                max_drawdown: 0.0,
                win_rate: 0.0,
                profit_factor: 0.0,
                trade_frequency: 0.0,
                risk_adjusted_return: 0.0,
            },
            best_generation,
            total_generations: *self.current_generation.read().await + 1,
            optimization_duration: optimization_start.elapsed().unwrap_or(Duration::ZERO),
            improvement_percent,
            convergence_metrics,
            final_diversity: self.calculate_population_diversity().await,
            timestamp: SystemTime::now(),
        };
        
        // Update current best
        if let Some(genome) = best_genome {
            *self.current_best.write().await = Some(genome);
        }
        
        // Store result
        self.optimization_history.write().await.push(result.clone());
        
        *self.is_optimizing.write().await = false;
        
        info!("🎉 GEPA optimization completed! Improvement: {:.2}%", improvement_percent);
        
        Ok(())
    }
    
    /// Evaluate fitness for entire population
    async fn evaluate_population_fitness(&self) -> Result<Vec<FitnessMetrics>> {
        let population = self.population.read().await;
        let mut fitness_results = Vec::new();
        
        for individual in &population.individuals {
            // This would involve running backtests or live trading with the genome's parameters
            // For now, we'll simulate fitness evaluation
            let fitness = self.fitness_evaluator.evaluate_genome(&individual.genome).await?;
            fitness_results.push(fitness);
        }
        
        Ok(fitness_results)
    }
    
    /// Evolve population to next generation
    async fn evolve_population(&self) -> Result<()> {
        let current_population = self.population.read().await.clone();
        let new_population = self.evolution_engine.evolve(&current_population).await?;
        *self.population.write().await = new_population;
        Ok(())
    }
    
    /// Calculate population diversity
    async fn calculate_population_diversity(&self) -> f64 {
        let population = self.population.read().await;
        
        if population.individuals.len() < 2 {
            return 0.0;
        }
        
        let mut total_distance = 0.0;
        let mut comparisons = 0;
        
        for i in 0..population.individuals.len() {
            for j in i + 1..population.individuals.len() {
                let distance = population.individuals[i].genome.distance(&population.individuals[j].genome);
                total_distance += distance;
                comparisons += 1;
            }
        }
        
        if comparisons > 0 {
            total_distance / comparisons as f64
        } else {
            0.0
        }
    }
    
    /// Get current best parameters
    pub async fn get_current_best(&self) -> Option<TradingGenome> {
        self.current_best.read().await.clone()
    }
    
    /// Get optimization history
    pub async fn get_optimization_history(&self) -> Vec<OptimizationResult> {
        self.optimization_history.read().await.clone()
    }
    
    /// Set baseline performance for comparison
    pub async fn set_baseline_performance(&self, baseline: FitnessMetrics) {
        *self.baseline_performance.write().await = Some(baseline);
    }
    
    /// Check if optimization is running
    pub async fn is_optimizing(&self) -> bool {
        *self.is_optimizing.read().await
    }
    
    /// Get current generation
    pub async fn get_current_generation(&self) -> u32 {
        *self.current_generation.read().await
    }
}
