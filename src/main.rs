//! Solana Arbitrage Bot - Main Entry Point
//! Real-time arbitrage trading with Jupiter integration

use anyhow::Result;
use clap::Parser;
use log::{info, error, warn};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use rust_decimal::Decimal;
use solana_arbitrage_bot::utils::conversions::*;

// Import from lib
use solana_arbitrage_bot::*;

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
    
    /// Test real API connections and exit
    #[arg(long, default_value_t = false)]
    test_apis: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Parse CLI arguments
    let args = Args::parse();
    
    info!("🚀 Starting Solana Arbitrage Bot v2.0");
    info!("Network: {}", args.network);
    info!("Mode: {}", if args.dry_run { "DRY RUN" } else { "LIVE TRADING" });
    
    // Handle test modes
    if args.test_ledger {
        info!("🔐 Testing Ledger connection...");
        match ledger::test_ledger_connection(Some(&args.ledger_path)).await {
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
    
    if args.test_apis {
        info!("🌐 Testing API connections...");
        test_api_connections().await?;
        return Ok(());
    }
    
    // Load configuration
    let config = load_config(&args.config)?;
    
    // Override max position if specified - kept for CLI, but runtime config may override later
    let cli_max_position = args.max_position;
    if let Some(v) = cli_max_position { info!("CLI override max position: {} SOL", v); }

    // Initialize shared state
    let state = SharedState {
        raydium_price: Arc::new(Mutex::new(None)),
        orca_price: Arc::new(Mutex::new(None)),
        trades_today: Arc::new(Mutex::new(0)),
        profit_today: Arc::new(Mutex::new(Decimal::ZERO)),
    };
    
    // Create price update channel
    let (price_tx, mut price_rx) = tokio::sync::mpsc::channel::<monitor::PriceUpdate>(100);
    
    // Initialize components
    let monitor = monitor::DexMonitor::new(
        state.raydium_price.clone(),
        state.orca_price.clone(),
        &config,
    )?
    .with_price_channel(price_tx);
    
    let mut calculator = calculator::ProfitCalculator::new(&config);
    let executor = executor::TransactionExecutor::new(&config, args.dry_run)?;
    let mut safety = safety::SafetyGuard::new(&config);

    // Setup Discord alerts (ENV overrides)
    // Prefer DISCORD_WEBHOOK_URL and DISCORD_ENABLED over config file values
    let webhook_env = std::env::var("DISCORD_WEBHOOK_URL").ok();
    let enabled_env = std::env::var("DISCORD_ENABLED")
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "True"));

    let discord = if webhook_env.is_some() || config.discord.is_some() {
        let webhook = webhook_env
            .or_else(|| config.discord.as_ref().map(|c| c.webhook_url.clone()))
            .unwrap_or_default();
        let enabled = enabled_env
            .unwrap_or_else(|| config.discord.as_ref().map(|c| c.enabled).unwrap_or(false));
        Some(discord::DiscordAlert::new(webhook, enabled))
    } else {
        None
    };

    // Setup Web Dashboard
    let web_server = if let Some(web_config) = &config.web {
        if web_config.enabled {
            match web::WebServer::new(state.clone(), web_config.clone(), discord.clone(), &config).await {
                Ok(server) => {
                    info!("🌐 Web dashboard initialized");
                    Some(server)
                }
                Err(e) => {
                    error!("Failed to initialize web dashboard: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // Send startup alert
    if let Some(ref discord_alert) = discord {
        let wallet_addr = executor.get_wallet_address();

        if let Err(e) = discord_alert.send_startup_alert(
            &wallet_addr,
            &args.network,
            if args.dry_run { "DRY RUN" } else { "LIVE TRADING" }
        ).await {
            warn!("Failed to send Discord startup alert: {}", e);
        }

        // Send dashboard status alert
        if web_server.is_some() {
            if let Err(e) = discord_alert.send_error_alert(
                "Web dashboard started successfully",
                "Dashboard is now available for monitoring and control"
            ).await {
                warn!("Failed to send dashboard status alert: {}", e);
            }
        }
    }
    
    // Get WebSocket sender, database, and runtime config references before moving web_server
    let (websocket_tx, web_db, runtime_cfg) = if let Some(ref web_server) = web_server {
        (
            Some(web_server.get_websocket_sender()),
            Some(web_server.get_database()),
            Some(web_server.get_runtime_config()),
        )
    } else {
        (None, None, None)
    };

    // Attach runtime_config to SafetyGuard for live updates of daily limits
    if let Some(ref arc_cfg) = runtime_cfg {
        safety = safety.with_runtime_config(arc_cfg.clone());
    }

    // Start web server in background
    let web_handle = if let Some(web_server) = web_server {
        Some(tokio::spawn(async move {
            if let Err(e) = web_server.start().await {
                error!("Web server error: {}", e);
            }
        }))
    } else {
        None
    };

    // Start monitoring in background
    let monitor_handle = tokio::spawn(async move {
        if let Err(e) = monitor.start_monitoring().await {
            error!("Monitor error: {}", e);
        }
    });
    
    // Start price update handler
    let discord_clone = discord.clone();
    let websocket_tx_clone = websocket_tx.clone();
    let price_handler = tokio::spawn(async move {
        let mut last_discord_update = std::time::Instant::now();
        let mut raydium_price = Decimal::ZERO;
        let mut orca_price = Decimal::ZERO;

        while let Some(update) = price_rx.recv().await {
            info!("📊 Price update from {}: ${:.4}", update.dex, update.price);

            // Update prices
            if update.dex == "Raydium" {
                raydium_price = update.price;
            } else if update.dex == "Orca" {
                orca_price = update.price;
            }

            // Send WebSocket price update to dashboard
            if raydium_price > Decimal::ZERO && orca_price > Decimal::ZERO {
                if let Some(ref ws_tx) = websocket_tx_clone {
                    let spread_percent = ((orca_price - raydium_price).abs() / raydium_price) * Decimal::from(100);
                    let price_update = web::PriceUpdate {
                        timestamp: chrono::Utc::now(),
                        raydium_price,
                        orca_price,
                        spread_percent,
                    };
                    let ws_msg = web::WebSocketMessage::PriceUpdate(price_update);
                    if let Err(e) = ws_tx.send(ws_msg) {
                        warn!("Failed to send WebSocket price update: {}", e);
                    }
                }
            }

            // Send Discord price update every 30 seconds (to avoid spam)
            if last_discord_update.elapsed().as_secs() > 30 && raydium_price > Decimal::ZERO && orca_price > Decimal::ZERO {
                if let Some(ref discord_alert) = discord_clone {
                    if let Err(e) = discord_alert.send_price_update(decimal_to_f64(raydium_price), decimal_to_f64(orca_price)).await {
                        warn!("Failed to send Discord price update: {}", e);
                    }
                }
                last_discord_update = std::time::Instant::now();
            }
        }
    });
    
    // Main arbitrage loop
    info!("💰 Starting arbitrage loop...");
    info!("Looking for opportunities > ${:.2} profit", config.limits.min_profit_usd);
    
    let mut last_opportunity_time = std::time::Instant::now();
    let mut opportunities_found = 0;
    let mut trades_executed = 0;
    
    loop {
        // Check if we should continue trading
        if !safety.should_continue_trading(&state).await {
            warn!("⛔ Safety limits reached, pausing for 1 hour");
            sleep(Duration::from_secs(3600)).await;
            safety.reset_daily_limits(&state).await;
            continue;
        }
        
        // Get current prices
        let raydium = *state.raydium_price.lock().await;
        let orca = *state.orca_price.lock().await;
        
        if let (Some(price_r), Some(price_o)) = (raydium, orca) {
            // Read runtime config values (with CLI override for max_position if provided)
            let (min_profit_usd_runtime, max_position_runtime) = if let Some(ref arc_cfg) = runtime_cfg {
                let cfg = arc_cfg.read().await.clone();
                (decimal_to_f64(cfg.min_profit_usd), decimal_to_f64(cfg.max_position_sol))
            } else {
                (decimal_to_f64(config.limits.min_profit_usd), decimal_to_f64(config.limits.max_position_sol))
            };
            let max_position_effective = cli_max_position.unwrap_or(max_position_runtime);

            // Calculate arbitrage opportunity
            if let Some(opportunity) = calculator.calculate_opportunity(
                decimal_to_f64(price_r),
                decimal_to_f64(price_o),
                max_position_effective,
            ) {
                opportunities_found += 1;
                last_opportunity_time = std::time::Instant::now();
                
                info!(
                    "🎯 Opportunity #{}: {} @ ${:.4} -> {} @ ${:.4} | Profit: ${:.2} ({:.2}%) | Confidence: {:.0}%",
                    opportunities_found,
                    opportunity.buy_dex,
                    opportunity.buy_price,
                    opportunity.sell_dex,
                    opportunity.sell_price,
                    opportunity.profit_after_fees_usd,
                    opportunity.profit_percentage,
                    opportunity.confidence_score * 100.0
                );

                // Send Discord opportunity alert
                if let Some(ref discord_alert) = discord {
                    if let Err(e) = discord_alert.send_opportunity_alert(
                        decimal_to_f64(price_r),
                        decimal_to_f64(price_o),
                        opportunity.profit_percentage,
                        (opportunity.confidence_score * 100.0) as u8
                    ).await {
                        warn!("Failed to send Discord opportunity alert: {}", e);
                    }
                }

                // Execute if profitable enough and confidence is high (min_profit_usd from runtime config)
                if opportunity.profit_after_fees_usd >= min_profit_usd_runtime
                    && opportunity.confidence_score > 0.5 {

                    // Pre-trade safety check
                    if !safety.pre_trade_check(&opportunity, config.wallet.use_ledger.unwrap_or(false)).await? {
                        warn!("⚠️ Safety check failed, skipping trade");
                        continue;
                    }
                    
                    match executor.execute_arbitrage(&opportunity).await {
                        Ok(signature) => {
                            trades_executed += 1;
                            info!("✅ Trade #{} sent! Signature: {}", trades_executed, signature);

                            // --- NEW: On-chain verification ---
                            match executor.verify_transaction(&signature).await {
                                Ok(actual_profit) => {
                                    // Send Discord profit alert with actual profit
                                    if let Some(ref discord_alert) = discord {
                                        if let Err(e) = discord_alert.send_profit_alert(
                                            actual_profit,
                                            &signature.to_string(),
                                            &opportunity.buy_dex,
                                            &opportunity.sell_dex,
                                            opportunity.amount_sol
                                        ).await {
                                            warn!("Failed to send Discord profit alert: {}", e);
                                        }
                                    }

                                    // Update statistics with actual profit
                                    let mut trades = state.trades_today.lock().await;
                                    *trades += 1;

                                    let mut profit = state.profit_today.lock().await;
                                    *profit += f64_to_decimal(actual_profit);

                                    // Broadcast to dashboard and store in DB
                                    if let Some(ref ws_tx) = websocket_tx {
                                        let price_update = web::WebSocketMessage::Transaction(web::TransactionRecord {
                                            id: None,
                                            timestamp: chrono::Utc::now(),
                                            signature: signature.to_string(),
                                            buy_dex: opportunity.buy_dex.clone(),
                                            sell_dex: opportunity.sell_dex.clone(),
                                            amount_sol: f64_to_decimal(opportunity.amount_sol),
                                            profit_usd: f64_to_decimal(actual_profit),
                                            raydium_price: raydium.unwrap_or(Decimal::ZERO),
                                            orca_price: orca.unwrap_or(Decimal::ZERO),
                                            spread_percent: Decimal::ZERO,
                                            gas_fee: Decimal::ZERO,
                                        });
                                        let _ = ws_tx.send(price_update);
                                    }

                                    if let Some(ref db) = web_db {
                                        let record = web::TransactionRecord {
                                            id: None,
                                            timestamp: chrono::Utc::now(),
                                            signature: signature.to_string(),
                                            buy_dex: opportunity.buy_dex.clone(),
                                            sell_dex: opportunity.sell_dex.clone(),
                                            amount_sol: f64_to_decimal(opportunity.amount_sol),
                                            profit_usd: f64_to_decimal(actual_profit),
                                            raydium_price: raydium.unwrap_or(Decimal::ZERO),
                                            orca_price: orca.unwrap_or(Decimal::ZERO),
                                            spread_percent: Decimal::ZERO,
                                            gas_fee: Decimal::ZERO,
                                        };
                                        if let Err(e) = db.insert_transaction(&record).await {
                                            warn!("Failed to store transaction: {}", e);
                                        }
                                    }

                                    // Record in safety system
                                    safety.record_trade(
                                        actual_profit,
                                        opportunity.amount_sol,
                                        actual_profit > 0.0
                                    ).await;

                                    info!("📊 Daily stats: {} trades, ${:.2} profit", *trades, *profit);
                                }
                                Err(e) => {
                                    error!("❌ Transaction verification failed: {}. Assuming gas fee loss.", e);
                                    // Record failure with estimated gas loss
                                    safety.record_trade(
                                        -opportunity.estimated_gas_sol * decimal_to_f64(price_r), // Lost gas cost
                                        opportunity.amount_sol,
                                        false
                                    ).await;
                                }
                            }
                        }
                        Err(e) => {
                            error!("❌ Trade execution failed: {}", e);

                            // Send Discord error alert
                            if let Some(ref discord_alert) = discord {
                                if let Err(discord_err) = discord_alert.send_error_alert(
                                    &e.to_string(),
                                    "Trade Execution"
                                ).await {
                                    warn!("Failed to send Discord error alert: {}", discord_err);
                                }
                            }

                            // Record failure
                            safety.record_trade(
                                -opportunity.estimated_gas_sol * decimal_to_f64(price_r), // Lost gas cost
                                opportunity.amount_sol,
                                false
                            ).await;
                        }
                    }
                }
            }
        } else {
            // Waiting for price data
            if raydium.is_none() && orca.is_none() {
                warn!("⏳ Waiting for price data from both DEXs...");
            } else if raydium.is_none() {
                warn!("⏳ Waiting for Raydium price data...");
            } else {
                warn!("⏳ Waiting for Orca price data...");
            }
            
            // Longer wait when no data
            sleep(Duration::from_secs(1)).await;
            continue;
        }
        
        // Status update every minute if no opportunities
        if last_opportunity_time.elapsed() > Duration::from_secs(60) {
            info!("👀 Monitoring... Last opportunity: {}s ago | Found: {} | Executed: {}",
                last_opportunity_time.elapsed().as_secs(),
                opportunities_found,
                trades_executed
            );
            last_opportunity_time = std::time::Instant::now();
        }
        
        // Small delay to prevent CPU spinning
        sleep(Duration::from_millis(100)).await;
    }
}

async fn test_api_connections() -> Result<()> {
    info!("Testing Solana RPC...");
    let client = solana_client::rpc_client::RpcClient::new("https://api.mainnet-beta.solana.com");
    match client.get_version() {
        Ok(version) => info!("✅ Solana RPC OK: {}", version.solana_core),
        Err(e) => warn!("⚠️ Solana RPC error: {}", e),
    }
    
    info!("Testing Jupiter API...");
    match monitor::DexMonitor::get_jupiter_quote(
        "So11111111111111111111111111111111111111112",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        1_000_000_000,
        50
    ).await {
        Ok(quote) => {
            info!("✅ Jupiter API OK: 1 SOL = {} USDC", 
                quote.out_amount as f64 / 1_000_000.0);
        }
        Err(e) => warn!("⚠️ Jupiter API error: {}", e),
    }
    
    info!("API tests completed!");
    Ok(())
}

fn load_config(path: &str) -> Result<Config> {
    let settings = config::Config::builder()
        .add_source(config::File::with_name(path))
        // Allow environment overrides, e.g. BOT__WEB__AUTH_TOKEN, BOT__RPC__URL
        .add_source(config::Environment::with_prefix("BOT").separator("__").try_parsing(true))
        .build()?;

    Ok(settings.try_deserialize()?)
}
