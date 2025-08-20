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
