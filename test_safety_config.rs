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
    pub max_token_age_minutes: u64,
    pub min_holders: u32,
    pub max_dev_percentage: f64,
    
    // Blacklists
    pub blacklist_mints: Vec<String>,
    pub blacklisted_creators: Vec<String>,
    pub blacklist_keywords: Vec<String>,
    
    // API Endpoints
    pub honeypot_api: Option<String>,
    pub rugcheck_api: Option<String>,
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
            enable_safety_checks: true,
        }
    }
}

fn main() {
    let config = SafetyConfig::default();
    println!("SafetyConfig created successfully: {:?}", config);
}
