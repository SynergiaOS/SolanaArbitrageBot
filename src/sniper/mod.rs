//! Sniper Bot Module
//! Advanced memecoin sniping functionality

pub mod executor;
pub mod monitor;
pub mod position;
pub mod rug_monitor;
pub mod safety;

// Enhanced sniper components
// Temporarily disabled due to compilation errors
// pub mod enhanced_detector;
// pub mod fast_executor;
// pub mod rug_detector;
// pub mod profit_taker;

// Simple but effective components
pub mod detector;
pub mod types;

pub use executor::{TradeExecutor, TradeParams, TradeResult};
pub use monitor::{NewToken, TokenMonitor};
pub use position::{Position, PositionManager, SellAction};
pub use rug_monitor::{MonitoredPosition, RugAlert, RugMonitor, RugMonitorConfig};
pub use safety::{SafetyChecker, SafetyConfig, SafetyResult};

// Enhanced exports (temporarily disabled)
// pub use enhanced_detector::{EnhancedPoolDetector, EnhancedTokenLaunch};
// pub use fast_executor::{FastExecutor, FastExecutorConfig, FastExecutionResult};
// pub use rug_detector::{RugDetector, RugDetectorConfig, RugRiskAssessment};
// pub use profit_taker::{ProfitTaker, ProfitTakerConfig, ProfitPosition};

