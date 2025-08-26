//! 🧬 GEPA Evolution Engine
//! 
//! Advanced genetic algorithm evolution engine with multiple selection,
//! crossover, and mutation strategies for trading parameter optimization

use std::sync::Arc;
use rand::{Rng, thread_rng};
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use log::{debug, info, warn};

use super::genome::TradingGenome;
use super::population::{Population, Individual};
use super::{ParameterBounds};

/// Evolution engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    /// Mutation rate (0.0 - 1.0)
    pub mutation_rate: f64,
    
    /// Crossover rate (0.0 - 1.0)
    pub crossover_rate: f64,
    
    /// Elite selection percentage (0.0 - 1.0)
    pub elite_percentage: f64,
    
    /// Tournament selection size
    pub tournament_size: usize,
    
    /// Enable adaptive mutation rates
    pub enable_adaptive_mutation: bool,
    
    /// Parameter bounds for mutations
    pub parameter_bounds: ParameterBounds,
    
    /// Population diversity threshold
    pub diversity_threshold: f64,
    
    /// Maximum stagnation generations before increasing mutation
    pub max_stagnation_generations: u32,
    
    /// Adaptive mutation multiplier
    pub adaptive_mutation_multiplier: f64,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            mutation_rate: 0.1,
            crossover_rate: 0.8,
            elite_percentage: 0.2,
            tournament_size: 5,
            enable_adaptive_mutation: true,
            parameter_bounds: ParameterBounds::default(),
            diversity_threshold: 0.01,
            max_stagnation_generations: 10,
            adaptive_mutation_multiplier: 1.5,
        }
    }
}

/// Selection methods for genetic algorithm
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionMethod {
    Tournament,
    RouletteWheel,
    RankSelection,
    StochasticUniversalSampling,
}

/// Crossover methods for genetic algorithm
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CrossoverMethod {
    SinglePoint,
    TwoPoint,
    Uniform,
    BlendAlpha,
    SimulatedBinary,
}

/// Mutation methods for genetic algorithm
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MutationMethod {
    Gaussian,
    Uniform,
    Polynomial,
    Adaptive,
}

/// Evolution statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionStats {
    pub generation: u32,
    pub best_fitness: f64,
    pub average_fitness: f64,
    pub worst_fitness: f64,
    pub population_diversity: f64,
    pub mutation_rate: f64,
    pub stagnation_count: u32,
    pub elite_count: usize,
    pub crossover_count: usize,
    pub mutation_count: usize,
}

/// Main evolution engine for genetic algorithm
pub struct EvolutionEngine {
    config: EvolutionConfig,
    current_generation: u32,
    stagnation_count: u32,
    current_mutation_rate: f64,
    evolution_stats: Vec<EvolutionStats>,
    best_fitness_history: Vec<f64>,
    
    // Selection methods
    selection_method: SelectionMethod,
    crossover_method: CrossoverMethod,
    mutation_method: MutationMethod,
}

impl EvolutionEngine {
    /// Create new evolution engine with configuration
    pub fn new(config: EvolutionConfig) -> Self {
        let current_mutation_rate = config.mutation_rate;
        
        Self {
            config,
            current_generation: 0,
            stagnation_count: 0,
            current_mutation_rate,
            evolution_stats: Vec::new(),
            best_fitness_history: Vec::new(),
            selection_method: SelectionMethod::Tournament,
            crossover_method: CrossoverMethod::BlendAlpha,
            mutation_method: MutationMethod::Adaptive,
        }
    }
    
    /// Create with default configuration
    pub fn with_default_config() -> Self {
        Self::new(EvolutionConfig::default())
    }
    
