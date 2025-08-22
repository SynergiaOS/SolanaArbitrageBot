//! Safety Checker - Protects against honeypots and scams

use crate::sniper::{NewToken, SafetyConfig};
use anyhow::{Result, anyhow};
use log::{info, warn, debug};
use reqwest::Client;
use std::collections::HashSet;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_config::RpcAccountInfoConfig;
use spl_token::state::{Mint, Account as TokenAccount};
use solana_sdk::account::Account;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_program_pack::Pack;
use std::sync::Arc;
use serde_json;

#[derive(Debug)]
pub enum SafetyResult {
    Safe,
    Unsafe(String),
}

pub struct SafetyChecker {
    config: SafetyConfig,
    http_client: Client,
    rpc_client: Arc<RpcClient>,
    blacklisted_creators: HashSet<Pubkey>,
    blacklisted_keywords: HashSet<String>,
}

impl SafetyChecker {
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        Self::from_config(&SafetyConfig::default(), rpc_client)
    }

    pub fn from_config(config: &SafetyConfig, rpc_client: Arc<RpcClient>) -> Self {
        // Konwertuj blacklisted_creators z String na Pubkey
        let mut blacklisted_creators = HashSet::new();
        for creator_str in &config.blacklisted_creators {
            if let Ok(pubkey) = Pubkey::from_str(creator_str) {
                blacklisted_creators.insert(pubkey);
            } else {
                warn!("Invalid creator pubkey in config: {}", creator_str);
            }
        }

        // Konwertuj blacklist_keywords na lowercase HashSet
        let blacklisted_keywords: HashSet<String> = config.blacklist_keywords
            .iter()
            .map(|k| k.to_lowercase())
            .collect();

        Self {
            config: config.clone(),
            http_client: Client::new(),
            rpc_client,
            blacklisted_creators,
            blacklisted_keywords,
        }
    }
    
    pub async fn check_token(&self, token: &NewToken) -> Result<SafetyResult> {
        debug!("🔍 Safety checking token: {}", token.mint);

        // Jeśli safety checks są wyłączone, zwróć Safe
        if !self.config.enable_safety_checks {
            debug!("⚠️ Safety checks disabled, skipping all checks");
            return Ok(SafetyResult::Safe);
        }
        
        // Check blacklisted creators
        if self.blacklisted_creators.contains(&token.creator) {
            return Ok(SafetyResult::Unsafe("Blacklisted creator".to_string()));
        }
        
        // Check token name/symbol for suspicious keywords
        if self.has_suspicious_keywords(token) {
            return Ok(SafetyResult::Unsafe("Suspicious name/symbol".to_string()));
        }
        
        // Check minimum liquidity (konwertuj SOL na lamports)
        let min_liquidity_lamports = (self.config.min_liquidity_sol * 1_000_000_000.0) as u64;
        if token.initial_liquidity < min_liquidity_lamports {
            return Ok(SafetyResult::Unsafe(
                format!("Insufficient liquidity: {} < {} SOL",
                    token.initial_liquidity as f64 / 1_000_000_000.0,
                    self.config.min_liquidity_sol)
            ));
        }
        
        // Check honeypot (external API)
        if let Ok(is_honeypot) = self.check_honeypot_api(&token.mint).await {
            if is_honeypot {
                return Ok(SafetyResult::Unsafe("Honeypot detected".to_string()));
            }
        }
        
        // Check token taxes (używaj progów z config)
        if let Ok((buy_tax, sell_tax)) = self.check_token_taxes(&token.mint).await {
            if buy_tax > self.config.max_buy_tax_percent || sell_tax > self.config.max_sell_tax_percent {
                return Ok(SafetyResult::Unsafe(
                    format!("High taxes: buy {}% (max {}%), sell {}% (max {}%)",
                        buy_tax, self.config.max_buy_tax_percent,
                        sell_tax, self.config.max_sell_tax_percent)
                ));
            }
        }
        
        debug!("🔄 Passed basic safety checks, running advanced checks...");
        // Check market cap (jeśli skonfigurowane)
        if self.config.max_market_cap_usd > 0.0 {
            if let Ok(market_cap) = self.calculate_market_cap(token).await {
                if market_cap > self.config.max_market_cap_usd {
                    return Ok(SafetyResult::Unsafe(
                        format!("Market cap too high: ${:.0} (max ${:.0})",
                            market_cap, self.config.max_market_cap_usd)
                    ));
                }
                debug!("✅ Market cap check passed: ${:.0}", market_cap);
            } else {
                warn!("⚠️ Could not calculate market cap for {}", token.mint);
            }
        }

        // Check holder count (jeśli skonfigurowane)
        if self.config.min_holders > 0 {
            if let Ok(holder_count) = self.get_token_holders(token).await {
                if holder_count < self.config.min_holders {
                    return Ok(SafetyResult::Unsafe(
                        format!("Too few holders: {} (min {})",
                            holder_count, self.config.min_holders)
                    ));
                }
                debug!("✅ Holder count check passed: {} holders", holder_count);
            } else {
                warn!("⚠️ Could not get holder count for {}", token.mint);
            }
        }

        // Check dev percentage (jeśli skonfigurowane)
        if self.config.max_dev_percentage > 0.0 {
            if let Ok(dev_percentage) = self.check_dev_percentage(token).await {
                if dev_percentage > self.config.max_dev_percentage {
                    return Ok(SafetyResult::Unsafe(
                        format!("Dev concentration too high: {:.1}% (max {:.1}%)",
                            dev_percentage, self.config.max_dev_percentage)
                    ));
                }
                debug!("✅ Dev percentage check passed: {:.1}%", dev_percentage);
            } else {
                warn!("⚠️ Could not check dev percentage for {}", token.mint);
            }
        }

        // Check token age (jeśli skonfigurowane)
        if self.config.max_token_age_minutes > 0 {
            if let Ok(age_minutes) = self.get_token_age(token).await {
                if age_minutes > self.config.max_token_age_minutes {
                    return Ok(SafetyResult::Unsafe(
                        format!("Token too old: {} minutes (max {} minutes)",
                            age_minutes, self.config.max_token_age_minutes)
                    ));
                }
                debug!("✅ Token age check passed: {} minutes", age_minutes);
            } else {
                warn!("⚠️ Could not determine token age for {}", token.mint);
            }
        }

        info!("✅ Token passed ALL safety checks: {}", token.mint);
        Ok(SafetyResult::Safe)
    }

    /// Calculate market cap from token supply and pool price
    async fn calculate_market_cap(&self, token: &NewToken) -> Result<f64> {
        debug!("📊 Calculating market cap for token: {}", token.mint);

        // 1. Pobierz mint account dla total supply
        let mint_account = self.rpc_client
            .get_account_with_config(
                &token.mint,
                RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    ..Default::default()
                }
            )
            .await?;

        // 2. Deserializuj mint data
        let account = mint_account.value.ok_or_else(|| anyhow!("Mint account not found"))?;
        let mint_data = Mint::unpack(&account.data)?;
        let total_supply = mint_data.supply as f64 / 10_f64.powi(mint_data.decimals as i32);

        debug!("📈 Total supply: {:.2} tokens", total_supply);

        // 3. Wylicz cenę z pool reserves (używamy liquidity_sol i liquidity_token)
        let price_per_token = if token.liquidity_token > 0.0 {
            token.liquidity_sol / token.liquidity_token
        } else {
            return Err(anyhow!("Invalid token liquidity: {}", token.liquidity_token));
        };

        debug!("💰 Price per token: ${:.8}", price_per_token);

        // 4. Market Cap = Total Supply × Price per Token × SOL Price (assume $100 for now)
        let sol_price_usd = 100.0; // TODO: Get real SOL price from API
        let market_cap = total_supply * price_per_token * sol_price_usd;

        debug!("🎯 Market cap calculated: ${:.0}", market_cap);
        Ok(market_cap)
    }

    /// Get token holder count using Helius Enhanced API or RPC fallback
    async fn get_token_holders(&self, token: &NewToken) -> Result<u32> {
        debug!("👥 Getting holder count for token: {}", token.mint);

        // Try Helius Enhanced API first (if configured)
        if let Some(helius_api_key) = &self.config.helius_api_key {
            if let Ok(count) = self.get_holders_helius(&token.mint, helius_api_key).await {
                return Ok(count);
            }
            warn!("⚠️ Helius API failed, falling back to RPC");
        }

        // Fallback to RPC getProgramAccounts
        self.get_holders_rpc(&token.mint).await
    }

    /// Get holder count using Helius Enhanced API
    async fn get_holders_helius(&self, mint: &Pubkey, api_key: &str) -> Result<u32> {
        let url = format!("https://api.helius.xyz/v0/tokens/{}/holders?api-key={}", mint, api_key);

        let response = self.http_client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Helius API error: {}", response.status()));
        }

        let json: serde_json::Value = response.json().await?;

        // Parse holder count from response
        if let Some(holders) = json.get("holders").and_then(|h| h.as_array()) {
            let unique_holders = holders.len() as u32;
            debug!("📊 Helius API: {} unique holders", unique_holders);
            Ok(unique_holders)
        } else {
            Err(anyhow!("Invalid Helius API response format"))
        }
    }

    /// Get holder count using RPC getProgramAccounts (fallback)
    async fn get_holders_rpc(&self, mint: &Pubkey) -> Result<u32> {
        use solana_client::rpc_config::{RpcProgramAccountsConfig, RpcAccountInfoConfig};
        use solana_client::rpc_filter::{RpcFilterType, Memcmp};
        use spl_token::state::Account as TokenAccount;

        debug!("🔍 Using RPC fallback for holder count");

        // Get all token accounts for this mint
        let config = RpcProgramAccountsConfig {
            filters: Some(vec![
                RpcFilterType::DataSize(TokenAccount::LEN as u64),
                RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, mint.to_bytes().to_vec())),
            ]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..Default::default()
            },
            ..Default::default()
        };

        let accounts = self.rpc_client
            .get_program_accounts_with_config(&spl_token::id(), config)
            .await?;

        // Count accounts with non-zero balance
        let mut holder_count = 0;
        for (_, account) in accounts {
            if let Ok(token_account) = TokenAccount::unpack(&account.data) {
                if token_account.amount > 0 {
                    holder_count += 1;
                }
            }
        }

        debug!("📊 RPC: {} holders with non-zero balance", holder_count);
        Ok(holder_count)
    }

    /// Check dev percentage by analyzing top holders
    async fn check_dev_percentage(&self, token: &NewToken) -> Result<f64> {
        debug!("🔍 Checking dev percentage for token: {}", token.mint);

        // Get top holders with balances
        let top_holders = self.get_top_holders(&token.mint, 10).await?;

        if top_holders.is_empty() {
            return Ok(0.0);
        }

        // Calculate total supply
        let mint_account = self.rpc_client
            .get_account_with_config(
                &token.mint,
                RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    ..Default::default()
                }
            )
            .await?;

        let account = mint_account.value.ok_or_else(|| anyhow!("Mint account not found"))?;
        let mint_data = Mint::unpack(&account.data)?;
        let total_supply = mint_data.supply;

        // Find the largest holder percentage
        let max_holder_balance = top_holders[0].1; // (pubkey, balance)
        let max_percentage = (max_holder_balance as f64 / total_supply as f64) * 100.0;

        debug!("📊 Top holder has {:.1}% of supply", max_percentage);

        // Additional heuristics for dev detection:
        // 1. If top holder has >50% = likely dev
        // 2. If top 3 holders combined have >80% = likely dev team
        let top3_combined = top_holders.iter().take(3).map(|(_, balance)| *balance).sum::<u64>();
        let top3_percentage = (top3_combined as f64 / total_supply as f64) * 100.0;

        let dev_percentage = if max_percentage > 50.0 {
            max_percentage // Single dev wallet
        } else if top3_percentage > 80.0 {
            top3_percentage // Dev team wallets
        } else {
            max_percentage // Conservative estimate
        };

        debug!("🎯 Estimated dev percentage: {:.1}%", dev_percentage);
        Ok(dev_percentage)
    }

    /// Get top token holders with their balances
    async fn get_top_holders(&self, mint: &Pubkey, limit: usize) -> Result<Vec<(Pubkey, u64)>> {
        use solana_client::rpc_config::{RpcProgramAccountsConfig, RpcAccountInfoConfig};
        use solana_client::rpc_filter::{RpcFilterType, Memcmp};
        use spl_token::state::Account as TokenAccount;

        let config = RpcProgramAccountsConfig {
            filters: Some(vec![
                RpcFilterType::DataSize(TokenAccount::LEN as u64),
                RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, mint.to_bytes().to_vec())),
            ]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                ..Default::default()
            },
            ..Default::default()
        };

        let accounts = self.rpc_client
            .get_program_accounts_with_config(&spl_token::id(), config)
            .await?;

        // Parse accounts and collect balances
        let mut holders: Vec<(Pubkey, u64)> = Vec::new();
        for (pubkey, account) in accounts {
            if let Ok(token_account) = TokenAccount::unpack(&account.data) {
                if token_account.amount > 0 {
                    holders.push((pubkey, token_account.amount));
                }
            }
        }

        // Sort by balance (descending) and take top N
        holders.sort_by(|a, b| b.1.cmp(&a.1));
        holders.truncate(limit);

        debug!("📊 Found {} top holders", holders.len());
        Ok(holders)
    }

    /// Get token age in minutes since creation
    async fn get_token_age(&self, token: &NewToken) -> Result<u32> {
        debug!("⏰ Getting token age for: {}", token.mint);

        // Method 1: Try to get mint account creation slot
        if let Ok(age) = self.get_token_age_from_mint(&token.mint).await {
            return Ok(age);
        }

        // Method 2: Fallback to pool creation time (from token data)
        if let Ok(age) = self.get_token_age_from_pool(token).await {
            return Ok(age);
        }

        // Method 3: Conservative fallback - assume very new
        warn!("⚠️ Could not determine token age, assuming 0 minutes");
        Ok(0)
    }

    /// Get token age from mint account creation slot
    async fn get_token_age_from_mint(&self, mint: &Pubkey) -> Result<u32> {
        // Get account info with context (includes slot)
        let account_info = self.rpc_client
            .get_account_with_commitment(mint, CommitmentConfig::confirmed())
            .await?;

        if let Some(account) = account_info.value {
            // Get current slot
            let current_slot = self.rpc_client.get_slot().await?;

            // Estimate age based on slot difference
            // Solana produces ~2.5 slots per second on average
            let slot_diff = current_slot.saturating_sub(account_info.context.slot);
            let age_seconds = (slot_diff as f64 / 2.5) as u32;
            let age_minutes = age_seconds / 60;

            debug!("📅 Token age from mint: {} minutes (slot diff: {})", age_minutes, slot_diff);
            Ok(age_minutes)
        } else {
            Err(anyhow!("Mint account not found"))
        }
    }

    /// Get token age from pool creation time (fallback)
    async fn get_token_age_from_pool(&self, token: &NewToken) -> Result<u32> {
        // This is a simplified approach - in reality you'd need to:
        // 1. Find the pool creation transaction
        // 2. Get the transaction timestamp
        // 3. Calculate age from that timestamp

        // For now, use a heuristic based on liquidity
        // New tokens typically have lower liquidity
        let estimated_age = if token.liquidity_sol < 1.0 {
            5 // Very new, assume 5 minutes
        } else if token.liquidity_sol < 10.0 {
            30 // Relatively new, assume 30 minutes
        } else {
            120 // Older token, assume 2 hours
        };

        debug!("📅 Token age from pool heuristic: {} minutes", estimated_age);
        Ok(estimated_age)
    }
    
    fn has_suspicious_keywords(&self, token: &NewToken) -> bool {
        let text = format!("{} {}", token.name.to_lowercase(), token.symbol.to_lowercase());
        
        for keyword in &self.blacklisted_keywords {
            if text.contains(keyword) {
                warn!("Suspicious keyword '{}' found in token: {}", keyword, token.mint);
                return true;
            }
        }
        
        false
    }
    
    async fn check_honeypot_api(&self, mint: &Pubkey) -> Result<bool> {
        // Użyj URL z config lub fallback na domyślny
        let base_url = self.config.honeypot_api.as_ref()
            .map(|s| s.as_str())
            .unwrap_or("https://api.honeypot.is/v2/IsHoneypot");
        let url = format!("{}?address={}", base_url, mint);
        
        match self.http_client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let data: serde_json::Value = response.json().await?;
                    
                    if let Some(is_honeypot) = data.get("IsHoneypot").and_then(|v| v.as_bool()) {
                        return Ok(is_honeypot);
                    }
                }
            }
            Err(e) => {
                debug!("Honeypot API error: {}", e);
            }
        }
        
        // Default to safe if API fails
        Ok(false)
    }
    
    async fn check_token_taxes(&self, mint: &Pubkey) -> Result<(f64, f64)> {
        // Użyj URL z config lub fallback na domyślny
        let base_url = self.config.rugcheck_api.as_ref()
            .map(|s| s.as_str())
            .unwrap_or("https://api.rugcheck.xyz/v1/tokens");
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
    
    pub fn add_blacklisted_creator(&mut self, creator: Pubkey) {
        self.blacklisted_creators.insert(creator);
    }
    
    pub fn add_blacklisted_keyword(&mut self, keyword: String) {
        self.blacklisted_keywords.insert(keyword.to_lowercase());
    }
}
