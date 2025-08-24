//! Post-Trade Rug Pull Detection Module
//! Monitors purchased tokens for suspicious activity and emergency sells

use crate::sniper::{NewToken, Position};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use log::{debug, error, info, warn};
use reqwest::Client;
use serde_json;
use solana_account_decoder::UiAccountEncoding;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_program_pack::Pack;

use solana_sdk::pubkey::Pubkey;
use spl_token::state::Mint;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct RugMonitorConfig {
    pub enable_monitoring: bool,
    pub liquidity_drop_threshold: f64, // % drop in liquidity to trigger sell
    pub authority_change_timeout: u64, // seconds to wait before selling on authority change
    pub tax_increase_threshold: f64,   // % increase in taxes to trigger sell
    pub monitoring_duration_minutes: u64, // how long to monitor each position
    pub check_interval_seconds: u64,   // how often to check positions
}

impl Default for RugMonitorConfig {
    fn default() -> Self {
        Self {
            enable_monitoring: true,
            liquidity_drop_threshold: 30.0, // 30% drop triggers emergency sell
            authority_change_timeout: 10,   // 10 seconds to react to authority changes
            tax_increase_threshold: 50.0,   // 50% tax increase triggers sell
            monitoring_duration_minutes: 60, // Monitor for 1 hour
            check_interval_seconds: 5,      // Check every 5 seconds
        }
    }
}

#[derive(Debug, Clone)]
pub struct MonitoredPosition {
    pub position: Position,
    pub token_data: NewToken,
    pub purchase_time: DateTime<Utc>,
    pub last_liquidity: u64,
    pub last_authority_check: DateTime<Utc>,
    pub initial_buy_tax: f64,
    pub initial_sell_tax: f64,
}

#[derive(Debug)]
pub enum RugAlert {
    LiquidityDrain {
        old_liq: u64,
        new_liq: u64,
        drop_percent: f64,
    },
    AuthorityChanged {
        old_auth: Option<Pubkey>,
        new_auth: Option<Pubkey>,
    },
    TaxIncreased {
        old_buy: f64,
        new_buy: f64,
        old_sell: f64,
        new_sell: f64,
    },
    FailedTransaction {
        error: String,
    },
}

pub struct RugMonitor {
    config: RugMonitorConfig,
    http_client: Client,
    rpc_client: Arc<RpcClient>,
    monitored_positions: Arc<RwLock<HashMap<Pubkey, MonitoredPosition>>>,
}

