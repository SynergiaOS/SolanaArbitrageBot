//! 🧬 GEPA Population Management
//! 
//! Population and individual management for genetic algorithm
//! optimization with diversity tracking and statistics

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use rand::{Rng, thread_rng};
use anyhow::{Result, anyhow};
use log::{debug, info};

use super::genome::TradingGenome;
use super::fitness::FitnessMetrics;
use super::ParameterBounds;

/// Individual in the genetic algorithm population
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Individual {
    /// Genome containing trading parameters
    pub genome: TradingGenome,
    
    /// Fitness evaluation result
    pub fitness: Option<FitnessMetrics>,
    
    /// Age of individual (generations survived)
    pub age: u32,
    
    /// Parent lineage for tracking
    pub parent_ids: Vec<String>,
    
    /// Unique identifier
    pub id: String,
    
    /// Creation timestamp
    pub created_generation: u32,
    
    /// Performance history
    pub performance_history: Vec<f64>,
    
    /// Survival count (number of selections)
    pub survival_count: u32,
}

impl Individual {
    /// Create new individual with random genome
    pub fn new_random(bounds: &ParameterBounds, generation: u32) -> Result<Self> {
        let genome = TradingGenome::new_random(bounds)?;
        let id = Self::generate_id();
        
        Ok(Self {
            genome,
            fitness: None,
            age: 0,
            parent_ids: Vec::new(),
            id,
            created_generation: generation,
            performance_history: Vec::new(),
            survival_count: 0,
        })
    }
    
    /// Create individual from existing genome
    pub fn from_genome(genome: TradingGenome, generation: u32) -> Self {
        let id = Self::generate_id();
        
        Self {
            genome,
            fitness: None,
            age: 0,
            parent_ids: Vec::new(),
            id,
            created_generation: generation,
            performance_history: Vec::new(),
            survival_count: 0,
        }
    }
    
    /// Create child individual from parents
    pub fn from_parents(genome: TradingGenome, parent1: &Individual, parent2: &Individual, generation: u32) -> Self {
        let id = Self::generate_id();
        let parent_ids = vec![parent1.id.clone(), parent2.id.clone()];
        
        Self {
            genome,
            fitness: None,
            age: 0,
            parent_ids,
            id,
            created_generation: generation,
            performance_history: Vec::new(),
            survival_count: 0,
        }
    }
    
    /// Generate unique ID for individual
    fn generate_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let random: u32 = thread_rng().gen();
        format!("ind_{}_{}", timestamp, random)
    }
    
    /// Check if individual has been evaluated
    pub fn is_evaluated(&self) -> bool {
        self.fitness.is_some()
    }
    
    /// Get fitness score
    pub fn get_fitness_score(&self) -> f64 {
        self.fitness.as_ref().map(|f| f.overall_score).unwrap_or(0.0)
    }
    
    /// Update performance history
    pub fn add_performance(&mut self, score: f64) {
        self.performance_history.push(score);
        // Keep only last 10 records
        if self.performance_history.len() > 10 {
            self.performance_history.remove(0);
        }
    }
    
    /// Get average historical performance
    pub fn get_average_performance(&self) -> f64 {
        if self.performance_history.is_empty() {
            0.0
        } else {
            self.performance_history.iter().sum::<f64>() / self.performance_history.len() as f64
        }
    }
    
    /// Age the individual
    pub fn age_up(&mut self) {
        self.age += 1;
    }
    
    /// Mark as surviving selection
    pub fn survived_selection(&mut self) {
        self.survival_count += 1;
    }
}

/// Population statistics for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationStats {
    pub size: usize,
    pub evaluated_count: usize,
    pub average_fitness: f64,
    pub best_fitness: f64,
    pub worst_fitness: f64,
    pub fitness_std_deviation: f64,
    pub average_age: f64,
    pub diversity_score: f64,
    pub generation: u32,
}

/// Genetic algorithm population
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Population {
    /// All individuals in the population
    pub individuals: Vec<Individual>,
    
    /// Current generation number
    pub generation: u32,
    
    /// Population diversity measure
    pub diversity: f64,
    
    /// Population capacity (target size)
    pub capacity: usize,
    
    /// Parameter bounds for the population
    pub parameter_bounds: ParameterBounds,
    
    /// Population statistics
    pub stats: Option<PopulationStats>,
    
    /// Hall of fame (best individuals ever)
    pub hall_of_fame: Vec<Individual>,
    
    /// Hall of fame size
    pub hall_of_fame_size: usize,
}

impl Population {
    /// Create new population with specified size and parameter bounds
    pub fn new(size: usize, bounds: &ParameterBounds) -> Self {
        let mut population = Self {
            individuals: Vec::new(),
            generation: 0,
            diversity: 0.0,
            capacity: size,
            parameter_bounds: bounds.clone(),
            stats: None,
            hall_of_fame: Vec::new(),
            hall_of_fame_size: 10,
        };
        
        // Initialize with random individuals
        population.initialize_random().unwrap_or_else(|e| {
            log::error!("Failed to initialize population: {}", e);
        });
        
        population
    }
    
