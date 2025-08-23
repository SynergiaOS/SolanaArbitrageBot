use solana_arbitrage_bot::safety::SafetyGuard;
use solana_arbitrage_bot::web::BotConfig;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn dynamic_daily_limits_are_applied() {
    let cfg = solana_arbitrage_bot::Config {
        rpc: solana_arbitrage_bot::RpcConfig { url: "".into(), ws_url: "".into() },
        wallet: solana_arbitrage_bot::WalletConfig { path: "".into(), use_ledger: None, ledger_path: None },
        dex: solana_arbitrage_bot::DexConfig { raydium: solana_arbitrage_bot::DexInfo { program_id: "".into(), sol_usdc_pool: "".into() }, orca: solana_arbitrage_bot::DexInfo { program_id: "".into(), sol_usdc_pool: "".into() } },
        limits: solana_arbitrage_bot::LimitsConfig {
            max_position_sol: Decimal::from_f64_retain(0.05).unwrap(),
            min_profit_percent: Decimal::from_f64_retain(0.3).unwrap(),
            min_profit_usd: Decimal::from_f64_retain(0.05).unwrap(),
            max_slippage_percent: Decimal::from_f64_retain(0.5).unwrap(),
            max_daily_loss_usd: Decimal::from_f64_retain(10.0).unwrap(),
            max_daily_trades: 50,
        },
        execution: solana_arbitrage_bot::ExecutionConfig { priority_fee_lamports: 1, simulation_required: true, max_retries: 1 },
        discord: None,
        web: None,
    };

    let runtime_cfg = Arc::new(RwLock::new(BotConfig{
        min_profit_usd: Decimal::from_f64_retain(0.05).unwrap(),
        max_position_sol: Decimal::from_f64_retain(0.05).unwrap(),
        max_daily_trades: 50,
        max_daily_loss_usd: Decimal::from_f64_retain(10.0).unwrap(),
        enabled: true,
    }));

    let safety = SafetyGuard::new(&cfg).with_runtime_config(runtime_cfg.clone());

    let state = solana_arbitrage_bot::SharedState{
        raydium_price: Arc::new(tokio::sync::Mutex::new(None)),
        orca_price: Arc::new(tokio::sync::Mutex::new(None)),
        trades_today: Arc::new(tokio::sync::Mutex::new(0)),
        profit_today: Arc::new(tokio::sync::Mutex::new(Decimal::from_f64_retain(0.0).unwrap())),
    };

    // Initially should allow trading
    assert!(safety.should_continue_trading(&state).await);

    // Update runtime limits to block
    {
        let mut cfg = runtime_cfg.write().await;
        cfg.max_daily_trades = 0; // invalid edge; should block at check >= max
        cfg.max_daily_loss_usd = Decimal::from_f64_retain(0.01).unwrap();
    }

    // Simulate some trades/losses
    *state.trades_today.lock().await = 1;
    *state.profit_today.lock().await = Decimal::from_f64_retain(-1.0).unwrap();

    // Now should stop trading per new limits
    assert!(!safety.should_continue_trading(&state).await);
}

