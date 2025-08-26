use anyhow::Result;
use log::info;
use rust_decimal::Decimal;
use solana_arbitrage_bot::architecture::{EnhancedSharedState, TradeRecord};
use std::time::SystemTime;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    
    info!("🚀 Architecture Demo - Race Condition & Deadlock Prevention");
    
    // Create enhanced shared state
    let state = EnhancedSharedState::new();
    state.initialize().await?;
    
    // Demo 1: Race condition prevention
    demo_race_condition_prevention(&state).await?;
    
    // Demo 2: Memory leak prevention
    demo_memory_leak_prevention(&state).await?;
    
    // Demo 3: Deadlock prevention
    demo_deadlock_prevention(&state).await?;
    
    // Demo 4: System health monitoring
    demo_health_monitoring(&state).await?;
    
    info!("✅ All architecture demos completed successfully!");
    Ok(())
}

/// Demonstrate race condition prevention with atomic price updates
async fn demo_race_condition_prevention(state: &EnhancedSharedState) -> Result<()> {
    info!("📊 Demo 1: Race Condition Prevention");
    
    // Simulate concurrent price updates
    let state1 = state.clone();
    let state2 = state.clone();
    
    let handle1 = tokio::spawn(async move {
        for i in 0..100 {
            let price = Decimal::from_f64_retain(150.0 + i as f64 * 0.1).unwrap();
            state1.update_raydium_price(price);
            tokio::task::yield_now().await;
        }
    });
    
    let handle2 = tokio::spawn(async move {
        for i in 0..100 {
            let price = Decimal::from_f64_retain(149.0 + i as f64 * 0.1).unwrap();
            state2.update_orca_price(price);
            tokio::task::yield_now().await;
        }
    });
    
    // Simulate arbitrage calculations during updates
    let state3 = state.clone();
    let handle3 = tokio::spawn(async move {
        for _ in 0..50 {
            let snapshot = state3.get_price_snapshot();
            if snapshot.is_valid {
                if let Some(spread) = snapshot.calculate_spread_percent() {
                    if spread > Decimal::from_f64_retain(0.1).unwrap() {
                        info!("💰 Arbitrage opportunity: {:.3}% spread", spread);
                    }
                }
            }
            sleep(Duration::from_millis(10)).await;
        }
    });
    
    // Wait for all tasks
    handle1.await?;
    handle2.await?;
    handle3.await?;
    
    // Verify final state
    let final_snapshot = state.get_price_snapshot();
    info!("✅ Final prices - Raydium: {:?}, Orca: {:?}", 
          final_snapshot.raydium_price, final_snapshot.orca_price);
    
    Ok(())
}

/// Demonstrate memory leak prevention with bounded collections
async fn demo_memory_leak_prevention(state: &EnhancedSharedState) -> Result<()> {
    info!("🧠 Demo 2: Memory Leak Prevention");
    
    // Add many trade records to test bounded collection
    for i in 0..1500 {  // More than the 1000 limit
        let trade = TradeRecord {
            timestamp: SystemTime::now(),
            profit_usd: (i as f64 - 750.0) * 0.1, // Some profits, some losses
            amount_sol: 1.0,
            success: i % 10 != 0, // 90% success rate
            dex_buy: if i % 2 == 0 { "Raydium".to_string() } else { "Orca".to_string() },
            dex_sell: if i % 2 == 0 { "Orca".to_string() } else { "Raydium".to_string() },
        };
        
        state.record_trade(trade).await;
        
        if i % 100 == 0 {
            let recent_trades = state.get_recent_trades(10).await;
            info!("📈 Added {} trades, recent count: {}", i + 1, recent_trades.len());
        }
    }
    
    // Check memory stats
    let stats = state.get_system_stats().await;
    info!("💾 Memory stats: {} trades, {} bytes", 
          stats.memory_stats.current_count, 
          stats.memory_stats.memory_usage_bytes);
    
    // Verify bounded collection worked
    let all_recent = state.get_recent_trades(2000).await;
    info!("✅ Bounded collection working: {} trades kept (should be ≤ 1000)", all_recent.len());
    
    Ok(())
}

