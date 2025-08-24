//! 💰 Automated Profit Taking System
//! 
//! Sophisticated profit-taking strategies with multiple exit levels

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use anyhow::{Result, anyhow};
use log::{info, warn, error, debug};

/// Profit taking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitTakerConfig {
    /// Profit levels (multipliers from entry price)
    pub profit_levels: Vec<Decimal>,
    
    /// Percentage to sell at each level
    pub sell_percentages: Vec<Decimal>,
    
    /// Stop loss percentage (negative for loss)
    pub stop_loss_percent: Decimal,
    
    /// Trailing stop percentage
    pub trailing_stop_percent: Decimal,
    
    /// Maximum hold time in hours
    pub max_hold_time_hours: u64,
    
    /// Minimum profit threshold to start taking profits
    pub min_profit_threshold: Decimal,
    
    /// Enable dynamic profit taking based on volatility
    pub enable_dynamic_exits: bool,
    
    /// Enable trailing stops
    pub enable_trailing_stops: bool,
    
    /// Enable time-based exits
    pub enable_time_exits: bool,
    
    /// Slippage tolerance for exit trades
    pub exit_slippage_tolerance: Decimal,
}

impl Default for ProfitTakerConfig {
    fn default() -> Self {
        Self {
            profit_levels: vec![
                Decimal::from_f64_retain(2.0).unwrap(),  // 2x
                Decimal::from_f64_retain(5.0).unwrap(),  // 5x
                Decimal::from_f64_retain(10.0).unwrap(), // 10x
                Decimal::from_f64_retain(20.0).unwrap(), // 20x
            ],
            sell_percentages: vec![
                Decimal::from_f64_retain(25.0).unwrap(), // Sell 25% at 2x
                Decimal::from_f64_retain(25.0).unwrap(), // Sell 25% at 5x
                Decimal::from_f64_retain(25.0).unwrap(), // Sell 25% at 10x
                Decimal::from_f64_retain(25.0).unwrap(), // Sell 25% at 20x
            ],
            stop_loss_percent: Decimal::from_f64_retain(-50.0).unwrap(), // -50% stop loss
            trailing_stop_percent: Decimal::from_f64_retain(20.0).unwrap(), // 20% trailing stop
            max_hold_time_hours: 24 * 7, // 1 week max hold
            min_profit_threshold: Decimal::from_f64_retain(10.0).unwrap(), // 10% min profit
            enable_dynamic_exits: true,
            enable_trailing_stops: true,
            enable_time_exits: true,
            exit_slippage_tolerance: Decimal::from_f64_retain(2.0).unwrap(), // 2%
        }
    }
}

/// Position tracking for profit taking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitPosition {
    pub token_mint: Pubkey,
    pub entry_price: Decimal,
    pub entry_time: SystemTime,
    pub initial_amount: Decimal,
    pub current_amount: Decimal,
    pub highest_price: Decimal,
    pub current_price: Decimal,
    pub profit_levels_hit: Vec<bool>,
    pub trailing_stop_price: Option<Decimal>,
    pub last_price_update: SystemTime,
    pub total_sold: Decimal,
    pub total_profit_realized: Decimal,
    pub status: PositionStatus,
}

/// Position status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionStatus {
    Active,
    PartiallyExited,
    FullyExited,
    StoppedOut,
    TimeExpired,
}

/// Profit taking action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitAction {
    pub action_type: ActionType,
    pub token_mint: Pubkey,
    pub amount_to_sell: Decimal,
    pub target_price: Decimal,
    pub reason: String,
    pub urgency: ActionUrgency,
    pub timestamp: SystemTime,
}

/// Action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    TakeProfit,
    StopLoss,
    TrailingStop,
    TimeExit,
    DynamicExit,
}

/// Action urgency levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionUrgency {
    Low,
    Medium,
    High,
    Critical,
}

/// Profit taking execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitExecutionResult {
    pub success: bool,
    pub transaction_signature: Option<Signature>,
    pub amount_sold: Decimal,
    pub price_achieved: Decimal,
    pub profit_realized: Decimal,
    pub execution_time_ms: u64,
    pub slippage: Decimal,
    pub error_message: Option<String>,
    pub timestamp: SystemTime,
}