use anyhow::Result;
use log::{error, info, warn};

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use std::sync::Arc;
use tokio::sync::Mutex;



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
    #[allow(dead_code)]
    keypair: Arc<Keypair>,
    #[allow(dead_code)]
    rpc_client: Arc<RpcClient>,
    config: SniperConfig,
    #[allow(dead_code)]
    safety_config: SafetyConfig,
    token_monitor: TokenMonitor,
    trade_executor: TradeExecutor,
    safety_checker: SafetyChecker,
    position_manager: PositionManager,
    rug_monitor: Arc<Mutex<RugMonitor>>,
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
        // Inicjalizuj RugMonitor dla post-trade bezpieczeństwa
        let rug_monitor = Arc::new(Mutex::new(RugMonitor::new(rpc_client.clone())));

        Self {
            keypair,
            rpc_client,
            config,
            safety_config,
            token_monitor,
            trade_executor,
            safety_checker,
            position_manager,
            rug_monitor,
            dry_run,
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("🎯 Starting Memecoin Sniper Engine!");
        info!("💰 Max position: {} SOL", self.config.max_position_sol);
        info!("🎯 Profit target: {}%", self.config.profit_target_percent);
        info!("🛑 Stop loss: {}%", self.config.stop_loss_percent);
        info!(
            "🏃 Mode: {}",
            if self.dry_run {
                "DRY RUN"
            } else {
                "LIVE TRADING"
            }
        );

        // Check wallet balance
        let balance = self.trade_executor.get_sol_balance().await?;
        let sol_balance = balance as f64 / 1_000_000_000.0;
        info!("💳 Wallet balance: {:.4} SOL", sol_balance);

        if sol_balance < self.config.max_position_sol * 2.0 {
            warn!(
                "⚠️ Low balance! Recommended: {:.2} SOL",
                self.config.max_position_sol * 2.0
            );
        }

        // Start monitoring tasks
        let token_scanner = self.start_token_scanner();
        let position_monitor = self.start_position_monitor();
        let rug_monitor_task = self.start_rug_monitoring();

        // Run all tasks concurrently
        tokio::try_join!(token_scanner, position_monitor, rug_monitor_task)?; // run all loops

        Ok(())
    }

    async fn start_token_scanner(&self) -> Result<()> {
        info!("🔍 Starting token scanner...");

        let mut new_token_rx = self.token_monitor.subscribe_to_new_tokens().await?;

        while let Some(new_token) = new_token_rx.recv().await {
            info!(
                "🆕 New token detected: {} ({})",
                new_token.symbol, new_token.mint
            );

            // Safety check
            match self.safety_checker.check_token(&new_token).await {
                Ok(SafetyResult::Safe) => {
                    info!("✅ Token passed safety checks: {}", new_token.mint);

                    if let Err(e) = self.execute_snipe(&new_token).await {
                        error!("❌ Snipe failed for {}: {}", new_token.symbol, e);
                    }
                }
                Ok(SafetyResult::Unsafe(reason)) => {
                    warn!(
                        "❌ Token failed safety check: {} - {}",
                        new_token.mint, reason
                    );
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
            info!(
                "🏃 DRY RUN - Would snipe {} for {} SOL",
                token.symbol, self.config.max_position_sol
            );
            return Ok(());
        }

        info!(
            "🔫 EXECUTING SNIPE: {} for {} SOL",
            token.symbol, self.config.max_position_sol
        );

        let sol_amount = (self.config.max_position_sol * 1_000_000_000.0) as u64;

        match self
            .trade_executor
            .buy_token(&token.mint, sol_amount, self.config.max_slippage_bps)
            .await
        {
            Ok(trade_result) => {
                info!(
                    "✅ SNIPE SUCCESS: {} - TX: {}",
                    token.symbol, trade_result.signature
                );
                info!(
                    "💰 Bought {} tokens for {} SOL",
                    trade_result.output_amount,
                    trade_result.input_amount as f64 / 1_000_000_000.0
                );

                // Add to position manager
                let position = Position::new(
                    token.mint,
                    trade_result.input_amount,
                    trade_result.output_amount,
                    trade_result.signature,
                );

                self.position_manager.add_position(position.clone()).await;

                // KRYTYCZNE: Rozpocznij monitoring rug pull po zakupie
                let rug_monitor = self.rug_monitor.lock().await;
                if let Err(e) = rug_monitor.start_monitoring(position, token.clone()).await {
                    error!(
                        "❌ Failed to start rug monitoring for {}: {}",
                        token.mint, e
                    );
                } else {
                    info!("🛡️ Rug monitoring started for position: {}", token.mint);
                }
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
            match self
                .position_manager
                .check_sell_conditions(&position, &self.config)
                .await
            {
                // includes timeout
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

    async fn start_rug_monitoring(&self) -> Result<()> {
        info!("🛡️ Starting rug pull monitoring...");

        loop {
            let rug_monitor = self.rug_monitor.lock().await;
            // Check all monitored positions for rug pull indicators
            match rug_monitor.check_all_positions().await {
                Ok(alerts) => {
                    for (mint, alert) in alerts {
                        match alert {
                            crate::sniper::rug_monitor::RugAlert::LiquidityDrain {
                                drop_percent,
                                ..
                            } => {
                                if drop_percent >= 30.0 {
                                    warn!(
                                        "🚨 RUG PULL ALERT: {} - Liquidity dropped by {:.1}%",
                                        mint, drop_percent
                                    );
                                    if let Err(e) = rug_monitor
                                        .emergency_sell(&mint, "Liquidity drain detected")
                                        .await
                                    {
                                        error!("❌ Emergency sell failed: {}", e);
                                    }
                                }
                            }
                            crate::sniper::rug_monitor::RugAlert::AuthorityChanged { .. } => {
                                warn!("🚨 RUG PULL ALERT: {} - Authority changed!", mint);
                                if let Err(e) =
                                    rug_monitor.emergency_sell(&mint, "Authority changed").await
                                {
                                    error!("❌ Emergency sell failed: {}", e);
                                }
                            }
                            crate::sniper::rug_monitor::RugAlert::TaxIncreased { .. } => {
                                warn!("🚨 RUG PULL ALERT: {} - Taxes increased!", mint);
                                if let Err(e) =
                                    rug_monitor.emergency_sell(&mint, "Taxes increased").await
                                {
                                    error!("❌ Emergency sell failed: {}", e);
                                }
                            }
                            crate::sniper::rug_monitor::RugAlert::FailedTransaction { error } => {
                                warn!("🚨 TRANSACTION ALERT: {} - Failed: {}", mint, error);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("❌ Rug monitoring error: {}", e);
                }
            }

            // Check every 5 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    async fn execute_sell(&self, position: &Position, reason: &str) -> Result<()> {
        if self.dry_run {
            info!("🏃 DRY RUN - Would sell {} ({})", position.mint, reason);
            return Ok(());
        }

        info!(
            "💰 SELLING: {} ({}) - {} tokens",
            position.mint, reason, position.token_amount
        );

        match self
            .trade_executor
            .sell_token(
                &position.mint,
                position.token_amount,
                self.config.max_slippage_bps * 2, // Higher slippage for sells
            )
            .await
        {
            Ok(trade_result) => {
                info!(
                    "✅ SELL SUCCESS: {} - TX: {}",
                    position.mint, trade_result.signature
                );

                let profit_loss = trade_result.output_amount as i64 - position.sol_amount as i64;
                let profit_percent = (profit_loss as f64 / position.sol_amount as f64) * 100.0;

                info!(
                    "📊 P&L: {:.4} SOL ({:.1}%)",
                    profit_loss as f64 / 1_000_000_000.0,
                    profit_percent
                );

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
