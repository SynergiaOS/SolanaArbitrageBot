use rust_decimal::Decimal;
use solana_arbitrage_bot::{calculator::ArbitrageOpportunity, executor::TransactionExecutor, Config, DexConfig, DexInfo, ExecutionConfig, LimitsConfig, RpcConfig, WalletConfig};

// NOTE: This test uses devnet endpoints and runs in DRY-RUN mode. It fetches a real Jupiter quote
// and performs a simulation without sending a live transaction.
// Marked as ignored by default to avoid network dependency in CI.

fn config_devnet() -> Config {
    Config {
        rpc: RpcConfig {
            url: "https://api.devnet.solana.com".into(),
            ws_url: "wss://api.devnet.solana.com".into(),
        },
        wallet: WalletConfig {
            path: "./wallet.json".into(),
            use_ledger: None,
            ledger_path: None,
        },
        dex: DexConfig {
            raydium: DexInfo { program_id: "".into(), sol_usdc_pool: "".into() },
            orca: DexInfo { program_id: "".into(), sol_usdc_pool: "".into() },
        },
        limits: LimitsConfig {
            max_position_sol: Decimal::from_f64_retain(0.01).unwrap(),
            min_profit_percent: Decimal::from_f64_retain(0.3).unwrap(),
            min_profit_usd: Decimal::from_f64_retain(0.05).unwrap(),
            max_slippage_percent: Decimal::from_f64_retain(0.5).unwrap(),
            max_daily_loss_usd: Decimal::from_f64_retain(10.0).unwrap(),
            max_daily_trades: 10,
        },
        execution: ExecutionConfig {
            priority_fee_lamports: 5_000,
            max_priority_fee_cap_lamports: Some(10_000),
            simulation_required: true,
            max_retries: 1,
        },
        discord: None,
        web: None,
    }
}

#[tokio::test]
#[ignore]
async fn devnet_jupiter_simulation_dry_run() {
    let cfg = config_devnet();
    let exec = TransactionExecutor::new(&cfg, true).expect("executor init");

    // Create a tiny arbitrage opportunity; we only care about the path hitting Jupiter quote + simulation
    let opp = ArbitrageOpportunity {
        buy_dex: "Raydium".into(),
        sell_dex: "Orca".into(),
        buy_price: 150.0,
        sell_price: 151.0,
        amount_sol: 0.001,
        expected_profit_usd: 0.10,
        profit_after_fees_usd: 0.08,
        profit_percentage: 0.5,
        estimated_gas_sol: 0.0003,
        price_impact: 0.01,
        confidence_score: 0.8,
    };

    // Call the internals via execute_direct_swap fallback for now (dry-run)
    // In a full integration we would use execute_arbitrage with a mocked Jupiter response
    let res = exec.execute_direct_swap(&opp).await;
    assert!(res.is_ok());
}

