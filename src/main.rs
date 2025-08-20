//! Solana Arbitrage Bot - Main Entry Point
//! Simple, fast, profitable.

use anyhow::Result;
use clap::Parser;
use log::{info, error, warn};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

mod monitor;
mod calculator;
mod executor;
mod safety;
mod ledger;
mod docx_reader;

use monitor::DexMonitor;
use calculator::ProfitCalculator;
use executor::TransactionExecutor;
use safety::SafetyGuard;
use ledger::test_ledger_connection;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run in dry-run mode (no real transactions)
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Network to use (mainnet, testnet, devnet)
    #[arg(long, default_value = "mainnet")]
    network: String,

    /// Path to config file
    #[arg(long, default_value = "./config.yaml")]
    config: String,

    /// Maximum position size in SOL (overrides config)
    #[arg(long)]
    max_position: Option<f64>,

    /// Test Ledger connection and exit
    #[arg(long, default_value_t = false)]
    test_ledger: bool,

    /// Ledger derivation path for testing
    #[arg(long, default_value = "m/44'/501'/0'/0'")]
    ledger_path: String,

    /// Read and analyze DOCX document
    #[arg(long)]
    read_docx: Option<String>,
}

#[derive(Clone)]
struct SharedState {
    raydium_price: Arc<Mutex<Option<f64>>>,
    orca_price: Arc<Mutex<Option<f64>>>,
    trades_today: Arc<Mutex<u32>>,
    profit_today: Arc<Mutex<f64>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Parse CLI arguments
    let args = Args::parse();
    
    info!("🚀 Starting Solana Arbitrage Bot");
    info!("Network: {}", args.network);
    info!("Dry run: {}", args.dry_run);
    
    // Handle DOCX reading
    if let Some(docx_path) = args.read_docx {
        info!("📄 Reading DOCX document: {}", docx_path);
        match read_docx_document(&docx_path).await {
            Ok(()) => {
                info!("✅ DOCX analysis completed successfully");
                return Ok(());
            }
            Err(e) => {
                error!("❌ DOCX reading failed: {}", e);
                return Err(e);
            }
        }
    }

    // Handle Ledger connection test
    if args.test_ledger {
        info!("🔐 Testing Ledger connection...");
        match test_ledger_connection(Some(&args.ledger_path)).await {
            Ok(()) => {
                info!("✅ Ledger test completed successfully");
                return Ok(());
            }
            Err(e) => {
                error!("❌ Ledger test failed: {}", e);
                return Err(e);
            }
        }
    }
    
    // Load configuration
    let config = load_config(&args.config)?;
    
    // Override max position if specified
    let max_position = args.max_position.unwrap_or(config.limits.max_position_sol);
    info!("Max position: {} SOL", max_position);
    
    // Initialize shared state
    let state = SharedState {
        raydium_price: Arc::new(Mutex::new(None)),
        orca_price: Arc::new(Mutex::new(None)),
        trades_today: Arc::new(Mutex::new(0)),
        profit_today: Arc::new(Mutex::new(0.0)),
    };
    
    // Initialize components
    let monitor = DexMonitor::new(
        state.raydium_price.clone(),
        state.orca_price.clone(),
        &config,
    )?;
    
    let calculator = ProfitCalculator::new(&config);
    let executor = TransactionExecutor::new(&config, args.dry_run)?;
    let safety = SafetyGuard::new(&config);
    
    // Start monitoring in background
    let monitor_handle = tokio::spawn(async move {
        monitor.start_monitoring().await
    });
    
    // Main arbitrage loop
    info!("💰 Starting arbitrage loop...");
    
