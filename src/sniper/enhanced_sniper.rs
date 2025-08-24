//! 🎯 Enhanced Sniper Bot - Complete High-Frequency Trading System
//! 
//! Integrates all enhanced components for maximum profitability

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, mpsc};
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use anyhow::{Result, anyhow};
use log::{info, warn, error, debug};

use super::{
    enhanced_detector::{EnhancedPoolDetector, EnhancedTokenLaunch},
    fast_executor::{FastExecutor, FastExecutorConfig},
    rug_detector::{RugDetector, RugDetectorConfig, RugRiskAssessment, RiskRecommendation},
    profit_taker::{ProfitTaker, ProfitTakerConfig},
};

/// Enhanced sniper bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSniperConfig {
    /// Maximum SOL allocation for sniping (percentage of total capital)
    pub max_sniper_allocation_percent: Decimal,
    
    /// Maximum SOL per individual snipe
    pub max_position_sol: Decimal,
    
    /// Minimum liquidity required for sniping
    pub min_liquidity_sol: Decimal,
    
    /// Maximum daily loss limit for sniper
    pub max_daily_loss_usd: Decimal,
    
    /// Target execution time (milliseconds)
    pub target_execution_ms: u64,
    
    /// Risk tolerance (0.0 - 1.0, higher = more risk)
    pub risk_tolerance: f64,
    
    /// Enable automatic profit taking
    pub enable_auto_profit_taking: bool,
    
    /// Enable rug pull detection
    pub enable_rug_detection: bool,
    
    /// Minimum market cap for consideration
    pub min_market_cap_usd: Decimal,
    
    /// Maximum market cap for consideration
    pub max_market_cap_usd: Decimal,
    
    /// Fast executor configuration
    pub executor_config: FastExecutorConfig,
    
    /// Rug detector configuration
    pub rug_detector_config: RugDetectorConfig,
    
    /// Profit taker configuration
    pub profit_taker_config: ProfitTakerConfig,
}

impl Default for EnhancedSniperConfig {
    fn default() -> Self {
        Self {
            max_sniper_allocation_percent: Decimal::from_f64_retain(20.0).unwrap(), // 20% of capital
            max_position_sol: Decimal::from_f64_retain(1.0).unwrap(), // 1 SOL max per snipe
            min_liquidity_sol: Decimal::from_f64_retain(10.0).unwrap(), // 10 SOL minimum
            max_daily_loss_usd: Decimal::from_f64_retain(100.0).unwrap(), // $100 daily limit
            target_execution_ms: 100, // Sub-100ms target
            risk_tolerance: 0.6, // Medium-high risk tolerance
            enable_auto_profit_taking: true,
            enable_rug_detection: true,
            min_market_cap_usd: Decimal::from_f64_retain(50000.0).unwrap(), // $50k min
            max_market_cap_usd: Decimal::from_f64_retain(5000000.0).unwrap(), // $5M max
            executor_config: FastExecutorConfig::default(),
            rug_detector_config: RugDetectorConfig::default(),
            profit_taker_config: ProfitTakerConfig::default(),
        }
    }
}

/// Enhanced sniper performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSniperMetrics {
    /// Total snipes attempted today
    pub total_snipes_attempted: u64,
    
    /// Successful snipes
    pub successful_snipes: u64,
    
    /// Failed snipes
    pub failed_snipes: u64,
    
    /// Rug pulls detected and avoided
    pub rugs_avoided: u64,
    
    /// Total invested (SOL)
    pub total_invested_sol: Decimal,
    
    /// Total realized profits (SOL)
    pub total_realized_profits_sol: Decimal,
    
    /// Total unrealized profits (SOL)
    pub total_unrealized_profits_sol: Decimal,
    
    /// Best performing trade (multiplier)
    pub best_trade_multiplier: Decimal,
    
    /// Worst performing trade (multiplier)
    pub worst_trade_multiplier: Decimal,
    
    /// Average execution time (ms)
    pub average_execution_time_ms: f64,
    
    /// Success rate percentage
    pub success_rate: f64,
    
    /// ROI percentage (daily)
    pub daily_roi_percent: f64,
    
    /// Risk-adjusted return (Sharpe-like ratio)
    pub risk_adjusted_return: f64,
}

