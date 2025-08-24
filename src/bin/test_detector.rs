//! 🎯 Token Detector Test - Quick Test of New Token Detection
//!
//! Simple test to verify our token detector is working

use log::{error, info};
use std::time::Duration;
use tokio::time::timeout;

use solana_arbitrage_bot::sniper::{
    detector::{TokenDetector, TokenDetectorConfig},
    types::*,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("🎯 Starting Token Detector Test...");

    // Create detector config
    let config = TokenDetectorConfig {
        max_queue_size: 50,
        processed_token_ttl: Duration::from_secs(1800), // 30 minutes
        min_liquidity_sol: 1.0,
        max_token_age_minutes: 15.0,
        pump_fun_poll_interval: Duration::from_secs(10),
        cleanup_interval: Duration::from_secs(60),
        ..Default::default()
    };

    // Create detector
    let mut detector = TokenDetector::new(config)?;

    info!("✅ Token detector created successfully");

    // Start monitoring in background
    let detector_clone = detector.clone();
    let monitoring_handle = tokio::spawn(async move {
        if let Err(e) = detector_clone.start_monitoring().await {
            error!("Monitoring error: {}", e);
        }
    });

    info!("🔍 Token monitoring started...");

    // Test loop - check for new tokens
    let mut total_tokens_found = 0;
    let start_time = std::time::Instant::now();

    for i in 1..=20 {
        // Run for 20 iterations (about 2 minutes)
        info!("📊 Check #{} - Looking for new tokens...", i);

        match timeout(Duration::from_secs(5), detector.get_new_tokens()).await {
            Ok(Ok(Some(tokens))) => {
                total_tokens_found += tokens.len();

                for token in tokens {
                    info!(
                        "🆕 Found token: {} ({}) - Age: {:.1}m, Liquidity: ${:.0}, DEX: {}",
                        token.symbol,
                        &token.mint[..8], // Show first 8 chars of mint
                        token.age_minutes,
                        token.liquidity_usd,
                        token.dex
                    );

                    // Show token details
                    info!(
                        "   📈 Market Cap: ${:.0}, Price: ${:.6}",
                        token.market_cap_usd, token.price
                    );
                    info!(
                        "   🔒 Mint Auth: {}, Freeze Auth: {}",
                        if token.mint_authority_disabled {
                            "✅ Disabled"
                        } else {
                            "❌ Enabled"
                        },
                        if token.freeze_authority_disabled {
                            "✅ Disabled"
                        } else {
                            "❌ Enabled"
                        }
                    );
                    info!("   👥 Top 10 holders: {:.1}%", token.top_10_holders_percent);

                    // Calculate a simple score
                    let score = calculate_simple_score(&token);
                    info!("   ⭐ Score: {:.1}/100", score);

                    if score >= 60.0 {
                        info!("   🎯 POTENTIAL SNIPE TARGET!");
                    }

                    println!(); // Empty line for readability
                }
            }
            Ok(Ok(None)) => {
                info!("   No new tokens found");
            }
            Ok(Err(e)) => {
                error!("   Error getting tokens: {}", e);
            }
            Err(_) => {
                warn!("   Timeout waiting for tokens");
            }
        }

        // Show statistics
        let elapsed = start_time.elapsed();
        info!(
            "📊 Stats: {} tokens found in {:.1}s",
            total_tokens_found,
            elapsed.as_secs_f64()
        );

        // Wait before next check
        tokio::time::sleep(Duration::from_secs(6)).await;
    }

    // Stop detector
    detector.stop().await;

    // Wait a bit for cleanup
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Cancel monitoring task
    monitoring_handle.abort();

    info!("🎉 Test completed!");
    info!("📊 Final stats: {} total tokens found", total_tokens_found);

    if total_tokens_found > 0 {
        info!("✅ Token detector is working!");
    } else {
        warn!("⚠️ No tokens found - this might be normal if markets are quiet");
        info!("💡 Try running during active trading hours for better results");
    }

    Ok(())
}

/// Calculate a simple score for a token (0-100)
fn calculate_simple_score(token: &NewToken) -> f64 {
    let mut score = 0.0;

    // Age bonus (newer = better for sniping)
    if token.age_minutes < 5.0 {
        score += 30.0;
    } else if token.age_minutes < 15.0 {
        score += 20.0;
    } else if token.age_minutes < 60.0 {
        score += 10.0;
    }

    // Liquidity health
    let liq_ratio = token.liquidity_usd / token.market_cap_usd.max(1.0);
    if liq_ratio > 0.2 {
        score += 25.0;
    } else if liq_ratio > 0.1 {
        score += 15.0;
    } else if liq_ratio > 0.05 {
        score += 10.0;
    }

    // Volume momentum
    if token.volume_5m > token.liquidity_usd * 0.1 {
        score += 20.0;
    } else if token.volume_5m > token.liquidity_usd * 0.05 {
        score += 10.0;
    }

    // Holder distribution
    if token.top_10_holders_percent < 40.0 {
        score += 15.0;
    } else if token.top_10_holders_percent < 60.0 {
        score += 10.0;
    } else if token.top_10_holders_percent < 80.0 {
        score += 5.0;
    }

    // Safety bonuses
    if token.mint_authority_disabled {
        score += 5.0;
    }
    if token.freeze_authority_disabled {
        score += 5.0;
    }

    // Safety penalties
    if !token.mint_authority_disabled {
        score -= 20.0;
    }
    if !token.freeze_authority_disabled {
        score -= 10.0;
    }

    // Market cap check
    if token.market_cap_usd > 1_000_000.0 {
        score -= 20.0; // Too big for sniping
    } else if token.market_cap_usd < 10_000.0 {
        score += 10.0; // Good size for sniping
    }

    score.max(0.0).min(100.0)
}