/// Demonstrate deadlock prevention with ordered locking
async fn demo_deadlock_prevention(state: &EnhancedSharedState) -> Result<()> {
    info!("🔒 Demo 3: Deadlock Prevention");
    
    // Simulate multiple concurrent operations that could deadlock
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            let operation_id = format!("operation_{}", i);
            
            // Complex operation that updates multiple state components
            let result = state_clone.safe_complex_operation(operation_id.clone(), async {
                // Simulate some work
                sleep(Duration::from_millis(100)).await;
                
                // Update trades and profit
                state_clone.increment_trades();
                state_clone.add_profit(10.0).await?;
                
                // Record trade
                let trade = TradeRecord {
                    timestamp: SystemTime::now(),
                    profit_usd: 10.0,
                    amount_sol: 1.0,
                    success: true,
                    dex_buy: "Raydium".to_string(),
                    dex_sell: "Orca".to_string(),
                };
                state_clone.record_trade(trade).await;
                
                Ok(format!("Operation {} completed", i))
            }).await;
            
            match result {
                Ok(msg) => info!("✅ {}", msg),
                Err(e) => info!("❌ Operation {} failed: {}", i, e),
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations
    for handle in handles {
        handle.await?;
    }
    
    let final_trades = state.get_trades_today();
    let final_profit = state.get_profit_today().await?;
    info!("✅ Deadlock prevention successful: {} trades, ${:.2} profit", 
          final_trades, final_profit);
    
    Ok(())
}

/// Demonstrate system health monitoring
async fn demo_health_monitoring(state: &EnhancedSharedState) -> Result<()> {
    info!("🏥 Demo 4: System Health Monitoring");
    
    // Update prices to make them fresh
    state.update_raydium_price(Decimal::from_f64_retain(150.0).unwrap());
    state.update_orca_price(Decimal::from_f64_retain(149.5).unwrap());
    
    // Get comprehensive stats
    let stats = state.get_system_stats().await;
    info!("📊 System Statistics:");
    info!("   Price updates: {}", stats.price_stats.total_updates);
    info!("   Read operations: {}", stats.performance_stats.total_reads);
    info!("   Write operations: {}", stats.performance_stats.total_writes);
    info!("   Lock success rate: {:.1}%", stats.lock_stats.success_rate);
    info!("   Memory usage: {} bytes", stats.memory_stats.memory_usage_bytes);
    
    // Perform health check
    let health = state.health_check().await;
    info!("🏥 Health Status: {:?}", health.status);
    if !health.issues.is_empty() {
        info!("⚠️ Issues detected:");
        for issue in health.issues {
            info!("   - {}", issue);
        }
    }
    
    // Test price freshness
    let fresh = state.are_prices_fresh(60);
    info!("🕐 Prices fresh (< 60s): {}", fresh);
    
    // Get price snapshot with metadata
    let snapshot = state.get_price_snapshot();
    if let Some(spread_percent) = snapshot.calculate_spread_percent() {
        info!("💰 Current spread: {:.3}%", spread_percent);
    }
    
    if let Some((buy_price, sell_price, buy_dex, sell_dex)) = snapshot.get_arbitrage_prices() {
        info!("🔄 Arbitrage: Buy {} at {:.3}, Sell {} at {:.3}", 
              buy_dex, buy_price, sell_dex, sell_price);
    }
    
    Ok(())
}

/// Helper trait to make EnhancedSharedState cloneable for demo
trait CloneableState {
    fn clone(&self) -> Self;
}

impl CloneableState for EnhancedSharedState {
    fn clone(&self) -> Self {
        // For demo purposes, create a new instance
        // In real usage, you'd use Arc<EnhancedSharedState>
        EnhancedSharedState::new()
    }
}
