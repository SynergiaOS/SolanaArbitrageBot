//! Memecoin Sniper Bot Binary
//! Standalone executable for sniping new memecoins

use anyhow::{Result, Context};
use clap::Parser;
use log::{info, error, warn};
use solana_arbitrage_bot::sniper::{SniperEngine, SniperConfig};
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use std::path::Path;
use std::sync::Arc;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.yaml")]
    config: String,

    /// Override RPC URL (optional)
    #[arg(long)]
    rpc_url: Option<String>,

    /// Override WebSocket URL (optional)
    #[arg(long)]
    ws_url: Option<String>,

    /// Override wallet path (optional)
    #[arg(long)]
    wallet_path: Option<String>,

    /// Run in dry-run mode (no real transactions)
    #[arg(long)]
    dry_run: bool,

    /// Maximum position size in SOL
    #[arg(long)]
    max_position: Option<f64>,

    /// Profit target percentage
    #[arg(long)]
    profit_target: Option<f64>,

    /// Stop loss percentage
    #[arg(long)]
    stop_loss: Option<f64>,

    /// Scan interval in milliseconds
    #[arg(long)]
    scan_interval: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    info!("🎯 Memecoin Sniper Bot v1.0");
    info!("⚡ Mode: {}", if args.dry_run { "DRY RUN" } else { "LIVE SNIPING" });

    // Load configuration from YAML (and CLI overrides)
    let loaded = load_config_sources(&args).context("Failed to load configuration")?;
    let config = loaded.sniper_config;

    // Load wallet
    let wallet = load_wallet(&loaded.wallet_path)?;
    let wallet_addr = wallet.pubkey();
    info!("💳 Wallet: {}", wallet_addr);

    // Check wallet balance
    let rpc_client = solana_client::rpc_client::RpcClient::new(loaded.rpc_url.clone());
    match rpc_client.get_balance(&wallet_addr) {
        Ok(balance) => {
            let sol_balance = balance as f64 / 1_000_000_000.0;
            info!("💰 Balance: {:.4} SOL", sol_balance);
            
            if sol_balance < config.max_position_sol * 2.0 {
                error!("⚠️ Low balance! Need at least {:.2} SOL for safe operation", 
                       config.max_position_sol * 2.0);
            }
        }
        Err(e) => {
            error!("❌ Failed to check balance: {}", e);
        }
    }

    if !args.dry_run {
        info!("🚨 LIVE SNIPING MODE ACTIVATED!");
        info!("💸 Max position: {} SOL per snipe", config.max_position_sol);
        info!("🎯 Profit target: {}%", config.profit_target_percent);
        info!("🛑 Stop loss: {}%", config.stop_loss_percent);
        
        // Final confirmation
        println!("\n⚠️  WARNING: This will execute REAL transactions with REAL money!");
        println!("Press Ctrl+C to abort, or wait 5 seconds to continue...\n");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }

    // Create and start sniper
    let sniper = SniperEngine::new(
        Arc::new(wallet),
        loaded.rpc_url,
        loaded.ws_url,
        config,
        args.dry_run,
    );

    info!("🚀 Starting memecoin sniper...");

    // Start sniping
    if let Err(e) = sniper.start().await {
        error!("💥 Sniper crashed: {}", e);
        return Err(e);
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct RootYamlConfig {
    rpc: Option<RpcSection>,
    wallet: Option<WalletSection>,
    sniper: Option<SniperSection>,
    safety: Option<SafetySection>,
}

#[derive(Debug, Deserialize, Clone)]
struct RpcSection { url: Option<String>, ws_url: Option<String> }

#[derive(Debug, Deserialize, Clone)]
struct WalletSection { path: Option<String> }

#[derive(Debug, Deserialize, Clone)]
struct SniperSection {
    max_position_sol: Option<f64>,
    scan_interval_ms: Option<u64>,
    max_slippage_bps: Option<u16>,
    priority_fee_lamports: Option<u64>,
    profit_target_percent: Option<f64>,
    stop_loss_percent: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
struct SafetySection {
    min_liquidity_sol: Option<f64>,
    max_market_cap_usd: Option<f64>,
    max_buy_tax_percent: Option<f64>,
    max_sell_tax_percent: Option<f64>,
}

struct LoadedConfig {
    sniper_config: SniperConfig,
    rpc_url: String,
    ws_url: String,
    wallet_path: String,
}

fn load_config_sources(args: &Args) -> Result<LoadedConfig> {
    // Defaults
    let mut rpc_url = "https://api.mainnet-beta.solana.com".to_string();
    let mut ws_url = "wss://api.mainnet-beta.solana.com".to_string();
    let mut wallet_path = "./wallet.json".to_string();

    let mut config = SniperConfig {
        max_position_sol: 0.02,
        min_liquidity_sol: 3.0,
        max_buy_tax: 5.0,
        max_sell_tax: 5.0,
        profit_target_percent: 200.0,
        stop_loss_percent: 50.0,
        max_market_cap: 100_000.0,
        scan_interval_ms: 250,
        max_slippage_bps: 150,
        priority_fee_lamports: 100_000,
    };

    // Load YAML if present
    if Path::new(&args.config).exists() {
        let content = std::fs::read_to_string(&args.config)
            .with_context(|| format!("Reading {}", &args.config))?;
        let root: RootYamlConfig = serde_yaml::from_str(&content)
            .with_context(|| format!("Parsing YAML {}", &args.config))?;

        if let Some(rpc) = root.rpc {
            if let Some(u) = rpc.url { rpc_url = u; }
            if let Some(w) = rpc.ws_url { ws_url = w; }
        }
        if let Some(w) = root.wallet {
            if let Some(p) = w.path { wallet_path = p; }
        }
        if let Some(s) = root.sniper {
            if let Some(v) = s.max_position_sol { config.max_position_sol = v; }
            if let Some(v) = s.scan_interval_ms { config.scan_interval_ms = v; }
            if let Some(v) = s.max_slippage_bps { config.max_slippage_bps = v; }
            if let Some(v) = s.priority_fee_lamports { config.priority_fee_lamports = v; }
            if let Some(v) = s.profit_target_percent { config.profit_target_percent = v; }
            if let Some(v) = s.stop_loss_percent { config.stop_loss_percent = v; }
        }
        if let Some(safe) = root.safety {
            if let Some(v) = safe.min_liquidity_sol { config.min_liquidity_sol = v; }
            if let Some(v) = safe.max_market_cap_usd { config.max_market_cap = v; }
            if let Some(v) = safe.max_buy_tax_percent { config.max_buy_tax = v; }
            if let Some(v) = safe.max_sell_tax_percent { config.max_sell_tax = v; }
        }
    } else {
        warn!("Config file not found: {}. Using defaults and CLI overrides.", &args.config);
    }

    // CLI overrides
    if let Some(s) = args.max_position { config.max_position_sol = s; }
    if let Some(s) = args.profit_target { config.profit_target_percent = s; }
    if let Some(s) = args.stop_loss { config.stop_loss_percent = s; }
    if let Some(s) = args.scan_interval { config.scan_interval_ms = s; }
    if let Some(s) = args.rpc_url.clone() { rpc_url = s; }
    if let Some(s) = args.ws_url.clone() { ws_url = s; }
    if let Some(s) = args.wallet_path.clone() { wallet_path = s; }

    info!("📋 Sniper Configuration:");
    info!("  💰 Max position: {} SOL", config.max_position_sol);
    info!("  💧 Min liquidity: {} SOL", config.min_liquidity_sol);
    info!("  📈 Profit target: {}%", config.profit_target_percent);
    info!("  📉 Stop loss: {}%", config.stop_loss_percent);
    info!("  🏭 Max market cap: ${}", config.max_market_cap);
    info!("  ⏱️  Scan interval: {}ms", config.scan_interval_ms);

    Ok(LoadedConfig { sniper_config: config, rpc_url, ws_url, wallet_path })
}

fn load_wallet(wallet_path: &str) -> Result<Keypair> {
    if !Path::new(wallet_path).exists() {
        return Err(anyhow::anyhow!("Wallet file not found: {}", wallet_path));
    }

    let wallet = read_keypair_file(wallet_path)
        .map_err(|e| anyhow::anyhow!("Failed to load wallet: {}", e))?;

    Ok(wallet)
}
