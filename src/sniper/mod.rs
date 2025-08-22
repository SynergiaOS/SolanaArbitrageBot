//! Sniper Bot Module
//! Advanced memecoin sniping functionality

pub mod monitor;
pub mod executor;
pub mod safety;
pub mod position;

pub use monitor::{TokenMonitor, NewToken};
pub use executor::{TradeExecutor, TradeResult, TradeParams};
pub use safety::{SafetyChecker, SafetyResult};
pub use position::{PositionManager, Position, SellAction};

use std::sync::Arc;
use solana_sdk::signature::Keypair;
use solana_client::nonblocking::rpc_client::RpcClient;
use anyhow::Result;
use log::{info, error, warn};
use tokio::sync::mpsc;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SafetyConfig {
    // Liquidity & Market Cap
    pub min_liquidity_sol: f64,
    pub max_market_cap_usd: f64,

    // Taxes
    pub max_buy_tax_percent: f64,
    pub max_sell_tax_percent: f64,

    // Token Age & Holders
    pub max_token_age_minutes: u32,
    pub min_holders: u32,
    pub max_dev_percentage: f64,

    // Blacklists
    pub blacklist_mints: Vec<String>,
    pub blacklisted_creators: Vec<String>,
    pub blacklist_keywords: Vec<String>,

    // API Endpoints
    pub honeypot_api: Option<String>,
    pub rugcheck_api: Option<String>,
    pub helius_api_key: Option<String>,
    pub enable_safety_checks: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            min_liquidity_sol: 3.0,
            max_market_cap_usd: 100_000.0,
            max_buy_tax_percent: 5.0,
            max_sell_tax_percent: 5.0,
            max_token_age_minutes: 60,
            min_holders: 10,
            max_dev_percentage: 30.0,
            blacklist_mints: vec![],
            blacklisted_creators: vec![],
            blacklist_keywords: vec![
                "test".to_string(),
                "fake".to_string(),
                "scam".to_string(),
                "rug".to_string(),
                "honeypot".to_string(),
            ],
            honeypot_api: Some("https://api.honeypot.is/v2/IsHoneypot".to_string()),
            rugcheck_api: Some("https://api.rugcheck.xyz/v1/tokens".to_string()),
            helius_api_key: None,
            enable_safety_checks: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SniperConfig {
    pub max_position_sol: f64,
    pub min_liquidity_sol: f64,
    pub max_buy_tax: f64,
    pub max_sell_tax: f64,
    pub profit_target_percent: f64,
    pub stop_loss_percent: f64,
    pub max_market_cap: f64,
    pub scan_interval_ms: u64,
    pub max_slippage_bps: u16,
    pub priority_fee_lamports: u64,
    pub position_timeout_minutes: u64,
}

impl Default for SniperConfig {
    fn default() -> Self {
        Self {
            max_position_sol: 0.05,
            min_liquidity_sol: 5.0,
            max_buy_tax: 5.0,
            max_sell_tax: 5.0,
            profit_target_percent: 200.0,
            stop_loss_percent: 50.0,
            max_market_cap: 100_000.0,
            scan_interval_ms: 250,
            max_slippage_bps: 1000, // 10%
            priority_fee_lamports: 100_000,
            position_timeout_minutes: 60,
        }
    }
}

pub struct SniperEngine {
    keypair: Arc<Keypair>,
    rpc_client: Arc<RpcClient>,
    config: SniperConfig,
    safety_config: SafetyConfig,
    token_monitor: TokenMonitor,
    trade_executor: TradeExecutor,
    safety_checker: SafetyChecker,
    position_manager: PositionManager,
    dry_run: bool,
}

impl SniperEngine {
    pub fn new(
        keypair: Arc<Keypair>,
        rpc_url: String,
        ws_url: String,
        config: SniperConfig,
        safety_config: SafetyConfig,
        dry_run: bool,
    ) -> Self {
        let rpc_client = Arc::new(RpcClient::new(rpc_url));
        
        let token_monitor = TokenMonitor::new(rpc_client.clone(), ws_url);
        let trade_executor = TradeExecutor::new(
            keypair.clone(),
            rpc_client.clone(),
            "https://quote-api.jup.ag/v6".to_string(),
        );
        // Przekaż safety_config do SafetyChecker
        let safety_checker = SafetyChecker::from_config(&safety_config, rpc_client.clone());
        let position_manager = PositionManager::new(config.clone());

        Self {
            keypair,
            rpc_client,
            config,
            safety_config,
            token_monitor,
            trade_executor,
            safety_checker,
            position_manager,
            dry_run,
        }
    }
    
    pub async fn start(&self) -> Result<()> {
        info!("🎯 Starting Memecoin Sniper Engine!");
        info!("💰 Max position: {} SOL", self.config.max_position_sol);
        info!("🎯 Profit target: {}%", self.config.profit_target_percent);
        info!("🛑 Stop loss: {}%", self.config.stop_loss_percent);
        info!("🏃 Mode: {}", if self.dry_run { "DRY RUN" } else { "LIVE TRADING" });
        
        // Check wallet balance
        let balance = self.trade_executor.get_sol_balance().await?;
        let sol_balance = balance as f64 / 1_000_000_000.0;
        info!("💳 Wallet balance: {:.4} SOL", sol_balance);
        
        if sol_balance < self.config.max_position_sol * 2.0 {
            warn!("⚠️ Low balance! Recommended: {:.2} SOL", self.config.max_position_sol * 2.0);
        }
        
        // Start monitoring tasks
        let token_scanner = self.start_token_scanner();
        let position_monitor = self.start_position_monitor();
        
        // Run both tasks concurrently
        tokio::try_join!(token_scanner, position_monitor)?; // run both loops
        
        Ok(())
    }
    
    async fn start_token_scanner(&self) -> Result<()> {
        info!("🔍 Starting token scanner...");
        
        let mut new_token_rx = self.token_monitor.subscribe_to_new_tokens().await?;
        
        while let Some(new_token) = new_token_rx.recv().await {
            info!("🆕 New token detected: {} ({})", new_token.symbol, new_token.mint);
            
            // Safety check
            match self.safety_checker.check_token(&new_token).await {
                Ok(SafetyResult::Safe) => {
                    info!("✅ Token passed safety checks: {}", new_token.mint);
                    
                    if let Err(e) = self.execute_snipe(&new_token).await {
                        error!("❌ Snipe failed for {}: {}", new_token.symbol, e);
                    }
                }
                Ok(SafetyResult::Unsafe(reason)) => {
                    warn!("❌ Token failed safety check: {} - {}", new_token.mint, reason);
                }
                Err(e) => {
                    error!("❌ Safety check error for {}: {}", new_token.mint, e);
                }
            }
        }
        
        Ok(())
    }
    
    async fn execute_snipe(&self, token: &NewToken) -> Result<()> {
        if self.dry_run {
            info!("🏃 DRY RUN - Would snipe {} for {} SOL", 
                  token.symbol, self.config.max_position_sol);
            return Ok(());
        }
        
        info!("🔫 EXECUTING SNIPE: {} for {} SOL", token.symbol, self.config.max_position_sol);
        
        let sol_amount = (self.config.max_position_sol * 1_000_000_000.0) as u64;
        
        match self.trade_executor.buy_token(
            &token.mint,
            sol_amount,
            self.config.max_slippage_bps,
        ).await {
            Ok(trade_result) => {
                info!("✅ SNIPE SUCCESS: {} - TX: {}", token.symbol, trade_result.signature);
                info!("💰 Bought {} tokens for {} SOL", 
                      trade_result.output_amount, 
                      trade_result.input_amount as f64 / 1_000_000_000.0);
                
                // Add to position manager
                let position = Position::new(
                    token.mint,
                    trade_result.input_amount,
                    trade_result.output_amount,
                    trade_result.signature,
                );
                
                self.position_manager.add_position(position).await;
            }
            Err(e) => {
                error!("❌ Snipe failed: {}", e);
                return Err(e);
            }
        }
        
        Ok(())
    }
    
    async fn start_position_monitor(&self) -> Result<()> {
        info!("📊 Starting position monitor...");
        
        loop {
            if let Err(e) = self.monitor_positions().await {
                error!("Position monitoring error: {}", e);
            }
            
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }
    
    async fn monitor_positions(&self) -> Result<()> {
        let positions = self.position_manager.get_active_positions().await;
        
        for position in positions {
            match self.position_manager.check_sell_conditions(&position, &self.config).await { // includes timeout
                Some(SellAction::TakeProfit) => {
                    info!("🎯 Taking profit on {}", position.mint);
                    self.execute_sell(&position, "profit target").await?;
                }
                Some(SellAction::StopLoss) => {
                    warn!("🛑 Stop loss triggered on {}", position.mint);
                    self.execute_sell(&position, "stop loss").await?;
                }
                Some(SellAction::Timeout) => {
                    info!("⏰ Position timeout on {}", position.mint);
                    self.execute_sell(&position, "timeout").await?;
                }
                None => {
                    // Continue holding
                }
            }
        }
        
        Ok(())
    }
    
    async fn execute_sell(&self, position: &Position, reason: &str) -> Result<()> {
        if self.dry_run {
            info!("🏃 DRY RUN - Would sell {} ({})", position.mint, reason);
            return Ok(());
        }
        
        info!("💰 SELLING: {} ({}) - {} tokens", position.mint, reason, position.token_amount);
        
        match self.trade_executor.sell_token(
            &position.mint,
            position.token_amount,
            self.config.max_slippage_bps * 2, // Higher slippage for sells
        ).await {
            Ok(trade_result) => {
                info!("✅ SELL SUCCESS: {} - TX: {}", position.mint, trade_result.signature);
                
                let profit_loss = trade_result.output_amount as i64 - position.sol_amount as i64;
                let profit_percent = (profit_loss as f64 / position.sol_amount as f64) * 100.0;
                
                info!("📊 P&L: {:.4} SOL ({:.1}%)", 
                      profit_loss as f64 / 1_000_000_000.0, 
                      profit_percent);
                
                // Remove from position manager
                self.position_manager.remove_position(&position.mint).await;
            }
            Err(e) => {
                error!("❌ Sell failed: {}", e);
                return Err(e);
            }
        }
        
        Ok(())
    }
}
