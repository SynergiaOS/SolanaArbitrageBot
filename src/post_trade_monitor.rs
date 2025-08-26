//! Post-Trade Monitoring Module - Rug Pull Detection
//! Real-time monitoring after trade execution for safety

use anyhow::{Result, Context};
use log::{error, info, warn};
use rust_decimal::Decimal;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};

use crate::config_manager::{BotConfig};
use crate::utils::conversions::*;

/// Events that can trigger emergency actions
#[derive(Debug, Clone)]
pub enum MonitoringEvent {
    LpDrain {
        pool: String,
        percent_drained: Decimal,
    },
    AuthorityChange {
        mint: String,
        old_authority: Option<String>,
        new_authority: Option<String>,
    },
    TaxIncrease {
        token: String,
        buy_tax: Decimal,
        sell_tax: Decimal,
    },
    TradingHalted {
        token: String,
        reason: String,
    },
    PriceCollapse {
        token: String,
        drop_percent: Decimal,
    },
}

/// Position being monitored
#[derive(Debug, Clone)]
pub struct MonitoredPosition {
    pub token_mint: Pubkey,
    pub pool_address: Pubkey,
    pub entry_price: Decimal,
    pub amount: Decimal,
    pub entry_time: std::time::Instant,
    pub dex: String,
}

/// Post-trade monitor for rug pull detection
pub struct PostTradeMonitor {
    config: Arc<RwLock<BotConfig>>,
    rpc_client: Arc<RpcClient>,
    positions: Arc<RwLock<Vec<MonitoredPosition>>>,
    event_tx: mpsc::Sender<MonitoringEvent>,
    event_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<MonitoringEvent>>>,
    monitoring_active: Arc<AtomicBool>,
}

