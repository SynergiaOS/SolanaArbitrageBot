//! Sniper Lite - minimalny dry-run skaner bez zależności od modułów sniper
//! Cel: szybka weryfikacja ścieżki uruchomienia i środowiska bez egzekucji transakcji.

use anyhow::Result;
use clap::Parser;
use log::{info, warn};
use std::path::Path;
use std::thread;
use std::time::Duration;

/// Parametry uruchomienia Sniper Lite
#[derive(Parser, Debug)]
#[command(author, version, about = "Sniper Lite - dry-run", long_about = None)]
struct Args {
    /// Ścieżka do pliku konfiguracyjnego (YAML)
    #[arg(long, default_value = "./config.yaml")]
    config: String,

    /// Sieć do logowania (devnet | mainnet | testnet)
    #[arg(long, default_value = "devnet")]
    network: String,

    /// Interwał skanowania w milisekundach
    #[arg(long, default_value_t = 500)]
    scan_interval_ms: u64,

    /// Liczba iteracji skanowania przed zakończeniem
    #[arg(long, default_value_t = 10)]
    iterations: usize,
}

fn main() -> Result<()> {
    // Logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    info!("🎯 Sniper Lite (dry-run)");
    info!("🌐 Network: {}", args.network);
    info!("⏱️  Scan interval: {} ms, iterations: {}", args.scan_interval_ms, args.iterations);

    // Opcjonalne wczytanie BotConfig (bez twardej zależności od pełnego snipera)
    let cfg_path = &args.config;
    if Path::new(cfg_path).exists() {
        match solana_arbitrage_bot::config_manager::BotConfig::from_file(cfg_path) {
            Ok(cfg) => {
                info!("📄 Config loaded: {}", cfg_path);
                info!(
                    "Limits preview -> max_position_sol: {}, min_profit_usd: {}, min_profit_percent: {}",
                    cfg.trading.max_position_sol, cfg.trading.min_profit_usd, cfg.trading.min_profit_percent
                );
            }
            Err(e) => {
                warn!("⚠️  Failed to parse config {}: {}. Using defaults.", cfg_path, e);
            }
        }
    } else {
        warn!("⚠️  Config file not found: {}. Using defaults.", cfg_path);
    }

    // Prosta pętla „skanowania”
    for i in 1..=args.iterations {
        info!("🔎 Scan #{}/{} ...", i, args.iterations);
        // tu można dodać lekkie sprawdzenia HTTP/WS w przyszłości (dry-run)
        thread::sleep(Duration::from_millis(args.scan_interval_ms));
    }

    info!("✅ Sniper Lite dry-run finished successfully.");
    Ok(())
}