    loop {
        // Check if we should continue trading
        if !safety.should_continue_trading(&state).await {
            warn!("⛔ Safety limits reached, stopping for today");
            sleep(Duration::from_secs(3600)).await; // Wait 1 hour
            safety.reset_daily_limits(&state).await;
            continue;
        }
        
        // Get current prices
        let raydium = *state.raydium_price.lock().await;
        let orca = *state.orca_price.lock().await;
        
        if let (Some(price_r), Some(price_o)) = (raydium, orca) {
            // Calculate arbitrage opportunity
            if let Some(opportunity) = calculator.calculate_opportunity(
                price_r,
                price_o,
                max_position,
            ) {
                info!(
                    "🎯 Arbitrage opportunity found! Buy {} @ {}, Sell {} @ {}, Profit: ${:.2}",
                    opportunity.buy_dex,
                    opportunity.buy_price,
                    opportunity.sell_dex,
                    opportunity.sell_price,
                    opportunity.expected_profit_usd
                );
                
                // Execute if profitable enough
                if opportunity.expected_profit_usd >= config.limits.min_profit_usd {
                    match executor.execute_arbitrage(&opportunity).await {
                        Ok(signature) => {
                            info!("✅ Trade executed! Signature: {}", signature);
                            
                            // Update statistics
                            let mut trades = state.trades_today.lock().await;
                            *trades += 1;
                            
                            let mut profit = state.profit_today.lock().await;
                            *profit += opportunity.expected_profit_usd;
                            
                            info!("📊 Daily stats: {} trades, ${:.2} profit", *trades, *profit);
                        }
                        Err(e) => {
                            error!("❌ Trade execution failed: {}", e);
                        }
                    }
                }
            }
        } else {
            // Waiting for price data
            if raydium.is_none() {
                warn!("⏳ Waiting for Raydium price data...");
            }
            if orca.is_none() {
                warn!("⏳ Waiting for Orca price data...");
            }
        }
        
        // Small delay to prevent CPU spinning
        sleep(Duration::from_millis(100)).await;
    }
}

fn load_config(path: &str) -> Result<Config> {
    let settings = config::Config::builder()
        .add_source(config::File::with_name(path))
        .build()?;
    
    Ok(settings.try_deserialize()?)
}

/// Read and analyze DOCX document for trading strategies
async fn read_docx_document(path: &str) -> Result<()> {
    use docx_reader::{DocxReader, TradingKeyword};

    info!("📄 Analyzing DOCX document: {}", path);

    // Create reader optimized for trading strategy documents
    let reader = DocxReader::for_trading_strategy()
        .with_tables(true)
        .with_metadata(true)
        .with_min_paragraph_length(15);

    // Read the document
    let content = reader.read_file(path).await?;

    // Display summary
    info!("📊 Document Summary:");
    println!("{}", content.summary());

    // Find trading-related keywords
    let keywords = reader.find_trading_keywords(&content);

    if !keywords.is_empty() {
        info!("🔍 Trading Keywords Found:");
        for keyword in keywords.iter().take(10) { // Show top 10
            println!("  • {} ({}x)", keyword.keyword, keyword.count);
            if !keyword.context.is_empty() {
                println!("    Context: \"{}\"", keyword.context[0]);
            }
        }
    }

    // Search for specific trading terms
    let important_terms = vec![
        "arbitrage", "profit", "strategy", "risk", "SOL", "USDC",
        "Raydium", "Orca", "trading", "bot"
    ];

    for term in important_terms {
        let matches = content.search(term);
        if !matches.is_empty() {
            info!("🎯 Found '{}' in {} paragraphs:", term, matches.len());
            for (i, paragraph) in matches.iter().take(3).enumerate() {
                let preview = if paragraph.len() > 100 {
                    format!("{}...", &paragraph[..100])
                } else {
                    paragraph.clone()
                };
                println!("  {}. {}", i + 1, preview);
            }
        }
    }

    // Extract potential configuration values
    info!("⚙️ Potential Configuration Values:");
    extract_config_values(&content.text);

    // Extract trading rules
    info!("📋 Potential Trading Rules:");
    extract_trading_rules(&content.paragraphs);

    Ok(())
}