    /// Evolve population to next generation
    pub async fn evolve(&mut self, population: &Population) -> Result<Population> {
        if population.individuals.is_empty() {
            return Err(anyhow!("Cannot evolve empty population"));
        }
        
        info!("🧬 Evolving generation {} -> {}", self.current_generation, self.current_generation + 1);
        
        // Calculate population statistics
        let stats = self.calculate_population_stats(population);
        
        // Check for stagnation and adapt mutation rate
        self.update_adaptive_parameters(&stats);
        
        // Store statistics
        self.evolution_stats.push(stats.clone());
        self.best_fitness_history.push(stats.best_fitness);
        
        // Create new population
        let mut new_individuals = Vec::new();
        let population_size = population.individuals.len();
        
        // 1. Elite selection - keep best individuals
        let elite_count = (population_size as f64 * self.config.elite_percentage) as usize;
        let mut elite_individuals = self.select_elite(population, elite_count)?;
        new_individuals.append(&mut elite_individuals);
        
        debug!("Selected {} elite individuals", elite_count);
        
        // 2. Generate offspring through crossover and mutation
        let remaining_count = population_size - elite_count;
        let mut offspring = Vec::new();
        
        for i in 0..remaining_count {
            if i % 2 == 0 && i + 1 < remaining_count {
                // Create pair of offspring through crossover
                let parent1 = self.select_parent(population)?;
                let parent2 = self.select_parent(population)?;
                
                let (mut child1, mut child2) = if thread_rng().gen::<f64>() < self.config.crossover_rate {
                    self.crossover(&parent1.genome, &parent2.genome)?
                } else {
                    // No crossover, just copy parents
                    (parent1.genome.clone(), parent2.genome.clone())
                };
                
                // Apply mutation
                if thread_rng().gen::<f64>() < self.current_mutation_rate {
                    self.mutate(&mut child1)?;
                }
                if thread_rng().gen::<f64>() < self.current_mutation_rate {
                    self.mutate(&mut child2)?;
                }
                
                offspring.push(Individual {
                    genome: child1,
                    fitness: None,
                    age: 0,
                });
                
                if offspring.len() < remaining_count {
                    offspring.push(Individual {
                        genome: child2,
                        fitness: None,
                        age: 0,
                    });
                }
            } else if offspring.len() < remaining_count {
                // Single offspring from mutation
                let parent = self.select_parent(population)?;
                let mut child = parent.genome.clone();
                
                if thread_rng().gen::<f64>() < self.current_mutation_rate {
                    self.mutate(&mut child)?;
                }
                
                offspring.push(Individual {
                    genome: child,
                    fitness: None,
                    age: 0,
                });
            }
        }
        
        new_individuals.extend(offspring.into_iter().take(remaining_count));
        
        debug!("Generated {} offspring individuals", remaining_count);
        
        // Increment generation counter
        self.current_generation += 1;
        
        // Update individual ages
        for individual in &mut new_individuals {
            individual.age += 1;
        }
        
        let new_population = Population {
            individuals: new_individuals,
            generation: self.current_generation,
            diversity: 0.0, // Will be calculated when needed
        };
        
        info!("✅ Evolution complete: Generation {}, {} individuals", 
              self.current_generation, new_population.individuals.len());
        
        Ok(new_population)
    }
    
    /// Select elite individuals (best fitness)
    fn select_elite(&self, population: &Population, count: usize) -> Result<Vec<Individual>> {
        let mut individuals_with_fitness: Vec<_> = population.individuals.iter()
            .filter(|ind| ind.fitness.is_some())
            .collect();
        
        if individuals_with_fitness.is_empty() {
            return Err(anyhow!("No individuals with fitness scores available for elite selection"));
        }
        
        // Sort by fitness (descending)
        individuals_with_fitness.sort_by(|a, b| {
            let fitness_a = a.fitness.as_ref().unwrap().overall_score;
            let fitness_b = b.fitness.as_ref().unwrap().overall_score;
            fitness_b.partial_cmp(&fitness_a).unwrap()
        });
        
        let elite_count = count.min(individuals_with_fitness.len());
        let elite = individuals_with_fitness.into_iter()
            .take(elite_count)
            .map(|ind| ind.clone())
            .collect();
        
        Ok(elite)
    }
    
    /// Select parent for reproduction using tournament selection
    fn select_parent(&self, population: &Population) -> Result<&Individual> {
        match self.selection_method {
            SelectionMethod::Tournament => self.tournament_selection(population),
            SelectionMethod::RouletteWheel => self.roulette_wheel_selection(population),
            SelectionMethod::RankSelection => self.rank_selection(population),
            SelectionMethod::StochasticUniversalSampling => self.sus_selection(population),
        }
    }
    