/// Automated profit taking system
pub struct ProfitTaker {
    config: ProfitTakerConfig,
    rpc_client: Arc<RpcClient>,
    
    /// Active positions being monitored
    positions: Arc<RwLock<HashMap<Pubkey, ProfitPosition>>>,
    
    /// Pending actions queue
    pending_actions: Arc<RwLock<Vec<ProfitAction>>>,
    
    /// Price monitoring
    price_cache: Arc<RwLock<HashMap<Pubkey, (Decimal, SystemTime)>>>,
    
    /// Performance metrics
    total_profits_taken: Arc<RwLock<Decimal>>,
    successful_exits: Arc<RwLock<u64>>,
    failed_exits: Arc<RwLock<u64>>,
    
    /// Jupiter client for price quotes and swaps
    jupiter_client: Arc<reqwest::Client>,
}

impl ProfitTaker {
    /// Create new profit taker
    pub fn new(config: ProfitTakerConfig, rpc_client: Arc<RpcClient>) -> Self {
        let jupiter_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(2000))
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()
            .expect("Failed to create Jupiter client");
        
        Self {
            config,
            rpc_client,
            positions: Arc::new(RwLock::new(HashMap::new())),
            pending_actions: Arc::new(RwLock::new(Vec::new())),
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            total_profits_taken: Arc::new(RwLock::new(Decimal::ZERO)),
            successful_exits: Arc::new(RwLock::new(0)),
            failed_exits: Arc::new(RwLock::new(0)),
            jupiter_client: Arc::new(jupiter_client),
        }
    }
    
    /// Add position to monitoring
    pub async fn add_position(
        &self,
        token_mint: Pubkey,
        entry_price: Decimal,
        amount: Decimal,
    ) -> Result<()> {
        let position = ProfitPosition {
            token_mint,
            entry_price,
            entry_time: SystemTime::now(),
            initial_amount: amount,
            current_amount: amount,
            highest_price: entry_price,
            current_price: entry_price,
            profit_levels_hit: vec![false; self.config.profit_levels.len()],
            trailing_stop_price: None,
            last_price_update: SystemTime::now(),
            total_sold: Decimal::ZERO,
            total_profit_realized: Decimal::ZERO,
            status: PositionStatus::Active,
        };
        
        self.positions.write().await.insert(token_mint, position);
        
        info!("📊 Added position to profit taker: {} @ {}", token_mint, entry_price);
        Ok(())
    }
    
    /// Start monitoring positions for profit taking opportunities
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("💰 Starting profit taking monitoring...");
        
        // Start price monitoring loop
        let price_monitor = self.clone();
        tokio::spawn(async move {
            price_monitor.price_monitoring_loop().await;
        });
        
        // Start action execution loop
        let action_executor = self.clone();
        tokio::spawn(async move {
            action_executor.action_execution_loop().await;
        });
        
        // Start position analysis loop
        let position_analyzer = self.clone();
        tokio::spawn(async move {
            position_analyzer.position_analysis_loop().await;
        });
        
        info!("✅ Profit taking monitoring started");
        Ok(())
    }
    
    /// Price monitoring loop
    async fn price_monitoring_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_millis(5000)); // 5 second updates
        
        loop {
            interval.tick().await;
            
            let positions = self.positions.read().await.clone();
            
            for (token_mint, _) in positions.iter() {
                if let Ok(current_price) = self.get_current_price(token_mint).await {
                    self.update_position_price(token_mint, current_price).await;
                }
            }
        }
    }
    
    /// Action execution loop
    async fn action_execution_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_millis(1000)); // 1 second checks
        
        loop {
            interval.tick().await;
            
            let actions = {
                let mut pending = self.pending_actions.write().await;
                let actions = pending.clone();
                pending.clear();
                actions
            };
            
            for action in actions {
                if let Err(e) = self.execute_profit_action(&action).await {
                    error!("Failed to execute profit action: {}", e);
                    
                    // Re-queue non-critical actions
                    if !matches!(action.urgency, ActionUrgency::Critical) {
                        self.pending_actions.write().await.push(action);
                    }
                }
            }
        }
    }
    
    /// Position analysis loop
    async fn position_analysis_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_millis(2000)); // 2 second analysis
        
        loop {
            interval.tick().await;
            
            let positions = self.positions.read().await.clone();
            
            for (token_mint, position) in positions.iter() {
                if matches!(position.status, PositionStatus::Active | PositionStatus::PartiallyExited) {
                    self.analyze_position_for_actions(token_mint, position).await;
                }
            }
        }
    }
    
    /// Update position price and check for triggers
    async fn update_position_price(&self, token_mint: &Pubkey, current_price: Decimal) {
        let mut positions = self.positions.write().await;
        
        if let Some(position) = positions.get_mut(token_mint) {
            position.current_price = current_price;
            position.last_price_update = SystemTime::now();
            
            // Update highest price for trailing stops
            if current_price > position.highest_price {
                position.highest_price = current_price;
                
                // Update trailing stop price
                if self.config.enable_trailing_stops {
                    let trailing_stop = position.highest_price * 
                        (Decimal::ONE - self.config.trailing_stop_percent / Decimal::from(100));
                    position.trailing_stop_price = Some(trailing_stop);
                }
            }
        }
    }
    
    /// Analyze position for profit taking actions
    async fn analyze_position_for_actions(&self, token_mint: &Pubkey, position: &ProfitPosition) {
        let current_multiplier = position.current_price / position.entry_price;
        
        // Check profit levels
        for (i, &profit_level) in self.config.profit_levels.iter().enumerate() {
            if !position.profit_levels_hit[i] && current_multiplier >= profit_level {
                let sell_percentage = self.config.sell_percentages[i];
                let amount_to_sell = position.current_amount * sell_percentage / Decimal::from(100);
                
                let action = ProfitAction {
                    action_type: ActionType::TakeProfit,
                    token_mint: *token_mint,
                    amount_to_sell,
                    target_price: position.current_price,
                    reason: format!("Profit level {} hit ({}x)", i + 1, profit_level),
                    urgency: ActionUrgency::High,
                    timestamp: SystemTime::now(),
                };
                
                self.pending_actions.write().await.push(action);
                
                // Mark level as hit
                let mut positions = self.positions.write().await;
                if let Some(pos) = positions.get_mut(token_mint) {
                    pos.profit_levels_hit[i] = true;
                }
            }
        }
        
        // Check stop loss
        let loss_percent = (position.current_price - position.entry_price) / position.entry_price * Decimal::from(100);
        if loss_percent <= self.config.stop_loss_percent {
            let action = ProfitAction {
                action_type: ActionType::StopLoss,
                token_mint: *token_mint,
                amount_to_sell: position.current_amount,
                target_price: position.current_price,
                reason: format!("Stop loss triggered at {:.1}%", loss_percent),
                urgency: ActionUrgency::Critical,
                timestamp: SystemTime::now(),
            };
            
            self.pending_actions.write().await.push(action);
        }
        
        // Check trailing stop
        if let Some(trailing_stop_price) = position.trailing_stop_price {
            if position.current_price <= trailing_stop_price {
                let action = ProfitAction {
                    action_type: ActionType::TrailingStop,
                    token_mint: *token_mint,
                    amount_to_sell: position.current_amount,
                    target_price: position.current_price,
                    reason: "Trailing stop triggered".to_string(),
                    urgency: ActionUrgency::High,
                    timestamp: SystemTime::now(),
                };
                
                self.pending_actions.write().await.push(action);
            }
        }
        
        // Check time-based exit
        if self.config.enable_time_exits {
            let hold_time = position.entry_time.elapsed().unwrap_or(Duration::ZERO);
            let max_hold_duration = Duration::from_secs(self.config.max_hold_time_hours * 3600);
            
            if hold_time >= max_hold_duration {
                let action = ProfitAction {
                    action_type: ActionType::TimeExit,
                    token_mint: *token_mint,
                    amount_to_sell: position.current_amount,
                    target_price: position.current_price,
                    reason: "Maximum hold time reached".to_string(),
                    urgency: ActionUrgency::Medium,
                    timestamp: SystemTime::now(),
                };
                
                self.pending_actions.write().await.push(action);
            }
        }
    }
    
    /// Execute profit taking action
    async fn execute_profit_action(&self, action: &ProfitAction) -> Result<ProfitExecutionResult> {
        let start_time = Instant::now();
        
        info!("💰 Executing profit action: {:?} for {} tokens", 
              action.action_type, action.amount_to_sell);
        
        // Get Jupiter quote for selling
        let quote = self.get_sell_quote(&action.token_mint, action.amount_to_sell).await?;
        
        // Execute the swap
        let result = self.execute_sell_swap(&action.token_mint, action.amount_to_sell, &quote).await;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(signature) => {
                // Calculate actual results
                let (price_achieved, profit_realized) = self.calculate_sell_results(
                    &signature,
                    action.amount_to_sell,
                    &action.token_mint,
                ).await?;
                
                // Update position
                self.update_position_after_sell(
                    &action.token_mint,
                    action.amount_to_sell,
                    profit_realized,
                ).await;
                
                // Update metrics
                *self.total_profits_taken.write().await += profit_realized;
                *self.successful_exits.write().await += 1;
                
                info!("✅ Profit action executed: {} SOL profit in {}ms", 
                      profit_realized, execution_time);
                
                Ok(ProfitExecutionResult {
                    success: true,
                    transaction_signature: Some(signature),
                    amount_sold: action.amount_to_sell,
                    price_achieved,
                    profit_realized,
                    execution_time_ms: execution_time,
                    slippage: Decimal::ZERO, // Calculate from expected vs actual
                    error_message: None,
                    timestamp: SystemTime::now(),
                })
            }
            Err(e) => {
                *self.failed_exits.write().await += 1;
                
                error!("❌ Profit action failed: {} ({}ms)", e, execution_time);
                
                Ok(ProfitExecutionResult {
                    success: false,
                    transaction_signature: None,
                    amount_sold: Decimal::ZERO,
                    price_achieved: Decimal::ZERO,
                    profit_realized: Decimal::ZERO,
                    execution_time_ms: execution_time,
                    slippage: Decimal::ZERO,
                    error_message: Some(e.to_string()),
                    timestamp: SystemTime::now(),
                })
            }
        }
    }
    
    /// Get current token price
    async fn get_current_price(&self, token_mint: &Pubkey) -> Result<Decimal> {
        // Check cache first
        {
            let cache = self.price_cache.read().await;
            if let Some((price, timestamp)) = cache.get(token_mint) {
                if timestamp.elapsed().unwrap_or(Duration::MAX) < Duration::from_secs(10) {
                    return Ok(*price);
                }
            }
        }
        
        // Fetch from Jupiter
        let url = format!(
            "https://quote-api.jup.ag/v6/quote?inputMint={}&outputMint=So11111111111111111111111111111111111111112&amount=1000000",
            token_mint
        );
        
        let response = self.jupiter_client.get(&url).send().await?;
        let quote: serde_json::Value = response.json().await?;
        
        let out_amount: u64 = quote["outAmount"].as_str()
            .ok_or_else(|| anyhow!("Invalid quote response"))?
            .parse()?;
        
        let price = Decimal::from(out_amount) / Decimal::from(1_000_000_000); // Convert lamports to SOL
        
        // Update cache
        self.price_cache.write().await.insert(*token_mint, (price, SystemTime::now()));
        
        Ok(price)
    }
    
    // Additional helper methods would be implemented here...
    // get_sell_quote, execute_sell_swap, calculate_sell_results, update_position_after_sell
}

impl Clone for ProfitTaker {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            rpc_client: self.rpc_client.clone(),
            positions: self.positions.clone(),
            pending_actions: self.pending_actions.clone(),
            price_cache: self.price_cache.clone(),
            total_profits_taken: self.total_profits_taken.clone(),
            successful_exits: self.successful_exits.clone(),
            failed_exits: self.failed_exits.clone(),
            jupiter_client: self.jupiter_client.clone(),
        }
    }
}