    /// Create empty population
    pub fn empty(bounds: &ParameterBounds) -> Self {
        Self {
            individuals: Vec::new(),
            generation: 0,
            diversity: 0.0,
            capacity: 0,
            parameter_bounds: bounds.clone(),
            stats: None,
            hall_of_fame: Vec::new(),
            hall_of_fame_size: 10,
        }
    }
    
    /// Initialize population with random individuals
    pub fn initialize_random(&mut self) -> Result<()> {
        info!("Initializing population with {} random individuals", self.capacity);
        
        self.individuals.clear();
        
        for i in 0..self.capacity {
            match Individual::new_random(&self.parameter_bounds, self.generation) {
                Ok(individual) => {
                    self.individuals.push(individual);
                }
                Err(e) => {
                    log::warn!("Failed to create individual {}: {}", i, e);
                }
            }
        }
        
        debug!("Created {} individuals", self.individuals.len());
        Ok(())
    }
    
    /// Add individual to population
    pub fn add_individual(&mut self, individual: Individual) {
        self.individuals.push(individual);
        
        // Maintain capacity limit
        if self.individuals.len() > self.capacity && self.capacity > 0 {
            // Remove worst individual
            if let Some(worst_index) = self.find_worst_individual_index() {
                self.individuals.remove(worst_index);
            }
        }
    }
    
    /// Remove individual by index
    pub fn remove_individual(&mut self, index: usize) -> Option<Individual> {
        if index < self.individuals.len() {
            Some(self.individuals.remove(index))
        } else {
            None
        }
    }
    
    /// Find best individual
    pub fn get_best_individual(&self) -> Option<&Individual> {
        self.individuals
            .iter()
            .filter(|ind| ind.is_evaluated())
            .max_by(|a, b| a.get_fitness_score().partial_cmp(&b.get_fitness_score()).unwrap())
    }
    
    /// Find worst individual
    pub fn get_worst_individual(&self) -> Option<&Individual> {
        self.individuals
            .iter()
            .filter(|ind| ind.is_evaluated())
            .min_by(|a, b| a.get_fitness_score().partial_cmp(&b.get_fitness_score()).unwrap())
    }
    
    /// Find worst individual index
    fn find_worst_individual_index(&self) -> Option<usize> {
        let mut worst_index = None;
        let mut worst_fitness = f64::INFINITY;
        
        for (i, individual) in self.individuals.iter().enumerate() {
            if individual.is_evaluated() {
                let fitness = individual.get_fitness_score();
                if fitness < worst_fitness {
                    worst_fitness = fitness;
                    worst_index = Some(i);
                }
            }
        }
        
        worst_index
    }
    
    /// Get all evaluated individuals
    pub fn get_evaluated_individuals(&self) -> Vec<&Individual> {
        self.individuals
            .iter()
            .filter(|ind| ind.is_evaluated())
            .collect()
    }
    
    /// Get unevaluated individuals
    pub fn get_unevaluated_individuals(&self) -> Vec<&Individual> {
        self.individuals
            .iter()
            .filter(|ind| !ind.is_evaluated())
            .collect()
    }
    
    /// Calculate population statistics
    pub fn calculate_stats(&mut self) {
        let evaluated: Vec<&Individual> = self.get_evaluated_individuals();
        
        if evaluated.is_empty() {
            self.stats = Some(PopulationStats {
                size: self.individuals.len(),
                evaluated_count: 0,
                average_fitness: 0.0,
                best_fitness: 0.0,
                worst_fitness: 0.0,
                fitness_std_deviation: 0.0,
                average_age: 0.0,
                diversity_score: 0.0,
                generation: self.generation,
            });
            return;
        }
        
        let fitness_scores: Vec<f64> = evaluated.iter().map(|ind| ind.get_fitness_score()).collect();
        
        let best_fitness = fitness_scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let worst_fitness = fitness_scores.iter().copied().fold(f64::INFINITY, f64::min);
        let average_fitness = fitness_scores.iter().sum::<f64>() / fitness_scores.len() as f64;
        
        // Calculate standard deviation
        let variance = fitness_scores.iter()
            .map(|score| (score - average_fitness).powi(2))
            .sum::<f64>() / fitness_scores.len() as f64;
        let fitness_std_deviation = variance.sqrt();
        
        // Calculate average age
        let average_age = self.individuals.iter()
            .map(|ind| ind.age as f64)
            .sum::<f64>() / self.individuals.len() as f64;
        
        // Calculate diversity
        self.diversity = self.calculate_diversity();
        
        self.stats = Some(PopulationStats {
            size: self.individuals.len(),
            evaluated_count: evaluated.len(),
            average_fitness,
            best_fitness,
            worst_fitness,
            fitness_std_deviation,
            average_age,
            diversity_score: self.diversity,
            generation: self.generation,
        });
        
        // Update hall of fame
        self.update_hall_of_fame();
    }
    
