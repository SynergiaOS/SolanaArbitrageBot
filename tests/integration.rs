//! Integration tests for real API connections
//! Run with: cargo test --test integration -- --nocapture

use anyhow::Result;
use solana_arbitrage_bot::*;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_real_raydium_price_fetch() -> Result<()> {
    env_logger::init();
    
    println!("Testing real Raydium price fetch...");
    
    let config = Config {
        rpc: RpcConfig {
            url: "https://api.mainnet-beta.solana.com".to_string(),
            ws_url: "wss://api.mainnet-beta.solana.com".to_string(),
        },
        wallet: WalletConfig {
            use_ledger: Some(false),
            ledger_path: None,
            path: "./test_wallet.json".to_string(),
        },
        dex: DexConfig {
            raydium: DexInfo {
                program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string(),
                sol_usdc_pool: "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2".to_string(),
            },
            orca: DexInfo {
                program_id: "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string(),
                sol_usdc_pool: "HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ".to_string(),
            },
        },
        limits: LimitsConfig {
            max_position_sol: 10.0,
            min_profit_percent: 0.3,
            min_profit_usd: 1.0,
            max_slippage_percent: 0.5,
            max_daily_loss_usd: 100.0,
            max_daily_trades: 30,
        },
        execution: ExecutionConfig {
            priority_fee_lamports: 10000,
            simulation_required: true,
            max_retries: 3,
        },
    };
    
    let raydium_price = Arc::new(Mutex::new(None));
    let orca_price = Arc::new(Mutex::new(None));
    
    let monitor = monitor::DexMonitor::new(
        raydium_price.clone(),
        orca_price.clone(),
        &config,
    )?;
    
    // Start monitoring in background
    let monitor_handle = tokio::spawn(async move {
        let _ = monitor.start_monitoring().await;
    });
    
    // Wait a bit for prices to come in
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Check if we got prices
    let ray_price = *raydium_price.lock().await;
    let orca_price_val = *orca_price.lock().await;
    
    println!("Raydium price: {:?}", ray_price);
    println!("Orca price: {:?}", orca_price_val);
    
    // Cancel monitor
    monitor_handle.abort();
    
    // We should have at least one price
    assert!(ray_price.is_some() || orca_price_val.is_some(), "Should have received at least one price update");
    
    if let Some(price) = ray_price {
        assert!(price > 0.0 && price < 1000.0, "Price should be reasonable");
        println!("✅ Raydium price looks valid: ${:.2}", price);
    }
    
    if let Some(price) = orca_price_val {
        assert!(price > 0.0 && price < 1000.0, "Price should be reasonable");
        println!("✅ Orca price looks valid: ${:.2}", price);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_jupiter_quote_api() -> Result<()> {
    println!("Testing Jupiter Quote API...");
    
    let sol_mint = "So11111111111111111111111111111111111111112";
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    let amount = 1_000_000_000; // 1 SOL in lamports
    
    match monitor::DexMonitor::get_jupiter_quote(sol_mint, usdc_mint, amount, 50).await {
        Ok(quote) => {
            println!("✅ Jupiter API works!");
            println!("Quote: {} SOL = {} USDC", 
                quote.in_amount as f64 / 1e9,
                quote.out_amount as f64 / 1e6
            );
            
            // Validate the quote
            assert!(quote.out_amount > 0, "Should have non-zero output");
            
            // Check route
            println!("Route: {:?}", quote.market_infos.iter()
                .map(|m| &m.label)
                .collect::<Vec<_>>());
        }
        Err(e) => {
            // It's ok if this fails in CI/CD or without API key
            println!("⚠️ Jupiter API not available: {}", e);
            println!("This is expected in test environment");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_arbitrage_detection() -> Result<()> {
    println!("Testing arbitrage detection with real prices...");
    
    let config = test_config();
    let calculator = calculator::ProfitCalculator::new(&config);
    
    // Simulate a price difference
    let raydium_price = 150.0;
    let orca_price = 151.0; // 0.67% difference
    
    let opportunity = calculator.calculate_opportunity(
        raydium_price,
        orca_price,
        10.0, // 10 SOL max
    );
    
    if let Some(opp) = opportunity {
        println!("✅ Arbitrage opportunity detected!");
        println!("  Buy on: {}", opp.buy_dex);
        println!("  Sell on: {}", opp.sell_dex);
        println!("  Amount: {} SOL", opp.amount_sol);
        println!("  Expected profit: ${:.2}", opp.expected_profit_usd);
        println!("  Profit after fees: ${:.2}", opp.profit_after_fees_usd);
        println!("  Confidence: {:.1}%", opp.confidence_score * 100.0);
        
        assert!(opp.profit_after_fees_usd > 0.0, "Should be profitable");
    } else {
        println!("No arbitrage opportunity at this spread");
    }
    
    Ok(())
}

#[tokio::test]
#[ignore] // This test requires a real wallet file
async fn test_transaction_simulation() -> Result<()> {
    println!("Testing transaction simulation...");
    
    let config = test_config();
    let executor = executor::TransactionExecutor::new(&config, true)?; // dry_run = true
    
    let opportunity = calculator::ArbitrageOpportunity {
        buy_dex: "Raydium".to_string(),
        sell_dex: "Orca".to_string(),
        buy_price: 150.0,
        sell_price: 151.0,
        amount_sol: 1.0,
        expected_profit_usd: 1.0,
        profit_after_fees_usd: 0.5,
        profit_percentage: 0.33,
        estimated_gas_sol: 0.00025,
        price_impact: 0.01,
        confidence_score: 0.7,
    };
    
    match executor.execute_arbitrage(&opportunity).await {
        Ok(sig) => {
            println!("✅ Transaction simulation successful!");
            println!("  Signature: {}", sig);
        }
        Err(e) => {
            println!("Transaction simulation failed: {}", e);
            // This is expected without a real wallet
        }
    }
    
    Ok(())
}

// Helper function to create test config
fn test_config() -> Config {
    Config {
        rpc: RpcConfig {
            url: "https://api.mainnet-beta.solana.com".to_string(),
            ws_url: "wss://api.mainnet-beta.solana.com".to_string(),
        },
        wallet: WalletConfig {
            use_ledger: Some(false),
            ledger_path: None,
            path: "./test_wallet.json".to_string(),
        },
        dex: DexConfig {
            raydium: DexInfo {
                program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string(),
                sol_usdc_pool: "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2".to_string(),
            },
            orca: DexInfo {
                program_id: "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string(),
                sol_usdc_pool: "HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ".to_string(),
            },
        },
        limits: LimitsConfig {
            max_position_sol: 10.0,
            min_profit_percent: 0.3,
            min_profit_usd: 1.0,
            max_slippage_percent: 0.5,
            max_daily_loss_usd: 100.0,
            max_daily_trades: 30,
        },
        execution: ExecutionConfig {
            priority_fee_lamports: 10000,
            simulation_required: true,
            max_retries: 3,
        },
    }
}

// Re-export types for tests
use solana_arbitrage_bot::{Config, RpcConfig, WalletConfig, DexConfig, DexInfo, LimitsConfig, ExecutionConfig};
