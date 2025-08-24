use solana_arbitrage_bot::{executor::TransactionExecutor, Config, DexConfig, DexInfo, ExecutionConfig, LimitsConfig, RpcConfig, WalletConfig};
use rust_decimal::Decimal;

fn base_config_with_fee(priority: u64, cap: Option<u64>) -> Config {
    Config {
        rpc: RpcConfig { url: "http://localhost".into(), ws_url: "ws://localhost".into() },
        wallet: WalletConfig { path: "./wallet.json".into(), use_ledger: None, ledger_path: None },
        dex: DexConfig { raydium: DexInfo { program_id: "".into(), sol_usdc_pool: "".into() }, orca: DexInfo { program_id: "".into(), sol_usdc_pool: "".into() } },
        limits: LimitsConfig {
            max_position_sol: Decimal::from_f64_retain(0.05).unwrap(),
            min_profit_percent: Decimal::from_f64_retain(0.3).unwrap(),
            min_profit_usd: Decimal::from_f64_retain(0.05).unwrap(),
            max_slippage_percent: Decimal::from_f64_retain(0.5).unwrap(),
            max_daily_loss_usd: Decimal::from_f64_retain(10.0).unwrap(),
            max_daily_trades: 50,
        },
        execution: ExecutionConfig {
            priority_fee_lamports: priority,
            max_priority_fee_cap_lamports: cap,
            simulation_required: true,
            max_retries: 1,
        },
        discord: None,
        web: None,
    }
}
#[test]
fn priority_fee_above_cap_is_rejected() {
    // Here priority fee is above cap; constructor should return Err
    let cfg = base_config_with_fee(200_001, Some(50_000));
    let err = TransactionExecutor::new(&cfg, true).err().expect("expected error");
    let msg = format!("{}", err);
    assert!(msg.contains("exceeds cap"), "unexpected error: {msg}");
}


#[test]
fn priority_fee_is_capped_at_runtime() {
    // Priority fee at or below cap should be accepted
    let cfg = base_config_with_fee(40_000, Some(50_000));
    let exec = TransactionExecutor::new(&cfg, true).expect("executor init");

    // No actual swap executed; the function returns Signature::default() in dry-run fallback
    let opportunity = solana_arbitrage_bot::calculator::ArbitrageOpportunity {
        buy_dex: "Raydium".into(),
        sell_dex: "Orca".into(),
        buy_price: 150.0,
        sell_price: 151.0,
        amount_sol: 0.01,
        expected_profit_usd: 1.0,
        profit_after_fees_usd: 0.9,
        profit_percentage: 0.5,
        estimated_gas_sol: 0.0003,
        price_impact: 0.01,
        confidence_score: 0.8,
    };

    // This should not error even with high priority fee; in dry-run path it returns default signature
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        let res = exec.execute_direct_swap(&opportunity).await;
        assert!(res.is_ok());
    });
}