/// Enhanced sniper bot with all advanced features
pub struct EnhancedSniperBot {
    config: EnhancedSniperConfig,
    rpc_client: Arc<RpcClient>,
    keypair: Arc<Keypair>,
    
    /// Core components
    pool_detector: EnhancedPoolDetector,
    fast_executor: FastExecutor,
    rug_detector: RugDetector,
    profit_taker: ProfitTaker,
    
    /// State management
    active_positions: Arc<RwLock<HashMap<Pubkey, EnhancedPosition>>>,
    daily_metrics: Arc<RwLock<EnhancedSniperMetrics>>,
    capital_allocated: Arc<RwLock<Decimal>>,
    
    /// Communication channels
    launch_channel: (mpsc::UnboundedSender<EnhancedTokenLaunch>, Arc<RwLock<Option<mpsc::UnboundedReceiver<EnhancedTokenLaunch>>>>),
    
    /// Performance tracking
    execution_history: Arc<RwLock<Vec<SnipeExecution>>>,
    risk_assessments: Arc<RwLock<HashMap<Pubkey, RugRiskAssessment>>>,
}

/// Enhanced position tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedPosition {
    pub token_mint: Pubkey,
    pub entry_price: Decimal,
    pub position_size_sol: Decimal,
    pub tokens_held: Decimal,
    pub entry_time: SystemTime,
    pub risk_assessment: RugRiskAssessment,
    pub current_multiplier: Decimal,
    pub unrealized_pnl_sol: Decimal,
    pub realized_pnl_sol: Decimal,
    pub profit_levels_hit: Vec<bool>,
    pub status: PositionStatus,
}

/// Position status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionStatus {
    Active,
    PartiallyExited,
    FullyExited,
    StoppedOut,
    RugDetected,
}

/// Snipe execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnipeExecution {
    pub token_mint: Pubkey,
    pub execution_time_ms: u64,
    pub success: bool,
    pub position_size_sol: Decimal,
    pub entry_price: Decimal,
    pub risk_score: f64,
    pub timestamp: SystemTime,
    pub final_multiplier: Option<Decimal>,
    pub profit_realized_sol: Option<Decimal>,
}

impl EnhancedSniperBot {
    /// Create new enhanced sniper bot
    pub fn new(
        config: EnhancedSniperConfig,
        rpc_client: Arc<RpcClient>,
        keypair: Arc<Keypair>,
    ) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        let pool_detector = EnhancedPoolDetector::new(rpc_client.clone());
        let fast_executor = FastExecutor::new(
            rpc_client.clone(),
            keypair.clone(),
            config.executor_config.clone(),
        );
        let rug_detector = RugDetector::new(
            config.rug_detector_config.clone(),
            rpc_client.clone(),
        );
        let profit_taker = ProfitTaker::new(
            config.profit_taker_config.clone(),
            rpc_client.clone(),
        );
        
        Self {
            config,
            rpc_client,
            keypair,
            pool_detector,
            fast_executor,
            rug_detector,
            profit_taker,
            active_positions: Arc::new(RwLock::new(HashMap::new())),
            daily_metrics: Arc::new(RwLock::new(EnhancedSniperMetrics::default())),
            capital_allocated: Arc::new(RwLock::new(Decimal::ZERO)),
            launch_channel: (sender, Arc::new(RwLock::new(Some(receiver)))),
            execution_history: Arc::new(RwLock::new(Vec::new())),
            risk_assessments: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Start the enhanced sniper bot
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting Enhanced Sniper Bot...");
        
        // Start pool detection
        let detector_sender = self.launch_channel.0.clone();
        let detector = self.pool_detector.clone();
        tokio::spawn(async move {
            if let Err(e) = detector.start_monitoring(detector_sender).await {
                error!("Pool detector error: {}", e);
            }
        });
        
        // Start profit taking monitoring
        if self.config.enable_auto_profit_taking {
            if let Err(e) = self.profit_taker.start_monitoring().await {
                error!("Profit taker startup error: {}", e);
            }
        }
        
        // Start main processing loop
        self.start_main_loop().await?;
        
        info!("✅ Enhanced Sniper Bot started successfully");
        Ok(())
    }
    