    /// Tournament selection
    fn tournament_selection(&self, population: &Population) -> Result<&Individual> {
        let individuals_with_fitness: Vec<_> = population.individuals.iter()
            .filter(|ind| ind.fitness.is_some())
            .collect();
        
        if individuals_with_fitness.is_empty() {
            return Err(anyhow!("No individuals with fitness for tournament selection"));
        }
        
        let tournament_size = self.config.tournament_size.min(individuals_with_fitness.len());
        let mut rng = thread_rng();
        let mut best = None;
        let mut best_fitness = f64::NEG_INFINITY;
        
        for _ in 0..tournament_size {
            let idx = rng.gen_range(0..individuals_with_fitness.len());
            let individual = individuals_with_fitness[idx];
            let fitness = individual.fitness.as_ref().unwrap().overall_score;
            
            if fitness > best_fitness {
                best_fitness = fitness;
                best = Some(individual);
            }
        }
        
        best.ok_or_else(|| anyhow!("Tournament selection failed"))
    }
    
    /// Roulette wheel selection
    fn roulette_wheel_selection(&self, population: &Population) -> Result<&Individual> {
        let individuals_with_fitness: Vec<_> = population.individuals.iter()
            .filter(|ind| ind.fitness.is_some())
            .collect();
        
        if individuals_with_fitness.is_empty() {
            return Err(anyhow!("No individuals with fitness for roulette selection"));
        }
        
        // Calculate total fitness (ensure all positive)
        let min_fitness = individuals_with_fitness.iter()
            .map(|ind| ind.fitness.as_ref().unwrap().overall_score)
            .fold(f64::INFINITY, f64::min);
        
        let offset = if min_fitness < 0.0 { -min_fitness + 0.01 } else { 0.0 };
        
        let total_fitness: f64 = individuals_with_fitness.iter()
            .map(|ind| ind.fitness.as_ref().unwrap().overall_score + offset)
            .sum();
        
        if total_fitness <= 0.0 {
            // Fallback to random selection
            let idx = thread_rng().gen_range(0..individuals_with_fitness.len());
            return Ok(individuals_with_fitness[idx]);
        }
        
        let mut rng = thread_rng();
        let target = rng.gen::<f64>() * total_fitness;
        let mut current = 0.0;
        
        for individual in &individuals_with_fitness {
            current += individual.fitness.as_ref().unwrap().overall_score + offset;
            if current >= target {
                return Ok(individual);
            }
        }
        
        // Fallback
        Ok(individuals_with_fitness[individuals_with_fitness.len() - 1])
    }
    
    /// Rank selection
    fn rank_selection(&self, population: &Population) -> Result<&Individual> {
        let mut individuals_with_fitness: Vec<_> = population.individuals.iter()
            .filter(|ind| ind.fitness.is_some())
            .collect();
        
        if individuals_with_fitness.is_empty() {
            return Err(anyhow!("No individuals with fitness for rank selection"));
        }
        
        // Sort by fitness
        individuals_with_fitness.sort_by(|a, b| {
            let fitness_a = a.fitness.as_ref().unwrap().overall_score;
            let fitness_b = b.fitness.as_ref().unwrap().overall_score;
            fitness_a.partial_cmp(&fitness_b).unwrap()
        });
        
        let n = individuals_with_fitness.len() as f64;
        let total_rank: f64 = (n * (n + 1.0)) / 2.0;
        let target = thread_rng().gen::<f64>() * total_rank;
        
        let mut current = 0.0;
        for (i, individual) in individuals_with_fitness.iter().enumerate() {
            current += (i + 1) as f64;
            if current >= target {
                return Ok(individual);
            }
        }
        
        Ok(individuals_with_fitness[individuals_with_fitness.len() - 1])
    }
    
    /// Stochastic Universal Sampling
    fn sus_selection(&self, population: &Population) -> Result<&Individual> {
        // Simplified implementation - use tournament for now
        self.tournament_selection(population)
    }
    
