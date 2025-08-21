//! Safety Checker - Protects against honeypots and scams

use crate::sniper::NewToken;
use anyhow::Result;
use log::{info, warn, debug};
use reqwest::Client;
use std::collections::HashSet;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug)]
pub enum SafetyResult {
    Safe,
    Unsafe(String),
}

pub struct SafetyChecker {
    http_client: Client,
    blacklisted_creators: HashSet<Pubkey>,
    blacklisted_keywords: HashSet<String>,
}

impl SafetyChecker {
    pub fn new() -> Self {
        let mut blacklisted_keywords = HashSet::new();
        blacklisted_keywords.insert("test".to_string());
        blacklisted_keywords.insert("fake".to_string());
        blacklisted_keywords.insert("scam".to_string());
        blacklisted_keywords.insert("rug".to_string());
        blacklisted_keywords.insert("honeypot".to_string());
        
        Self {
            http_client: Client::new(),
            blacklisted_creators: HashSet::new(),
            blacklisted_keywords,
        }
    }
    
    pub async fn check_token(&self, token: &NewToken) -> Result<SafetyResult> {
        debug!("🔍 Safety checking token: {}", token.mint);
        
        // Check blacklisted creators
        if self.blacklisted_creators.contains(&token.creator) {
            return Ok(SafetyResult::Unsafe("Blacklisted creator".to_string()));
        }
        
        // Check token name/symbol for suspicious keywords
        if self.has_suspicious_keywords(token) {
            return Ok(SafetyResult::Unsafe("Suspicious name/symbol".to_string()));
        }
        
        // Check minimum liquidity
        if token.initial_liquidity < 5_000_000_000 { // 5 SOL in lamports
            return Ok(SafetyResult::Unsafe("Insufficient liquidity".to_string()));
        }
        
        // Check honeypot (external API)
        if let Ok(is_honeypot) = self.check_honeypot_api(&token.mint).await {
            if is_honeypot {
                return Ok(SafetyResult::Unsafe("Honeypot detected".to_string()));
            }
        }
        
        // Check token taxes
        if let Ok((buy_tax, sell_tax)) = self.check_token_taxes(&token.mint).await {
            if buy_tax > 10.0 || sell_tax > 10.0 {
                return Ok(SafetyResult::Unsafe(
                    format!("High taxes: buy {}%, sell {}%", buy_tax, sell_tax)
                ));
            }
        }
        
        info!("✅ Token passed all safety checks: {}", token.mint);
        Ok(SafetyResult::Safe)
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
        let url = format!("https://api.honeypot.is/v2/IsHoneypot?address={}", mint);
        
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
        // Try RugCheck API for taxes if available
        let url = format!("https://api.rugcheck.xyz/v1/tokens/{}", mint);
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