impl PostTradeMonitor {
    /// Create new post-trade monitor
    pub fn new(config: Arc<RwLock<BotConfig>>, rpc_client: Arc<RpcClient>) -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);
        
        Self {
            config,
            rpc_client,
            positions: Arc::new(RwLock::new(Vec::new())),
            event_tx,
            event_rx: Arc::new(tokio::sync::Mutex::new(event_rx)),
            monitoring_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Add position to monitor
    pub async fn add_position(&self, position: MonitoredPosition) -> Result<()> {
        let mut positions = self.positions.write().await;
        positions.push(position.clone());
        
        info!("📊 Added position to monitoring: {} on {}", 
            position.token_mint, position.dex);
        
        // Start monitoring if not already active
        if !self.monitoring_active.load(std::sync::atomic::Ordering::Relaxed) {
            self.start_monitoring().await?;
        }
        
        Ok(())
    }

    /// Remove position from monitoring
    pub async fn remove_position(&self, token_mint: &Pubkey) -> Result<()> {
        let mut positions = self.positions.write().await;
        positions.retain(|p| p.token_mint != *token_mint);
        
        info!("📊 Removed position from monitoring: {}", token_mint);
        
        // Stop monitoring if no positions
        if positions.is_empty() {
            self.monitoring_active.store(false, std::sync::atomic::Ordering::Relaxed);
        }
        
        Ok(())
    }

    /// Start monitoring all positions
    pub async fn start_monitoring(&self) -> Result<()> {
        if self.monitoring_active.swap(true, std::sync::atomic::Ordering::Relaxed) {
            return Ok(()); // Already monitoring
        }

        info!("🔍 Starting post-trade monitoring...");

        let config = self.config.clone();
        let rpc_client = self.rpc_client.clone();
        let positions = self.positions.clone();
        let event_tx = self.event_tx.clone();
        let monitoring_active = self.monitoring_active.clone();

        // Spawn monitoring tasks
        tokio::spawn(async move {
            let mut tasks = vec![];

            // LP monitoring task
            let lp_task = Self::monitor_liquidity_pools(
                config.clone(),
                rpc_client.clone(),
                positions.clone(),
                event_tx.clone(),
                monitoring_active.clone(),
            );
            tasks.push(tokio::spawn(lp_task));

            // Authority monitoring task
            let auth_task = Self::monitor_authorities(
                config.clone(),
                rpc_client.clone(),
                positions.clone(),
                event_tx.clone(),
                monitoring_active.clone(),
            );
            tasks.push(tokio::spawn(auth_task));

            // Price monitoring task
            let price_task = Self::monitor_prices(
                config.clone(),
                rpc_client.clone(),
                positions.clone(),
                event_tx.clone(),
                monitoring_active.clone(),
            );
            tasks.push(tokio::spawn(price_task));

            // Wait for all tasks
            for task in tasks {
                if let Err(e) = task.await {
                    error!("Monitoring task failed: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Monitor liquidity pools for drains
    async fn monitor_liquidity_pools(
        config: Arc<RwLock<BotConfig>>,
        rpc_client: Arc<RpcClient>,
        positions: Arc<RwLock<Vec<MonitoredPosition>>>,
        event_tx: mpsc::Sender<MonitoringEvent>,
        active: Arc<AtomicBool>,
    ) -> Result<()> {
        let check_interval = {
            let cfg = config.read().await;
            Duration::from_secs(cfg.monitoring.rug_pull_detection.check_interval_seconds)
        };

        let mut interval = interval(check_interval);
        let mut initial_reserves: std::collections::HashMap<String, (u64, u64)> = 
            std::collections::HashMap::new();

        while active.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;

            let positions_snapshot = positions.read().await.clone();
            
            for position in positions_snapshot {
                let pool_key = position.pool_address.to_string();
                
                // Get current reserves
                match Self::get_pool_reserves(&rpc_client, &position.pool_address).await {
                    Ok((base_reserve, quote_reserve)) => {
                        // Check if we have initial reserves
                        if let Some((initial_base, initial_quote)) = initial_reserves.get(&pool_key) {
                            // Calculate drain percentage
                            let base_drain = if *initial_base > 0 {
                                ((*initial_base as f64 - base_reserve as f64) / *initial_base as f64) * 100.0
                            } else { 0.0 };
                            
                            let quote_drain = if *initial_quote > 0 {
                                ((*initial_quote as f64 - quote_reserve as f64) / *initial_quote as f64) * 100.0
                            } else { 0.0 };

                            let max_drain = base_drain.max(quote_drain);
                            let threshold = {
                                let cfg = config.read().await;
                                decimal_to_f64(cfg.monitoring.rug_pull_detection.lp_drain_threshold_percent)
                            };

                            if max_drain > threshold {
                                warn!("⚠️ LP DRAIN DETECTED: {:.1}% drained from pool {}", 
                                    max_drain, pool_key);
                                
                                let event = MonitoringEvent::LpDrain {
                                    pool: pool_key.clone(),
                                    percent_drained: f64_to_decimal(max_drain),
                                };
                                
                                let _ = event_tx.send(event).await;
                            }
                        } else {
                            // Store initial reserves
                            initial_reserves.insert(pool_key, (base_reserve, quote_reserve));
                        }
                    }
                    Err(e) => {
                        error!("Failed to get pool reserves: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Monitor mint authorities for changes
    async fn monitor_authorities(
        _config: Arc<RwLock<BotConfig>>,
        rpc_client: Arc<RpcClient>,
        positions: Arc<RwLock<Vec<MonitoredPosition>>>,
        event_tx: mpsc::Sender<MonitoringEvent>,
        active: Arc<AtomicBool>,
    ) -> Result<()> {
        let mut interval = interval(Duration::from_secs(30));
        let mut known_authorities: std::collections::HashMap<String, (Option<Pubkey>, Option<Pubkey>)> = 
            std::collections::HashMap::new();

        while active.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;

            let positions_snapshot = positions.read().await.clone();
            
            for position in positions_snapshot {
                let mint_key = position.token_mint.to_string();
                
                // Get current authorities
                match Self::get_mint_authorities(&rpc_client, &position.token_mint).await {
                    Ok((mint_auth, freeze_auth)) => {
                        if let Some((known_mint, known_freeze)) = known_authorities.get(&mint_key) {
                            // Check for changes
                            if *known_mint != mint_auth || *known_freeze != freeze_auth {
                                warn!("⚠️ AUTHORITY CHANGE DETECTED for token {}", mint_key);
                                
                                let event = MonitoringEvent::AuthorityChange {
                                    mint: mint_key.clone(),
                                    old_authority: known_mint.map(|a| a.to_string()),
                                    new_authority: mint_auth.map(|a| a.to_string()),
                                };
                                
                                let _ = event_tx.send(event).await;
                                
                                // Update known authorities
                                known_authorities.insert(mint_key.clone(), (mint_auth, freeze_auth));
                            }
                        } else {
                            // Store initial authorities
                            known_authorities.insert(mint_key, (mint_auth, freeze_auth));
                        }
                    }
                    Err(e) => {
                        error!("Failed to get mint authorities: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Monitor token prices for collapse
    async fn monitor_prices(
        config: Arc<RwLock<BotConfig>>,
        _rpc_client: Arc<RpcClient>,
        positions: Arc<RwLock<Vec<MonitoredPosition>>>,
        event_tx: mpsc::Sender<MonitoringEvent>,
        active: Arc<AtomicBool>,
    ) -> Result<()> {
        let mut interval = interval(Duration::from_secs(10));

        while active.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;

            let positions_snapshot = positions.read().await.clone();
            
            for position in positions_snapshot {
                // Get current price from Jupiter
                match Self::get_current_price(&position.token_mint).await {
                    Ok(current_price) => {
                        let price_change = ((current_price - position.entry_price) / position.entry_price) * Decimal::from(100);
                        
                        // Check for significant price drop
                        if price_change < Decimal::from(-30) {
                            warn!("⚠️ PRICE COLLAPSE: Token {} down {:.1}%", 
                                position.token_mint, price_change);
                            
                            let event = MonitoringEvent::PriceCollapse {
                                token: position.token_mint.to_string(),
                                drop_percent: price_change.abs(),
                            };
                            
                            let _ = event_tx.send(event).await;
                        }
                    }
                    Err(e) => {
                        error!("Failed to get current price: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get pool reserves
    async fn get_pool_reserves(
        rpc_client: &RpcClient,
        pool_address: &Pubkey,
    ) -> Result<(u64, u64)> {
        // This would fetch actual pool data from chain
        // Simplified for now
        let _account = rpc_client.get_account(pool_address).await?;
        
        // Parse pool state based on DEX type
        // Return (base_reserve, quote_reserve)
        Ok((1000000, 1000000)) // Placeholder
    }

    /// Get mint authorities
    async fn get_mint_authorities(
        rpc_client: &RpcClient,
        mint: &Pubkey,
    ) -> Result<(Option<Pubkey>, Option<Pubkey>)> {
    let account = rpc_client.get_account(mint).await?;
    
    // Parse mint account
    use solana_program_pack::Pack;
    let mint_data = spl_token::state::Mint::unpack(&account.data)?;
    
    let mint_authority = mint_data.mint_authority.into();
    let freeze_authority = mint_data.freeze_authority.into();
    
    Ok((mint_authority, freeze_authority))
}

    /// Get current price from Jupiter
    async fn get_current_price(token_mint: &Pubkey) -> Result<Decimal> {
        // Call Jupiter price API
        let url = format!(
            "https://price.jup.ag/v4/price?ids={}",
            token_mint
        );
        
        let client = reqwest::Client::new();
        let response = client.get(&url).send().await?;
        let data: serde_json::Value = response.json().await?;
        
        let price = data["data"][token_mint.to_string()]["price"]
            .as_f64()
            .context("Failed to parse price")?;
        
        Ok(f64_to_decimal(price))
    }

    /// Handle monitoring events
    pub async fn handle_events<F>(&self, mut handler: F) -> Result<()>
    where
        F: FnMut(MonitoringEvent) -> Result<()>,
    {
        let mut rx = self.event_rx.lock().await;
        
        while let Some(event) = rx.recv().await {
            handler(event)?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_position_management() {
        let config = Arc::new(RwLock::new(crate::config_manager::BotConfig::default()));
        let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
        
        let monitor = PostTradeMonitor::new(config, rpc_client);
        
        let position = MonitoredPosition {
            token_mint: Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            pool_address: Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            entry_price: Decimal::from(100),
            amount: Decimal::from(10),
            entry_time: std::time::Instant::now(),
            dex: "Raydium".to_string(),
        };
        
        monitor.add_position(position.clone()).await.unwrap();
        
        let positions = monitor.positions.read().await;
        assert_eq!(positions.len(), 1);
    }
}