    /// Perform crossover between two genomes
    fn crossover(&self, parent1: &TradingGenome, parent2: &TradingGenome) -> Result<(TradingGenome, TradingGenome)> {
        match self.crossover_method {
            CrossoverMethod::SinglePoint => self.single_point_crossover(parent1, parent2),
            CrossoverMethod::TwoPoint => self.two_point_crossover(parent1, parent2),
            CrossoverMethod::Uniform => self.uniform_crossover(parent1, parent2),
            CrossoverMethod::BlendAlpha => self.blend_alpha_crossover(parent1, parent2),
            CrossoverMethod::SimulatedBinary => self.sbx_crossover(parent1, parent2),
        }
    }
    
    /// Single point crossover
    fn single_point_crossover(&self, parent1: &TradingGenome, parent2: &TradingGenome) -> Result<(TradingGenome, TradingGenome)> {
        let params1 = parent1.get_all_parameters();
        let params2 = parent2.get_all_parameters();
        
        if params1.len() != params2.len() {
            return Err(anyhow!("Parent genomes have different parameter counts"));
        }
        
        let crossover_point = thread_rng().gen_range(1..params1.len());
        let mut child1_params = params1.clone();
        let mut child2_params = params2.clone();
        
        // Swap parameters after crossover point
        for (key, value) in params2.iter().skip(crossover_point) {
            child1_params.insert(key.clone(), *value);
        }
        for (key, value) in params1.iter().skip(crossover_point) {
            child2_params.insert(key.clone(), *value);
        }
        
        let child1 = TradingGenome::from_parameters(child1_params)?;
        let child2 = TradingGenome::from_parameters(child2_params)?;
        
        Ok((child1, child2))
    }
    
    /// Two point crossover
    fn two_point_crossover(&self, parent1: &TradingGenome, parent2: &TradingGenome) -> Result<(TradingGenome, TradingGenome)> {
        let params1 = parent1.get_all_parameters();
        let params2 = parent2.get_all_parameters();
        
        if params1.len() < 2 {
            return self.single_point_crossover(parent1, parent2);
        }
        
        let mut rng = thread_rng();
        let point1 = rng.gen_range(1..params1.len());
        let point2 = rng.gen_range(point1 + 1..=params1.len());
        
        let mut child1_params = params1.clone();
        let mut child2_params = params2.clone();
        
        // Get parameter keys as vec for indexed access
        let keys: Vec<_> = params1.keys().collect();
        
        // Swap parameters between crossover points
        for i in point1..point2 {
            if let Some(key) = keys.get(i) {
                if let Some(&value2) = params2.get(*key) {
                    child1_params.insert((*key).clone(), value2);
                }
                if let Some(&value1) = params1.get(*key) {
                    child2_params.insert((*key).clone(), value1);
                }
            }
        }
        
        let child1 = TradingGenome::from_parameters(child1_params)?;
        let child2 = TradingGenome::from_parameters(child2_params)?;
        
        Ok((child1, child2))
    }
    
    /// Uniform crossover
    fn uniform_crossover(&self, parent1: &TradingGenome, parent2: &TradingGenome) -> Result<(TradingGenome, TradingGenome)> {
        let params1 = parent1.get_all_parameters();
        let params2 = parent2.get_all_parameters();
        
        let mut child1_params = params1.clone();
        let mut child2_params = params2.clone();
        let mut rng = thread_rng();
        
        for key in params1.keys() {
            if rng.gen::<f64>() < 0.5 {
                // Swap this parameter
                if let (Some(&value1), Some(&value2)) = (params1.get(key), params2.get(key)) {
                    child1_params.insert(key.clone(), value2);
                    child2_params.insert(key.clone(), value1);
                }
            }
        }
        
        let child1 = TradingGenome::from_parameters(child1_params)?;
        let child2 = TradingGenome::from_parameters(child2_params)?;
        
        Ok((child1, child2))
    }
    