    /// Calculate population diversity
    pub fn calculate_diversity(&self) -> f64 {
        if self.individuals.len() < 2 {
            return 0.0;
        }
        
        let mut total_distance = 0.0;
        let mut comparisons = 0;
        
        for i in 0..self.individuals.len() {
            for j in i + 1..self.individuals.len() {
                let distance = self.individuals[i].genome.distance(&self.individuals[j].genome);
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
    
    /// Update hall of fame with best individuals
    fn update_hall_of_fame(&mut self) {
        if let Some(best) = self.get_best_individual() {
            // Check if this individual should be in hall of fame
            let best_fitness = best.get_fitness_score();
            
            // Add to hall of fame if:
            // 1. Hall of fame is not full, or
            // 2. This individual is better than the worst in hall of fame
            let should_add = self.hall_of_fame.len() < self.hall_of_fame_size ||
                self.hall_of_fame.iter().any(|hof_ind| best_fitness > hof_ind.get_fitness_score());
            
            if should_add && !self.hall_of_fame.iter().any(|hof_ind| hof_ind.id == best.id) {
                self.hall_of_fame.push(best.clone());
                
                // Sort by fitness (descending)
                self.hall_of_fame.sort_by(|a, b| 
                    b.get_fitness_score().partial_cmp(&a.get_fitness_score()).unwrap());
                
                // Trim to size
                if self.hall_of_fame.len() > self.hall_of_fame_size {
                    self.hall_of_fame.truncate(self.hall_of_fame_size);
                }
            }
        }
    }
    
    /// Get top N individuals by fitness
    pub fn get_top_individuals(&self, n: usize) -> Vec<&Individual> {
        let mut evaluated = self.get_evaluated_individuals();
        evaluated.sort_by(|a, b| b.get_fitness_score().partial_cmp(&a.get_fitness_score()).unwrap());
        evaluated.into_iter().take(n).collect()
    }
    
    /// Get bottom N individuals by fitness
    pub fn get_bottom_individuals(&self, n: usize) -> Vec<&Individual> {
        let mut evaluated = self.get_evaluated_individuals();
        evaluated.sort_by(|a, b| a.get_fitness_score().partial_cmp(&b.get_fitness_score()).unwrap());
        evaluated.into_iter().take(n).collect()
    }
    
    /// Replace worst individuals with new ones
    pub fn replace_worst(&mut self, new_individuals: Vec<Individual>) {
        for new_individual in new_individuals {
            if let Some(worst_index) = self.find_worst_individual_index() {
                self.individuals[worst_index] = new_individual;
            } else {
                self.add_individual(new_individual);
            }
        }
    }
    
    /// Age all individuals
    pub fn age_population(&mut self) {
        for individual in &mut self.individuals {
            individual.age_up();
        }
    }
    
    /// Set generation number
    pub fn set_generation(&mut self, generation: u32) {
        self.generation = generation;
    }
    
    /// Get population size
    pub fn size(&self) -> usize {
        self.individuals.len()
    }
    
    /// Check if population is empty
    pub fn is_empty(&self) -> bool {
        self.individuals.is_empty()
    }
    
    /// Get individuals with fitness above threshold
    pub fn get_individuals_above_fitness(&self, threshold: f64) -> Vec<&Individual> {
        self.individuals
            .iter()
            .filter(|ind| ind.is_evaluated() && ind.get_fitness_score() > threshold)
            .collect()
    }
    
    /// Validate population integrity
    pub fn validate(&self) -> Result<()> {
        // Check for duplicate IDs
        let mut ids = std::collections::HashSet::new();
        for individual in &self.individuals {
            if !ids.insert(&individual.id) {
                return Err(anyhow!("Duplicate individual ID found: {}", individual.id));
            }
        }
        
        // Check parameter bounds
        for individual in &self.individuals {
            individual.genome.validate_bounds(&self.parameter_bounds)?;
        }
        
        Ok(())
    }
    
    /// Get population summary
    pub fn get_summary(&self) -> String {
        let evaluated_count = self.get_evaluated_individuals().len();
        let best_fitness = self.get_best_individual()
            .map(|ind| ind.get_fitness_score())
            .unwrap_or(0.0);
        let diversity = self.diversity;
        
        format!(
            "Population Gen{}: {} individuals ({} evaluated), Best: {:.4}, Diversity: {:.4}",
            self.generation, self.individuals.len(), evaluated_count, best_fitness, diversity
        )
    }
    
    /// Clear population
    pub fn clear(&mut self) {
        self.individuals.clear();
        self.stats = None;
        self.diversity = 0.0;
    }
    
    /// Export population to JSON
    pub fn export_to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| anyhow!("Failed to export population: {}", e))
    }
    
    /// Import population from JSON
    pub fn import_from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| anyhow!("Failed to import population: {}", e))
    }
}

impl Default for Population {
    fn default() -> Self {
        Self::new(50, &ParameterBounds::default())
    }
}