    /// Main processing loop
    async fn start_main_loop(&self) -> Result<()> {
        let mut receiver = self.launch_channel.1.write().await.take()
            .ok_or("Launch receiver already taken")?;
        
        let config = self.config.clone();
        let fast_executor = self.fast_executor.clone();
        let rug_detector = self.rug_detector.clone();
        let profit_taker = self.profit_taker.clone();
        let active_positions = self.active_positions.clone();
        let daily_metrics = self.daily_metrics.clone();
        let capital_allocated = self.capital_allocated.clone();
        let execution_history = self.execution_history.clone();
        let risk_assessments = self.risk_assessments.clone();
        
        tokio::spawn(async move {
            while let Some(launch) = receiver.recv().await {
                let start_time = std::time::Instant::now();
                
                // Update metrics
                daily_metrics.write().await.total_snipes_attempted += 1;
                
                // Pre-flight checks
                if !Self::should_attempt_snipe(&launch, &config, &capital_allocated).await {
                    continue;
                }
                
                // Risk assessment
                let risk_assessment = if config.enable_rug_detection {
                    match rug_detector.analyze_rug_risk(&launch).await {
                        Ok(assessment) => {
                            risk_assessments.write().await.insert(launch.token_mint, assessment.clone());
                            assessment
                        }
                        Err(e) => {
                            warn!("Risk assessment failed for {}: {}", launch.token_mint, e);
                            continue;
                        }
                    }
                } else {
                    // Create dummy assessment if rug detection disabled
                    RugRiskAssessment {
                        overall_risk_score: 0.5,
                        recommendation: RiskRecommendation::TradeWithCaution,
                        // ... other fields with defaults
                        risk_factors: vec![],
                        analysis: Default::default(),
                        confidence: 0.5,
                        timestamp: SystemTime::now(),
                    }
                };
                
                // Risk-based decision
                if !Self::should_snipe_based_on_risk(&risk_assessment, config.risk_tolerance) {
                    daily_metrics.write().await.rugs_avoided += 1;
                    info!("🛡️ Avoided potential rug: {} (risk: {:.2})", 
                          launch.token_mint, risk_assessment.overall_risk_score);
                    continue;
                }
                
                // Calculate position size based on risk
                let position_size = Self::calculate_position_size(&launch, &risk_assessment, &config);
                
                // Execute snipe
                match fast_executor.execute_snipe(&launch, position_size).await {
                    Ok(result) => {
                        let execution_time = start_time.elapsed().as_millis() as u64;
                        
                        if result.success {
                            // Create enhanced position
                            let position = EnhancedPosition {
                                token_mint: launch.token_mint,
                                entry_price: result.entry_price,
                                position_size_sol: result.position_size_sol,
                                tokens_held: result.tokens_received,
                                entry_time: SystemTime::now(),
                                risk_assessment: risk_assessment.clone(),
                                current_multiplier: Decimal::ONE,
                                unrealized_pnl_sol: Decimal::ZERO,
                                realized_pnl_sol: Decimal::ZERO,
                                profit_levels_hit: vec![false; config.profit_taker_config.profit_levels.len()],
                                status: PositionStatus::Active,
                            };
                            
                            // Add to active positions
                            active_positions.write().await.insert(launch.token_mint, position);
                            
                            // Add to profit taker monitoring
                            if config.enable_auto_profit_taking {
                                if let Err(e) = profit_taker.add_position(
                                    launch.token_mint,
                                    result.entry_price,
                                    result.tokens_received,
                                ).await {
                                    error!("Failed to add position to profit taker: {}", e);
                                }
                            }
                            
                            // Update metrics
                            let mut metrics = daily_metrics.write().await;
                            metrics.successful_snipes += 1;
                            metrics.total_invested_sol += result.position_size_sol;
                            
                            // Update capital allocation
                            *capital_allocated.write().await += result.position_size_sol;
                            
                            info!("✅ Successful snipe: {} @ {} SOL ({}ms, risk: {:.2})", 
                                  launch.token_mint, result.entry_price, execution_time, risk_assessment.overall_risk_score);
                        } else {
                            daily_metrics.write().await.failed_snipes += 1;
                            error!("❌ Snipe failed: {}", result.error_message.unwrap_or_default());
                        }
                        
                        // Record execution
                        let execution = SnipeExecution {
                            token_mint: launch.token_mint,
                            execution_time_ms: execution_time,
                            success: result.success,
                            position_size_sol: result.position_size_sol,
                            entry_price: result.entry_price,
                            risk_score: risk_assessment.overall_risk_score,
                            timestamp: SystemTime::now(),
                            final_multiplier: None,
                            profit_realized_sol: None,
                        };
                        
                        execution_history.write().await.push(execution);
                    }
                    Err(e) => {
                        daily_metrics.write().await.failed_snipes += 1;
                        error!("❌ Snipe execution error: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Check if we should attempt this snipe
    async fn should_attempt_snipe(
        launch: &EnhancedTokenLaunch,
        config: &EnhancedSniperConfig,
        capital_allocated: &Arc<RwLock<Decimal>>,
    ) -> bool {
        // Check liquidity requirements
        if launch.initial_liquidity_sol < config.min_liquidity_sol {
            return false;
        }
        
        // Check market cap requirements
        if launch.estimated_market_cap_usd < config.min_market_cap_usd ||
           launch.estimated_market_cap_usd > config.max_market_cap_usd {
            return false;
        }
        
        // Check capital allocation limits
        let current_allocation = *capital_allocated.read().await;
        let max_allocation = config.max_sniper_allocation_percent; // This would need total capital context
        
        // For now, simple check against max position size
        if current_allocation + config.max_position_sol > config.max_position_sol * Decimal::from(10) {
            return false;
        }
        
        true
    }
    
    /// Determine if we should snipe based on risk assessment
    fn should_snipe_based_on_risk(assessment: &RugRiskAssessment, risk_tolerance: f64) -> bool {
        match assessment.recommendation {
            RiskRecommendation::SafeToTrade => true,
            RiskRecommendation::TradeWithCaution => assessment.overall_risk_score <= risk_tolerance,
            RiskRecommendation::HighRisk => assessment.overall_risk_score <= risk_tolerance * 0.7,
            RiskRecommendation::DoNotTrade => false,
        }
    }
    
    /// Calculate position size based on risk
    fn calculate_position_size(
        launch: &EnhancedTokenLaunch,
        risk_assessment: &RugRiskAssessment,
        config: &EnhancedSniperConfig,
    ) -> Decimal {
        let base_size = config.max_position_sol;
        
        // Adjust based on risk score (lower risk = larger position)
        let risk_multiplier = (1.0 - risk_assessment.overall_risk_score).max(0.1);
        let risk_adjusted_size = base_size * Decimal::from_f64_retain(risk_multiplier).unwrap();
        
        // Adjust based on liquidity (more liquidity = can take larger position)
        let liquidity_multiplier = (launch.initial_liquidity_sol / config.min_liquidity_sol)
            .min(Decimal::from(2)) // Cap at 2x
            .max(Decimal::from_f64_retain(0.5).unwrap()); // Min 0.5x
        
        let final_size = risk_adjusted_size * liquidity_multiplier;
        
        // Ensure within bounds
        final_size.min(config.max_position_sol).max(Decimal::from_f64_retain(0.1).unwrap())
    }
    
    /// Get current performance metrics
    pub async fn get_metrics(&self) -> EnhancedSniperMetrics {
        self.daily_metrics.read().await.clone()
    }
    
    /// Get active positions
    pub async fn get_active_positions(&self) -> HashMap<Pubkey, EnhancedPosition> {
        self.active_positions.read().await.clone()
    }
}

impl Default for EnhancedSniperMetrics {
    fn default() -> Self {
        Self {
            total_snipes_attempted: 0,
            successful_snipes: 0,
            failed_snipes: 0,
            rugs_avoided: 0,
            total_invested_sol: Decimal::ZERO,
            total_realized_profits_sol: Decimal::ZERO,
            total_unrealized_profits_sol: Decimal::ZERO,
            best_trade_multiplier: Decimal::ZERO,
            worst_trade_multiplier: Decimal::ZERO,
            average_execution_time_ms: 0.0,
            success_rate: 0.0,
            daily_roi_percent: 0.0,
            risk_adjusted_return: 0.0,
        }
    }
}