    /// Blend Alpha crossover (BLX-α)
    fn blend_alpha_crossover(&self, parent1: &TradingGenome, parent2: &TradingGenome) -> Result<(TradingGenome, TradingGenome)> {
        let params1 = parent1.get_all_parameters();
        let params2 = parent2.get_all_parameters();
        let alpha = 0.5; // Blend factor
        
        let mut child1_params = params1.clone();
        let mut child2_params = params2.clone();
        let mut rng = thread_rng();
        
        for key in params1.keys() {
            if let (Some(&value1), Some(&value2)) = (params1.get(key), params2.get(key)) {
                let min_val = value1.min(value2);
                let max_val = value1.max(value2);
                let range = max_val - min_val;
                let extension = range * alpha;
                
                let lower_bound = min_val - extension;
                let upper_bound = max_val + extension;
                
                let child1_value = rng.gen_range(lower_bound..=upper_bound);
                let child2_value = rng.gen_range(lower_bound..=upper_bound);
                
                child1_params.insert(key.clone(), child1_value);
                child2_params.insert(key.clone(), child2_value);
            }
        }
        
        let child1 = TradingGenome::from_parameters(child1_params)?;
        let child2 = TradingGenome::from_parameters(child2_params)?;
        
        Ok((child1, child2))
    }
    
    /// Simulated Binary Crossover (SBX)
    fn sbx_crossover(&self, parent1: &TradingGenome, parent2: &TradingGenome) -> Result<(TradingGenome, TradingGenome)> {
        // Simplified implementation - use blend alpha for now
        self.blend_alpha_crossover(parent1, parent2)
    }
    
    /// Perform mutation on a genome
    fn mutate(&self, genome: &mut TradingGenome) -> Result<()> {
        match self.mutation_method {
            MutationMethod::Gaussian => self.gaussian_mutation(genome),
            MutationMethod::Uniform => self.uniform_mutation(genome),
            MutationMethod::Polynomial => self.polynomial_mutation(genome),
            MutationMethod::Adaptive => self.adaptive_mutation(genome),
        }
    }
    
    /// Gaussian mutation
    fn gaussian_mutation(&self, genome: &mut TradingGenome) -> Result<()> {
        let mut rng = thread_rng();
        let sigma = 0.1; // Standard deviation for Gaussian mutation
        
        for (key, value) in genome.get_all_parameters() {
            let noise = rng.gen::<f64>() * sigma - (sigma / 2.0);
            let mutated_value = value + noise;
            
            // Apply bounds checking
            let bounded_value = self.apply_parameter_bounds(key, mutated_value);
            genome.set_parameter(key, bounded_value)?;
        }
        
        Ok(())
    }
    
    /// Uniform mutation
    fn uniform_mutation(&self, genome: &mut TradingGenome) -> Result<()> {
        let mut rng = thread_rng();
        let mutation_strength = 0.2; // How much of the parameter range to use
        
        for (key, value) in genome.get_all_parameters() {
            let (min_bound, max_bound) = self.get_parameter_bounds(key);
            let range = max_bound - min_bound;
            let mutation_range = range * mutation_strength;
            
            let noise = rng.gen::<f64>() * mutation_range - (mutation_range / 2.0);
            let mutated_value = value + noise;
            
            let bounded_value = self.apply_parameter_bounds(key, mutated_value);
            genome.set_parameter(key, bounded_value)?;
        }
        
        Ok(())
    }
    
    /// Polynomial mutation
    fn polynomial_mutation(&self, genome: &mut TradingGenome) -> Result<()> {
        // Simplified - use Gaussian for now
        self.gaussian_mutation(genome)
    }
    
    /// Adaptive mutation based on population diversity
    fn adaptive_mutation(&self, genome: &mut TradingGenome) -> Result<()> {
        // Use current adaptive mutation rate
        let base_strength = self.current_mutation_rate;
        let mut rng = thread_rng();
        
        for (key, value) in genome.get_all_parameters() {
            let (min_bound, max_bound) = self.get_parameter_bounds(key);
            let range = max_bound - min_bound;
            let mutation_strength = range * base_strength;
            
            let noise = rng.gen::<f64>() * mutation_strength - (mutation_strength / 2.0);
            let mutated_value = value + noise;
            
            let bounded_value = self.apply_parameter_bounds(key, mutated_value);
            genome.set_parameter(key, bounded_value)?;
        }
        
        Ok(())
    }
    