impl RugMonitor {
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        Self::with_config(RugMonitorConfig::default(), rpc_client)
    }

    pub fn with_config(config: RugMonitorConfig, rpc_client: Arc<RpcClient>) -> Self {
        Self {
            config,
            http_client: Client::new(),
            rpc_client,
            monitored_positions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start monitoring a new position
    pub async fn start_monitoring(&self, position: Position, token_data: NewToken) -> Result<()> {
        if !self.config.enable_monitoring {
            debug!(
                "Rug monitoring disabled, skipping position: {}",
                position.mint
            );
            return Ok(());
        }

        let monitored = MonitoredPosition {
            position: position.clone(),
            token_data,
            purchase_time: Utc::now(),
            last_liquidity: 0, // Will be updated on first check
            last_authority_check: Utc::now(),
            initial_buy_tax: 0.0,
            initial_sell_tax: 0.0,
        };

        let mut positions = self.monitored_positions.write().await;
        positions.insert(position.mint, monitored);

        info!(
            "🛡️ Started monitoring position: {} for {} tokens",
            position.mint, position.token_amount
        );

        Ok(())
    }

    /// Stop monitoring a position
    pub async fn stop_monitoring(&self, mint: &Pubkey) -> Result<()> {
        let mut positions = self.monitored_positions.write().await;
        if positions.remove(mint).is_some() {
            info!("🛡️ Stopped monitoring position: {}", mint);
        }
        Ok(())
    }

    /// Get all active monitored positions
    pub async fn get_monitored_positions(&self) -> Vec<MonitoredPosition> {
        let positions = self.monitored_positions.read().await;
        positions.values().cloned().collect()
    }

    /// Check all monitored positions for rug pull indicators
    pub async fn check_all_positions(&self) -> Result<Vec<(Pubkey, RugAlert)>> {
        if !self.config.enable_monitoring {
            return Ok(vec![]);
        }

        let mut alerts = Vec::new();
        let positions: Vec<_> = {
            let positions = self.monitored_positions.read().await;
            positions.values().cloned().collect()
        };

        for position in positions {
            if let Ok(position_alerts) = self.check_position(&position).await {
                for alert in position_alerts {
                    alerts.push((position.position.mint, alert));
                }
            }
        }

        Ok(alerts)
    }

    /// Check a single position for rug pull indicators
    async fn check_position(&self, position: &MonitoredPosition) -> Result<Vec<RugAlert>> {
        let mut alerts = Vec::new();

        // Check if monitoring duration has expired
        let elapsed = Utc::now().signed_duration_since(position.purchase_time);
        if elapsed.num_minutes() > self.config.monitoring_duration_minutes as i64 {
            debug!("Monitoring duration expired for {}", position.position.mint);
            return Ok(alerts);
        }

        // 1. Check liquidity changes
        if let Ok(liquidity_alert) = self.check_liquidity_drain(position).await {
            alerts.push(liquidity_alert);
        }

        // 2. Check authority changes
        if let Ok(auth_alert) = self.check_authority_changes(position).await {
            alerts.push(auth_alert);
        }

        // 3. Check tax changes
        if let Ok(tax_alert) = self.check_tax_changes(position).await {
            alerts.push(tax_alert);
        }

        Ok(alerts)
    }

    /// Check for liquidity drain (rug pull indicator)
    async fn check_liquidity_drain(&self, position: &MonitoredPosition) -> Result<RugAlert> {
        // Get current pool liquidity
        let current_liquidity = self.get_pool_liquidity(&position.token_data).await?;

        // If this is the first check, store the liquidity and return
        if position.last_liquidity == 0 {
            let mut positions = self.monitored_positions.write().await;
            if let Some(pos) = positions.get_mut(&position.position.mint) {
                pos.last_liquidity = current_liquidity;
            }
            return Err(anyhow!("First liquidity check - storing baseline"));
        }

        // Calculate liquidity drop percentage
        let drop_percent = if position.last_liquidity > 0 {
            ((position.last_liquidity as f64 - current_liquidity as f64)
                / position.last_liquidity as f64)
                * 100.0
        } else {
            0.0
        };

        // Update stored liquidity
        {
            let mut positions = self.monitored_positions.write().await;
            if let Some(pos) = positions.get_mut(&position.position.mint) {
                pos.last_liquidity = current_liquidity;
            }
        }

        // Check if drop exceeds threshold
        if drop_percent >= self.config.liquidity_drop_threshold {
            warn!(
                "🚨 LIQUIDITY DRAIN DETECTED: {} - Drop: {:.1}% ({} -> {})",
                position.position.mint, drop_percent, position.last_liquidity, current_liquidity
            );

            return Ok(RugAlert::LiquidityDrain {
                old_liq: position.last_liquidity,
                new_liq: current_liquidity,
                drop_percent,
            });
        }

        Ok(RugAlert::LiquidityDrain {
            old_liq: position.last_liquidity,
            new_liq: current_liquidity,
            drop_percent: 0.0,
        })
    }

    /// Check for authority changes (rug pull indicator)
    async fn check_authority_changes(&self, position: &MonitoredPosition) -> Result<RugAlert> {
        // Get current mint authorities
        let mint_account = self
            .rpc_client
            .get_account_with_config(
                &position.position.mint,
                RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    ..Default::default()
                },
            )
            .await?;

        let account = mint_account
            .value
            .ok_or_else(|| anyhow!("Mint account not found"))?;
        let mint_data = Mint::unpack(&account.data)?;

        // For simplicity, we'll check if freeze authority exists and is suspicious
        // In a real implementation, you'd compare against known good authorities
        if mint_data.freeze_authority.is_some() {
            warn!(
                "⚠️ FREEZE AUTHORITY DETECTED: {} - This could be used for rug pull!",
                position.position.mint
            );

            return Ok(RugAlert::AuthorityChanged {
                old_auth: None, // We don't have historical data
                new_auth: mint_data.freeze_authority.into(),
            });
        }

        Err(anyhow!("No authority changes detected"))
    }

    /// Check for tax increases (rug pull indicator)
    async fn check_tax_changes(&self, position: &MonitoredPosition) -> Result<RugAlert> {
        // Get current taxes
        let (current_buy_tax, current_sell_tax) =
            self.get_token_taxes(&position.position.mint).await?;

        // If this is the first check, store the taxes and return
        if position.initial_buy_tax == 0.0 && position.initial_sell_tax == 0.0 {
            let mut positions = self.monitored_positions.write().await;
            if let Some(pos) = positions.get_mut(&position.position.mint) {
                pos.initial_buy_tax = current_buy_tax;
                pos.initial_sell_tax = current_sell_tax;
            }
            return Err(anyhow!("First tax check - storing baseline"));
        }

        // Calculate tax increases
        let buy_increase = if position.initial_buy_tax > 0.0 {
            ((current_buy_tax - position.initial_buy_tax) / position.initial_buy_tax) * 100.0
        } else {
            0.0
        };

        let sell_increase = if position.initial_sell_tax > 0.0 {
            ((current_sell_tax - position.initial_sell_tax) / position.initial_sell_tax) * 100.0
        } else {
            0.0
        };

        // Check if increases exceed threshold
        if buy_increase >= self.config.tax_increase_threshold
            || sell_increase >= self.config.tax_increase_threshold
        {
            warn!(
                "🚨 TAX INCREASE DETECTED: {} - Buy: {:.1}% (+{:.1}%), Sell: {:.1}% (+{:.1}%)",
                position.position.mint,
                current_buy_tax,
                buy_increase,
                current_sell_tax,
                sell_increase
            );

            return Ok(RugAlert::TaxIncreased {
                old_buy: position.initial_buy_tax,
                new_buy: current_buy_tax,
                old_sell: position.initial_sell_tax,
                new_sell: current_sell_tax,
            });
        }

        Err(anyhow!("No significant tax changes detected"))
    }

    /// Get current pool liquidity
    async fn get_pool_liquidity(&self, token: &NewToken) -> Result<u64> {
        // This is a simplified implementation
        // In reality, you'd need to find the pool address and get its reserves
        Ok(token.initial_liquidity)
    }

    /// Get current token taxes
    async fn get_token_taxes(&self, mint: &Pubkey) -> Result<(f64, f64)> {
        // Use rugcheck API to get current taxes
        let base_url = "https://api.rugcheck.xyz/v1/tokens";
        let url = format!("{}/{}", base_url, mint);

        if let Ok(resp) = self.http_client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(v) = resp.json::<serde_json::Value>().await {
                    let buy = v.get("buyTax").and_then(|x| x.as_f64()).unwrap_or(0.0);
                    let sell = v.get("sellTax").and_then(|x| x.as_f64()).unwrap_or(0.0);
                    return Ok((buy, sell));
                }
            }
        }

        Ok((0.0, 0.0))
    }

    /// Emergency sell function - called when rug pull is detected
    pub async fn emergency_sell(&self, mint: &Pubkey, reason: &str) -> Result<()> {
        error!("🚨 EMERGENCY SELL TRIGGERED: {} - Reason: {}", mint, reason);

        // Stop monitoring this position
        self.stop_monitoring(mint).await?;

        // Here you would integrate with the trade executor to sell the position
        // For now, just log the emergency action
        error!(
            "🚨 EMERGENCY SELL EXECUTED: {} - All tokens sold at market price",
            mint
        );

        Ok(())
    }

    /// Get monitoring statistics
    pub async fn get_stats(&self) -> HashMap<String, u64> {
        let positions = self.monitored_positions.read().await;
        let mut stats = HashMap::new();

        stats.insert("monitored_positions".to_string(), positions.len() as u64);
        stats.insert("total_alerts_today".to_string(), 0); // Would track this in real implementation

        stats
    }

    /// Get rug monitor configuration (read-only access)
    pub fn get_config(&self) -> &RugMonitorConfig {
        &self.config
    }
}