/// Extract potential configuration values from text
fn extract_config_values(text: &str) {
    use regex::Regex;

    println!("  📊 Numerical Values Found:");

    // Look for percentage values
    if let Ok(percent_regex) = Regex::new(r"(\d+(?:\.\d+)?)\s*%") {
        let mut percentages = Vec::new();
        for cap in percent_regex.captures_iter(text) {
            if let Some(value) = cap.get(1) {
                percentages.push(value.as_str());
            }
        }
        if !percentages.is_empty() {
            println!("    • Percentages: {}", percentages.join(", "));
        }
    }

    // Look for dollar amounts
    if let Ok(dollar_regex) = Regex::new(r"\$(\d+(?:\.\d+)?)") {
        let mut dollars = Vec::new();
        for cap in dollar_regex.captures_iter(text) {
            if let Some(value) = cap.get(1) {
                dollars.push(format!("${}", value.as_str()));
            }
        }
        if !dollars.is_empty() {
            println!("    • Dollar amounts: {}", dollars.join(", "));
        }
    }

    // Look for SOL amounts
    if let Ok(sol_regex) = Regex::new(r"(\d+(?:\.\d+)?)\s*SOL") {
        let mut sol_amounts = Vec::new();
        for cap in sol_regex.captures_iter(text) {
            if let Some(value) = cap.get(1) {
                sol_amounts.push(format!("{} SOL", value.as_str()));
            }
        }
        if !sol_amounts.is_empty() {
            println!("    • SOL amounts: {}", sol_amounts.join(", "));
        }
    }

    // Look for USDC amounts
    if let Ok(usdc_regex) = Regex::new(r"(\d+(?:\.\d+)?)\s*USDC") {
        let mut usdc_amounts = Vec::new();
        for cap in usdc_regex.captures_iter(text) {
            if let Some(value) = cap.get(1) {
                usdc_amounts.push(format!("{} USDC", value.as_str()));
            }
        }
        if !usdc_amounts.is_empty() {
            println!("    • USDC amounts: {}", usdc_amounts.join(", "));
        }
    }

    // Look for time values
    if let Ok(time_regex) = Regex::new(r"(\d+)\s*(second|minute|hour|day)s?") {
        let mut times = Vec::new();
        for cap in time_regex.captures_iter(text) {
            if let (Some(value), Some(unit)) = (cap.get(1), cap.get(2)) {
                times.push(format!("{} {}", value.as_str(), unit.as_str()));
            }
        }
        if !times.is_empty() {
            println!("    • Time values: {}", times.join(", "));
        }
    }

    // Look for addresses (Solana public keys)
    if let Ok(address_regex) = Regex::new(r"[1-9A-HJ-NP-Za-km-z]{32,44}") {
        let mut addresses = Vec::new();
        for cap in address_regex.captures_iter(text) {
            let addr = cap.get(0).unwrap().as_str();
            if addr.len() >= 32 && addr.len() <= 44 {
                addresses.push(format!("{}...{}", &addr[..8], &addr[addr.len()-4..]));
            }
        }
        if !addresses.is_empty() && addresses.len() <= 10 {
            println!("    • Potential addresses: {}", addresses.join(", "));
        }
    }
}

/// Extract trading rules from paragraphs
fn extract_trading_rules(paragraphs: &[String]) {
    let rule_keywords = vec![
        "must", "should", "never", "always", "if", "when", "limit",
        "maximum", "minimum", "stop", "exit", "enter"
    ];

    for paragraph in paragraphs {
        let lower = paragraph.to_lowercase();
        let rule_count = rule_keywords.iter()
            .filter(|&&keyword| lower.contains(keyword))
            .count();

        if rule_count >= 2 { // Paragraph contains multiple rule keywords
            let preview = if paragraph.len() > 150 {
                format!("{}...", &paragraph[..150])
            } else {
                paragraph.clone()
            };
            println!("  • {}", preview);
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct Config {
    rpc: RpcConfig,
    wallet: WalletConfig,
    dex: DexConfig,
    limits: LimitsConfig,
    execution: ExecutionConfig,
}

#[derive(Debug, serde::Deserialize)]
struct RpcConfig {
    url: String,
    ws_url: String,
}

#[derive(Debug, serde::Deserialize)]
struct WalletConfig {
    path: String,
    use_ledger: Option<bool>,
    ledger_path: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct DexConfig {
    raydium: DexInfo,
    orca: DexInfo,
}

#[derive(Debug, serde::Deserialize)]
struct DexInfo {
    program_id: String,
    sol_usdc_pool: String,
}

#[derive(Debug, serde::Deserialize)]
struct LimitsConfig {
    max_position_sol: f64,
    min_profit_percent: f64,
    min_profit_usd: f64,
    max_slippage_percent: f64,
    max_daily_loss_usd: f64,
    max_daily_trades: u32,
}

#[derive(Debug, serde::Deserialize)]
struct ExecutionConfig {
    priority_fee_lamports: u64,
    simulation_required: bool,
    max_retries: u32,
}