    /// Get parameter bounds
    fn get_parameter_bounds(&self, parameter_name: &str) -> (f64, f64) {
        match parameter_name {
            "position_size" => (self.config.parameter_bounds.position_size_min, self.config.parameter_bounds.position_size_max),
            "profit_threshold" => (self.config.parameter_bounds.profit_threshold_min, self.config.parameter_bounds.profit_threshold_max),
            "slippage_tolerance" => (self.config.parameter_bounds.slippage_tolerance_min, self.config.parameter_bounds.slippage_tolerance_max),
            "stop_loss" => (self.config.parameter_bounds.stop_loss_min, self.config.parameter_bounds.stop_loss_max),
            _ => (0.0, 1.0), // Default bounds
        }
    }
    
    /// Apply bounds to parameter value
    fn apply_parameter_bounds(&self, parameter_name: &str, value: f64) -> f64 {
        let (min_bound, max_bound) = self.get_parameter_bounds(parameter_name);
        value.max(min_bound).min(max_bound)
    }
    
    /// Calculate population statistics
    fn calculate_population_stats(&self, population: &Population) -> EvolutionStats {
        let individuals_with_fitness: Vec<_> = population.individuals.iter()
            .filter(|ind| ind.fitness.is_some())
            .collect();
        
        if individuals_with_fitness.is_empty() {
            return EvolutionStats {
                generation: self.current_generation,
                best_fitness: 0.0,
                average_fitness: 0.0,
                worst_fitness: 0.0,
                population_diversity: 0.0,
                mutation_rate: self.current_mutation_rate,
                stagnation_count: self.stagnation_count,
                elite_count: 0,
                crossover_count: 0,
                mutation_count: 0,
            };
        }
        
        let fitness_values: Vec<f64> = individuals_with_fitness.iter()
            .map(|ind| ind.fitness.as_ref().unwrap().overall_score)
            .collect();
        
        let best_fitness = fitness_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let worst_fitness = fitness_values.iter().copied().fold(f64::INFINITY, f64::min);
        let average_fitness = fitness_values.iter().sum::<f64>() / fitness_values.len() as f64;
        
        // Calculate population diversity (simplified)
        let diversity = self.calculate_diversity(population);
        
        EvolutionStats {
            generation: self.current_generation,
            best_fitness,
            average_fitness,
            worst_fitness,
            population_diversity: diversity,
            mutation_rate: self.current_mutation_rate,
            stagnation_count: self.stagnation_count,
            elite_count: (population.individuals.len() as f64 * self.config.elite_percentage) as usize,
            crossover_count: 0, // Would track during evolution
            mutation_count: 0,  // Would track during evolution
        }
    }
    
    /// Calculate population diversity
    fn calculate_diversity(&self, population: &Population) -> f64 {
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
    
    /// Update adaptive parameters based on evolution statistics
    fn update_adaptive_parameters(&mut self, stats: &EvolutionStats) {
        if !self.config.enable_adaptive_mutation {
            return;
        }
        
        // Check for stagnation
        if let Some(&last_best) = self.best_fitness_history.last() {
            if (stats.best_fitness - last_best).abs() < 0.001 {
                self.stagnation_count += 1;
            } else {
                self.stagnation_count = 0;
            }
        }
        
        // Adapt mutation rate based on stagnation and diversity
        if self.stagnation_count >= self.config.max_stagnation_generations || stats.population_diversity < self.config.diversity_threshold {
            self.current_mutation_rate = (self.config.mutation_rate * self.config.adaptive_mutation_multiplier).min(0.5);
            debug!("Increased mutation rate to {:.3} due to stagnation/low diversity", self.current_mutation_rate);
        } else {
            // Gradually decrease mutation rate
            self.current_mutation_rate = (self.current_mutation_rate * 0.95).max(self.config.mutation_rate);
        }
    }
    
    /// Get evolution statistics
    pub fn get_evolution_stats(&self) -> &Vec<EvolutionStats> {
        &self.evolution_stats
    }
    
    /// Get current generation
    pub fn get_current_generation(&self) -> u32 {
        self.current_generation
    }
    
    /// Reset evolution state
    pub fn reset(&mut self) {
        self.current_generation = 0;
        self.stagnation_count = 0;
        self.current_mutation_rate = self.config.mutation_rate;
        self.evolution_stats.clear();
        self.best_fitness_history.clear();
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: EvolutionConfig) {
        self.config = config;
        self.current_mutation_rate = config.mutation_rate;
    }
}
